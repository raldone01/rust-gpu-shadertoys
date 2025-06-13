use alloc::{boxed::Box, format};
use miniz_oxide::{
  inflate::stream::{inflate, InflateState},
  MZError,
};

use crate::{
  buffered_reader::{Read, ReadError},
  dynamic_error::DynamicError,
};

pub struct CompressedReader<'a> {
  compressed_data: &'a [u8],
  decompressor: InflateState,
  current_position: usize,
}

impl<'a> CompressedReader<'a> {
  #[must_use]
  pub fn new(compressed_data: &'a [u8]) -> Self {
    Self {
      compressed_data,
      decompressor: InflateState::new(miniz_oxide::DataFormat::Zlib),
      current_position: 0,
    }
  }
}

impl Read for CompressedReader<'_> {
  fn read(&mut self, buf: &mut [u8]) -> Result<usize, ReadError> {
    if self.current_position >= self.compressed_data.len() {
      return Ok(0); // No more data to read
    }

    let remaining_data = &self.compressed_data[self.current_position..];
    let result = inflate(
      &mut self.decompressor,
      remaining_data,
      buf,
      miniz_oxide::MZFlush::None,
    );
    self.current_position += result.bytes_consumed;
    match result.status {
      Ok(_) => Ok(result.bytes_written),
      Err(e) => {
        if e == MZError::Buf {
          Err(ReadError::BufferTooSmall)
        } else {
          Err(ReadError::Io(Box::new(DynamicError(format!(
            "Decompression error: {:?}",
            e
          )))))
        }
      },
    }
  }
}
