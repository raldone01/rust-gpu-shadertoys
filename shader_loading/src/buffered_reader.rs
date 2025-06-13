use alloc::{boxed::Box, vec::Vec};

use thiserror::Error;

/// Minimal I/O error type
#[derive(Error, Debug)]
pub enum ReadError {
  #[error("Underlying I/O error")]
  Io(#[from] Box<dyn core::error::Error + Send + Sync>),
  #[error("Buffer too small for requested read size")]
  BufferTooSmall,
  #[error("Unexpected end of file while reading")]
  UnexpectedEof,
  #[error("Memory limit exceeded for buffered read")]
  MemoryLimitExceeded,
}

/// Trait for reading bytes
pub trait Read {
  /// Read up to `buf.len()` bytes into `buf`.
  ///
  /// Returns number of bytes read, or `Error::Io`.
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, ReadError>;
}

pub struct BufferReader<'a> {
  source: &'a [u8],
  position: usize,
}

impl<'a> BufferReader<'a> {
  #[must_use]
  pub fn new(source: &'a [u8]) -> Self {
    Self {
      source,
      position: 0,
    }
  }
}

impl<'a> Read for BufferReader<'a> {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, ReadError> {
    if self.position >= self.source.len() {
      return Ok(0); // No more data to read
    }

    // Determine the number of bytes available in the source from the current position.
    let remaining_in_source = self.source.len() - self.position;

    // Determine the number of bytes to read. It's the minimum of the buffer's
    // capacity and the number of bytes remaining in our source.
    let bytes_to_read = core::cmp::min(buf.len(), remaining_in_source);

    // Get the part of the source slice we are going to copy from.
    let source_slice = &self.source[self.position..self.position + bytes_to_read];

    // Get the part of the destination buffer we are going to copy into.
    let dest_slice = &mut buf[..bytes_to_read];

    // Copy the bytes.
    dest_slice.copy_from_slice(source_slice);

    // Advance our position in the source.
    self.position += bytes_to_read;

    // Return the number of bytes that were read.
    Ok(bytes_to_read)
  }
}

/// A buffered reader that allows pulling exact sized chunks from a reader.
pub struct BufferedReader<R: Read> {
  source: R,
  buffer: Vec<u8>,
  last_user_read: usize,
  bytes_in_buffer: usize,
  max_buffer_size: usize,
}

impl<R: Read> BufferedReader<R> {
  #[must_use]
  pub fn new(max_buffer_size: usize, source: R) -> Self {
    Self {
      source,
      buffer: Vec::new(),
      last_user_read: 0,
      bytes_in_buffer: 0,
      max_buffer_size,
    }
  }

  /// Reads exactly `byte_count` bytes from the reader.
  pub fn read_exact(&mut self, byte_count: usize) -> Result<&[u8], ReadError> {
    if byte_count > self.max_buffer_size {
      return Err(ReadError::MemoryLimitExceeded);
    }
    if byte_count == 0 {
      return Ok(&[]);
    }

    // Move the remaining bytes in the buffer to the front.
    self
      .buffer
      .copy_within(self.last_user_read..self.bytes_in_buffer, 0);
    self.bytes_in_buffer -= self.last_user_read;

    // If the buffer is smaller than the requested size, we need to fill it.
    while self.bytes_in_buffer < byte_count {
      // If the buffer is full, we need to resize it.
      // Use a doubling strategy to avoid frequent reallocations.
      if self.buffer.len() < self.max_buffer_size {
        let new_len = (self.buffer.len() * 2)
          .max(byte_count)
          .min(self.max_buffer_size);
        if new_len == self.buffer.len() {
          // If the buffer is already at max size, we can't grow it further.
          return Err(ReadError::MemoryLimitExceeded);
        }
        self.buffer.resize(new_len, 0);
      }

      // Read more data into the buffer.
      let bytes_read = self.source.read(&mut self.buffer[self.bytes_in_buffer..])?;
      if bytes_read == 0 {
        return Err(ReadError::UnexpectedEof);
      }
      self.bytes_in_buffer += bytes_read;
    }

    // Now we have enough data in the buffer, return the requested slice.
    self.last_user_read = byte_count;
    let result = &self.buffer[..byte_count];
    Ok(result)
  }
}

#[cfg(test)]
mod tests {
  use crate::dynamic_error::DynamicError;

  use super::*;
  use alloc::{
    format,
    vec::{self, Vec},
  };

