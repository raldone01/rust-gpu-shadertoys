use alloc::{boxed::Box, format, string::String, vec::Vec};
use hashbrown::HashMap;

use crate::{
  buffered_reader::{BufferReader, BufferedReader, Read, ReadError},
  compressed_reader::CompressedReader,
  dynamic_error::DynamicError,
  tar_constants::ustar::*,
};

/// Represents a parsed TAR header containing file metadata.
#[derive(Debug)]
pub struct TarHeader {
  pub path: String,
  pub size: u64,
}

/// Parses a null-terminated string from a byte slice.
/// Maps parsing errors to `ReadError::Io`.
fn parse_name(bytes: &[u8]) -> Result<String, ReadError> {
  let end = bytes
    .iter()
    .position(|&b| b == b'\0')
    .unwrap_or(bytes.len());
  core::str::from_utf8(&bytes[..end])
    .map(String::from)
    .map_err(|err| ReadError::Io(Box::new(err)))
}

/// Parses a null-terminated, space-padded octal number from a byte slice.
/// Maps parsing errors to `ReadError::Io`.
fn parse_octal(bytes: &[u8]) -> Result<u64, ReadError> {
  let end = bytes
    .iter()
    .position(|&b| b == b'\0')
    .unwrap_or(bytes.len());
  let s = core::str::from_utf8(&bytes[..end]).map_err(|err| ReadError::Io(Box::new(err)))?;
  u64::from_str_radix(s.trim(), 8).map_err(|err| ReadError::Io(Box::new(err)))
}

/// Parses a 512-byte buffer as a USTAR header.
fn parse_header(header_buf: &[u8]) -> Result<TarHeader, ReadError> {
  // Verify the USTAR magic string: "ustar\0"
  if &header_buf[MAGIC_OFFSET..MAGIC_OFFSET + MAGIC_LEN] != MAGIC {
    // This indicates the archive is not in the expected USTAR format or is corrupted.
    return Err(ReadError::Io(Box::new(DynamicError(String::from(
      "Invalid USTAR magic string",
    )))));
  }

  // NOTE: A more robust implementation would also verify the header checksum here.

  let path = parse_name(&header_buf[NAME_OFFSET..NAME_OFFSET + NAME_LEN])?;
  let size = parse_octal(&header_buf[SIZE_OFFSET..SIZE_OFFSET + SIZE_LEN])?;

  Ok(TarHeader { path, size })
}

/// A reader for decompressing and parsing a TAR archive on the fly.
///
/// It reads from a compressed data source, decompresses the data, and parses
/// the USTAR (TAR) format to extract files.
struct TarReader<R: Read> {
  // The BufferedReader is crucial for performance. It minimizes calls to the
  // underlying decompressor by reading larger chunks into its internal buffer.
  reader: BufferedReader<R>,
  /// A security limit on the total number of bytes that can be extracted
  /// from the archive to prevent decompression bomb attacks.
  max_extracted_bytes: usize,
  /// The number of bytes extracted so far.
  total_extracted: usize,
}

impl<R: Read> TarReader<R> {
  /// Creates a new `TarReader`.
  ///
  /// # Arguments
  /// * `reader` - A reader that provides compressed TAR data.
  /// * `max_extracted_bytes` - A security limit on the total number of bytes
  ///   that can be extracted from the archive.
  fn new(reader: R, max_extracted_bytes: usize) -> Self {
    let buffer_size = max_extracted_bytes / 8;
    let buffered_reader = BufferedReader::new(buffer_size, reader);

    Self {
      reader: buffered_reader,
      max_extracted_bytes,
      total_extracted: 0,
    }
  }

  /// Reads all files from the archive into a HashMap.
  ///
  /// This method iterates through the entire TAR archive, decompresses it on the fly,
  /// and collects all files into memory. It assumes the `BufferedReader` has a
  /// `read_exact` method to simplify reading fixed-size chunks.
  ///
  /// # Errors
  /// Returns `ReadError` if:
  /// - The archive is malformed or corrupted.
  /// - An I/O error occurs during decompression.
  /// - The total size of extracted files would exceed `max_extracted_bytes`.
  fn read_all_files(&mut self) -> Result<HashMap<String, Vec<u8>>, ReadError> {
    let mut files = HashMap::new();

    loop {
      // Attempt to read a full header block.
      let header_buf = match self.reader.read_exact(BLOCK_SIZE) {
        Ok(header_buf) => header_buf,
        // A clean end of the underlying stream is a valid way to end the archive.
        Err(ReadError::UnexpectedEof) => break,
        // Any other read error is fatal.
        Err(e) => return Err(e),
      };

      // Check for the end-of-archive marker, which is a block of all zeros.
      // A standard TAR archive ends with two such blocks, but encountering one
      // is a reliable signal to stop reading.
      if header_buf.iter().all(|&b| b == 0) {
        break;
      }

      // If it's not a zero block, parse it as a file header.
      let header = parse_header(&header_buf)?;
      let file_size = header.size as usize;

      // Security check: ensure we don't exceed the extraction limit.
      if self.total_extracted.saturating_add(file_size) > self.max_extracted_bytes {
        return Err(ReadError::BufferTooSmall);
      }

      // Read the file's data content.
      let file_data = self.reader.read_exact(file_size)?;
      self.total_extracted += file_size;

      // Store the extracted file.
      files.insert(header.path, file_data.to_vec());

      // File data in a TAR archive is padded with null bytes to fill a 512-byte block.
      // We must consume these padding bytes to align the stream for the next header.
      let padding_size = (BLOCK_SIZE - (file_size % BLOCK_SIZE)) % BLOCK_SIZE;
      if padding_size > 0 {
        // Skip padding by reading it into a temporary, throwaway buffer.
        let _padding_buf = self.reader.read_exact(padding_size)?;
      }
    }

    Ok(files)
  }
}

pub fn extract_tar_file(
  compressed_data: &[u8],
  max_extracted_bytes: usize,
) -> Result<HashMap<String, Vec<u8>>, ReadError> {
  // Try compressed Reader first, then BufferReader.
  let compressed_reader = CompressedReader::new(compressed_data);
  let mut tar_reader = TarReader::new(compressed_reader, max_extracted_bytes);
  let compressed_error = match tar_reader.read_all_files() {
    Ok(files) => return Ok(files),
    Err(e) => e,
  };
  // If compressed reading fails, try with a BufferReader.
  let buffer_reader = BufferReader::new(compressed_data);
  let mut tar_reader = TarReader::new(buffer_reader, max_extracted_bytes);
  let uncompressed_error = match tar_reader.read_all_files() {
    Ok(files) => return Ok(files),
    Err(e) => e,
  };
  // If both methods fail, return a DynamicError combining both errors.
  Err(ReadError::Io(Box::new(DynamicError(format!(
    "Failed to extract TAR file: Compressed error: {:?}, Uncompressed error: {:?}",
    compressed_error, uncompressed_error
  )))))
}
