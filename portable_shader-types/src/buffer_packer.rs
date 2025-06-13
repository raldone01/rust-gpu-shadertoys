use bytemuck::NoUninit;

/// Packs a byte buffer with properly aligned `Pod` data.
struct BufferPacker<'a> {
  offset: usize,
  buffer: &'a mut [u8],
}

impl<'a> BufferPacker<'a> {
  /// Create a new BufferPacker over a mutable byte slice.
  #[must_use]
  pub fn new(buffer: &'a mut [u8]) -> Self {
    Self { buffer, offset: 0 }
  }

  /// Align the current offset to `alignment`.
  #[must_use]
  fn align_offset(&self, align: usize) -> usize {
    let rem = self.offset % align;
    if rem == 0 {
      self.offset
    } else {
      self.offset + (align - rem)
    }
  }

  /// Pack a single `NoUninit` struct, respecting alignment.
  /// Returns the offset at which it was written.
  pub fn pack<T: NoUninit>(&mut self, value: &T) -> Result<usize, ()> {
    let align = align_of::<T>();
    let size = size_of::<T>();

    let aligned_offset = self.align_offset(align);
    let end = aligned_offset + size;

    if end > self.buffer.len() {
      return Err(());
    }

    let dst = &mut self.buffer[aligned_offset..end];
    let src = bytemuck::bytes_of(value);
    dst.copy_from_slice(src);

    self.offset = end;
    Ok(aligned_offset)
  }

  /// Returns the current offset (end of packed data).
  #[must_use]
  pub fn current_offset(&self) -> usize {
    self.offset
  }

  /// Returns the remaining size in the buffer.
  #[must_use]
  pub fn remaining(&self) -> usize {
    self.buffer.len().saturating_sub(self.offset)
  }
}