  /// A mock reader for testing purposes that can simulate I/O errors and EOF.
  struct MockReader<'a> {
    data: &'a [u8],
    pos: usize,
    /// If Some, the reader will return `ReadError::Io` after this many bytes have been successfully read.
    fail_after_n_bytes: Option<usize>,
    /// The total number of bytes read from this mock reader so far.
    bytes_read_count: usize,
  }

  impl<'a> MockReader<'a> {
    fn new(data: &'a [u8]) -> Self {
      Self {
        data,
        pos: 0,
        fail_after_n_bytes: None,
        bytes_read_count: 0,
      }
    }

    /// Configures the mock reader to fail after a certain number of bytes.
    fn with_fail_after(mut self, n: usize) -> Self {
      self.fail_after_n_bytes = Some(n);
      self
    }
  }

  impl<'a> Read for MockReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ReadError> {
      // 1. Check if we have reached the failure point.
      if let Some(fail_at) = self.fail_after_n_bytes {
        if self.bytes_read_count >= fail_at {
          return Err(ReadError::Io(Box::new(DynamicError(format!(
            "MockReader failed after {} bytes",
            fail_at
          )))));
        }
      }

      // 2. Check for End-Of-File.
      if self.pos >= self.data.len() {
        return Ok(0);
      }

      // 3. Determine how many bytes to read in this call.
      // It's the minimum of the buffer's available space, and the data remaining in our source.
      let mut bytes_to_read = (self.data.len() - self.pos).min(buf.len());

      // 4. If a failure is scheduled, ensure we don't read past the failure point in this single call.
      if let Some(fail_at) = self.fail_after_n_bytes {
        let remaining_until_fail = fail_at.saturating_sub(self.bytes_read_count);
        bytes_to_read = bytes_to_read.min(remaining_until_fail);
      }

      if bytes_to_read == 0 {
        return Ok(0);
      }

      // 5. Copy the data and update our internal state.
      let read_slice = &self.data[self.pos..self.pos + bytes_to_read];
      buf[..bytes_to_read].copy_from_slice(read_slice);

      self.pos += bytes_to_read;
      self.bytes_read_count += bytes_to_read;

      Ok(bytes_to_read)
    }
  }

  #[test]
  fn test_simple_reads() {
    let source_data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mock_reader = MockReader::new(&source_data);
    let mut reader = BufferedReader::new(10, mock_reader);

    // Read the first 3 bytes
    assert_eq!(reader.read_exact(3).unwrap(), &[0, 1, 2]);

    // Read the next 4 bytes. The buffer should handle the internal offset.
    assert_eq!(reader.read_exact(4).unwrap(), &[3, 4, 5, 6]);

    // The remaining data in the source should be copied and returned.
    assert_eq!(reader.read_exact(3).unwrap(), &[7, 8, 9]);
  }

  #[test]
  fn test_mid_stream_io_failure() {
    let source_data: Vec<u8> = (0..20).collect();
    // This mock reader will provide 8 bytes successfully, then fail.
    let mock_reader = MockReader::new(&source_data).with_fail_after(8);
    let mut reader = BufferedReader::new(15, mock_reader);

    // We request 10 bytes. The buffer needs to read from the source to fulfill this.
    // The first internal read will get 8 bytes.
    // The `while` loop will execute again to get the remaining 2 bytes.
    // The second internal read will trigger the I/O error.
    let result = reader.read_exact(10);

    // Assert that the error was correctly propagated.
    assert!(result.is_err());
  }

  #[test]
  fn test_unexpected_eof() {
    let source_data = [0, 1, 2, 3, 4]; // Only 5 bytes available
    let mock_reader = MockReader::new(&source_data);
    let mut reader = BufferedReader::new(10, mock_reader);

    // Try to read 6 bytes, which is more than available.
    let result = reader.read_exact(6);
    assert!(matches!(result, Err(ReadError::UnexpectedEof)));
  }

  #[test]
  fn test_unexpected_eof_after_successful_read() {
    let source_data = [0, 1, 2, 3, 4]; // 5 bytes total
    let mock_reader = MockReader::new(&source_data);
    let mut reader = BufferedReader::new(10, mock_reader);

    // Successfully read 3 bytes, leaving 2 in the source.
    assert_eq!(reader.read_exact(3).unwrap(), &[0, 1, 2]);

    // Now, try to read 3 more bytes. Only 2 are available.
    let result = reader.read_exact(3);
    assert!(matches!(result, Err(ReadError::UnexpectedEof)));
  }

  #[test]
  fn test_memory_limit_exceeded() {
    let source_data = [0; 100];
    let mock_reader = MockReader::new(&source_data);
    // Set a max buffer size of 50.
    let mut reader = BufferedReader::new(50, mock_reader);

    // Requesting more than the max size should fail immediately.
    let result = reader.read_exact(51);
    assert!(matches!(result, Err(ReadError::MemoryLimitExceeded)));
  }

  #[test]
  fn test_buffer_growth_strategy() {
    let source_data: Vec<u8> = (0..30).collect();
    let mock_reader = MockReader::new(&source_data);
    let mut reader = BufferedReader::new(20, mock_reader); // Max size is 20

    // Initial buffer is empty. A read of 5 should resize it to at least 5.
    assert_eq!(reader.read_exact(5).unwrap(), &[0, 1, 2, 3, 4]);
    assert!(reader.buffer.len() >= 5);
    let len_after_first_read = reader.buffer.len();

    // The next read requires more data. The buffer should double in size.
    assert_eq!(reader.read_exact(10).unwrap(), &source_data[5..15]);
    assert_eq!(reader.buffer.len(), len_after_first_read * 2);

    // The next read would cause doubling to 20, which is the max size.
    assert_eq!(reader.read_exact(15).unwrap(), &source_data[15..30]);
    assert_eq!(reader.buffer.len(), 20); // Should be capped at max_buffer_size
  }

  #[test]
  fn test_read_zero_bytes() {
    let source_data = [1, 2, 3];
    let mock_reader = MockReader::new(&source_data);
    let mut reader = BufferedReader::new(10, mock_reader);

    // Reading 0 bytes should return an empty slice without error.
    let chunk = reader.read_exact(0).unwrap();
    assert_eq!(chunk, &[]);

    // The internal state should not have changed, and we can still read data.
    assert_eq!(reader.read_exact(3).unwrap(), &[1, 2, 3]);
  }

  #[test]
  fn test_exhausting_the_source() {
    let source_data = [0, 1, 2, 3, 4, 5];
    let mock_reader = MockReader::new(&source_data);
    let mut reader = BufferedReader::new(10, mock_reader);

    // Read in two chunks
    assert_eq!(reader.read_exact(4).unwrap(), &[0, 1, 2, 3]);
    assert_eq!(reader.read_exact(2).unwrap(), &[4, 5]);

    // The source is now exhausted. Any further read should result in UnexpectedEof.
    let result = reader.read_exact(1);
    assert!(matches!(result, Err(ReadError::UnexpectedEof)));
  }
}
