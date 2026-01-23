// Copyright (C) 2025-2026 Daniel Mueller <deso@posteo.net>
// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use std::cmp::min;
use std::io;
use std::mem::MaybeUninit;


/// A type implementing `io::Write` for a potentially uninitialized
/// slice of memory.
///
/// The explicit intent is to enable formatted writing to uninitialized
/// stack allocated memory, which is "allocatable" with a mere increment
/// of the stack pointer. E.g.,
///
/// ```rust
/// # use std::io::stdout;
/// # use std::io::Write as _;
/// # use std::mem::MaybeUninit;
/// let mut buffer = [MaybeUninit::<u8>::uninit(); 256];
/// let mut writer = bufio::Writer::new(&mut buffer);
/// write!(writer, "{:x}", 1337).unwrap();
/// // Of course if you are writing to stdout you might as well use
/// // `print!` or `write!`; this example is just for illustration
/// // purposes.
/// stdout().write(writer.written()).unwrap();
/// ```
#[derive(Debug)]
pub struct Writer<'buf> {
  /// The underlying stack allocated buffer.
  buffer: &'buf mut [MaybeUninit<u8>],
  /// The total number of bytes written to `buffer`.
  written: usize,
}

impl<'buf> Writer<'buf> {
  /// Create a new [`Writer`] using the provided slice as buffer memory
  /// to write to.
  #[inline]
  pub fn new(buffer: &'buf mut [MaybeUninit<u8>]) -> Self {
    Self { buffer, written: 0 }
  }

  /// Retrieve the slice of the managed buffer that has been written so
  /// far.
  #[inline]
  pub fn written(&self) -> &[u8] {
    let slice = &self.buffer[0..self.written];
    // TODO: Use `MaybeUninit::assume_init_ref` once our MSRV is
    //       1.93.
    // SAFETY: This type guarantees that `written` bytes have been
    //         initialized in the buffer.
    unsafe { &*(slice as *const [MaybeUninit<u8>] as *const [u8]) }
  }

  /// Reset the buffer to its "empty" state.
  #[inline]
  pub fn reset(&mut self) {
    self.written = 0;
  }
}

impl io::Write for Writer<'_> {
  #[inline]
  fn write(&mut self, data: &[u8]) -> io::Result<usize> {
    let len = min(data.len(), self.buffer.len() - self.written);
    let ptr = self.buffer[self.written..].as_mut_ptr().cast::<u8>();
    // SAFETY: Both source and destination are valid for reads and are
    //         properly aligned as they originate from references. They
    //         cannot overlap because this method has exclusive access
    //         to the buffer we write to.
    let () = unsafe { ptr.copy_from_nonoverlapping(data.as_ptr(), len) };

    self.written += len;
    Ok(len)
  }

  #[inline]
  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::io::Write as _;


  /// Check that our `Writer` works as expected.
  #[test]
  fn stack_writing() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 8];
    let mut writer = Writer::new(&mut buffer);

    assert_eq!(writer.written(), []);
    let n = writer.write(b"1").unwrap();
    assert_eq!(n, 1);
    assert_eq!(writer.written(), [b'1']);

    let n = writer.write(b"23").unwrap();
    assert_eq!(n, 2);
    assert_eq!(writer.written(), [b'1', b'2', b'3']);

    let () = writer.reset();
    assert_eq!(writer.written(), []);

    let n = writer.write(b"456").unwrap();
    assert_eq!(n, 3);
    assert_eq!(writer.written(), [b'4', b'5', b'6']);

    let n = writer.write(b"123456").unwrap();
    assert_eq!(n, 5);
    assert_eq!(
      writer.written(),
      [b'4', b'5', b'6', b'1', b'2', b'3', b'4', b'5']
    );

    let n = writer.write(b"1337").unwrap();
    assert_eq!(n, 0);
    assert_eq!(
      writer.written(),
      [b'4', b'5', b'6', b'1', b'2', b'3', b'4', b'5']
    );
  }
}
