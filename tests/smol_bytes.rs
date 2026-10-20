#![allow(unused_imports)]

// Integration tests for the optional `smol-bytes` integration, mirroring `bytes_buf.rs`.
// Gated on `std` because the shared/compact `Bytes` and `BytesMut` holders only exist in
// smol's std/alloc tiers; the pure-core `Buffer` tier is additionally covered here (it works
// under std too) and its no_std build is checked separately.
#[cfg(test)]
#[cfg(all(feature = "smol_bytes_01", feature = "std"))]
mod smol_bytes_tests {
  use buffo::{Chunk, ChunkExt, EmptyChunk};
  use smol_bytes_01::{compact, Buffer, Bytes, BytesMut};

  // ---- shared::Bytes (Chunk) ------------------------------------------------------------

  #[test]
  fn shared_remaining_and_buffer() {
    let buf = Bytes::from(vec![1, 2, 3, 4, 5]);
    assert_eq!(buf.remaining(), 5);
    assert!(buf.has_remaining());
    assert_eq!(buf.buffer(), &[1, 2, 3, 4, 5]);
    assert_eq!(buf.remaining(), buf.buffer().len());

    let empty = Bytes::empty();
    assert_eq!(empty.remaining(), 0);
    assert!(!empty.has_remaining());
  }

  #[test]
  fn shared_advance_and_truncate() {
    let mut buf = Bytes::from(vec![1, 2, 3, 4, 5]);
    buf.advance(2);
    assert_eq!(buf.buffer(), &[3, 4, 5]);
    assert_eq!(buf.remaining(), buf.buffer().len());
    buf.truncate(2);
    assert_eq!(buf.buffer(), &[3, 4]);
    buf.truncate(10); // no-op when >= len
    assert_eq!(buf.remaining(), 2);
  }

  #[test]
  #[should_panic]
  fn shared_advance_panics() {
    let mut buf = Bytes::from(vec![1, 2, 3]);
    buf.advance(5);
  }

  #[test]
  fn shared_split_and_segment() {
    let base = Bytes::from(b"hello world".to_vec());

    let mut a = base.clone();
    let tail = a.split_off(5);
    assert_eq!(a.buffer(), b"hello");
    assert_eq!(tail.buffer(), b" world");

    let mut b = base.clone();
    let head = b.split_to(5);
    assert_eq!(head.buffer(), b"hello");
    assert_eq!(b.buffer(), b" world");

    assert_eq!(base.segment(0..5).buffer(), b"hello");
    assert_eq!(base.segment(6..11).buffer(), b"world");
  }

  #[test]
  fn shared_read_primitives() {
    let mut buf = Bytes::from(vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
    assert_eq!(buf.read_u8(), 0x12);
    assert_eq!(buf.read_u16_le(), 0x5634);
    assert_eq!(buf.read_u32_be(), 0x789ABCDE);
    assert_eq!(buf.read_u8(), 0xF0);
    assert_eq!(buf.remaining(), 0);
  }

  #[test]
  fn shared_try_read_reports_error() {
    let mut buf = Bytes::from(vec![0x12, 0x34]);
    let err = buf.try_read_u32_le().unwrap_err();
    assert_eq!(err.requested().get(), 4);
    assert_eq!(err.available(), 2);
    // failed read did not consume
    assert_eq!(buf.remaining(), 2);
  }

  #[test]
  fn shared_conversions_roundtrip() {
    let mut buf = Bytes::from(vec![1, 2, 3, 4, 5]);
    buf.advance(1);
    assert_eq!(buf.to_vec(), vec![2, 3, 4, 5]);
    assert_eq!(buf.to_smol_bytes().buffer(), &[2, 3, 4, 5]);
    assert_eq!(buf.to_smol_bytes_mut().buffer(), &[2, 3, 4, 5]);
    // to_bytes / to_bytes_mut (bytes crate) require `bytes_1` in addition to
    // `smol_bytes_01` + `std`; gate only these two assertions so the rest of this
    // test still runs under smol_bytes_01+std without bytes_1.
    #[cfg(feature = "bytes_1")]
    assert_eq!(buf.to_bytes().as_ref(), &[2, 3, 4, 5]);
    #[cfg(feature = "bytes_1")]
    assert_eq!(buf.to_bytes_mut().as_ref(), &[2, 3, 4, 5]);
  }

  // ---- compact::Bytes (Chunk) -----------------------------------------------------------

  #[test]
  fn compact_basic() {
    let mut buf = compact::Bytes::copy_from_slice(b"abcd");
    assert_eq!(buf.remaining(), 4);
    assert_eq!(buf.remaining(), buf.buffer().len());
    assert_eq!(buf.read_u8(), b'a');
    assert_eq!(buf.buffer(), b"bcd");
    assert_eq!(buf.segment(0..2).buffer(), b"bc");
    assert_eq!(compact::Bytes::empty().remaining(), 0);
  }

  // ---- BytesMut (Chunk) ------------------------------------------------------------------

  #[test]
  fn bytes_mut_split_inline_path() {
    let mut b = BytesMut::from(&b"hello world"[..]);
    assert!(b.is_inline());
    let tail = Chunk::split_off(&mut b, 5);
    assert_eq!(Chunk::buffer(&b), b"hello");
    assert_eq!(Chunk::buffer(&tail), b" world");

    let mut b2 = BytesMut::from(&b"hello world"[..]);
    let head = Chunk::split_to(&mut b2, 5);
    assert_eq!(Chunk::buffer(&head), b"hello");
    assert_eq!(Chunk::buffer(&b2), b" world");
  }

  #[test]
  fn bytes_mut_split_heap_path() {
    let mut b = BytesMut::with_capacity(128);
    b.extend_from_slice(b"hello world");
    assert!(b.is_heap());
    let tail = Chunk::split_off(&mut b, 5);
    assert_eq!(Chunk::buffer(&b), b"hello");
    assert_eq!(Chunk::buffer(&tail), b" world");
  }

  // Regression: `len < at <= capacity` must panic (smol alone would not).
  #[test]
  #[should_panic]
  fn bytes_mut_split_off_beyond_len_panics() {
    let mut b = BytesMut::with_capacity(128);
    b.extend_from_slice(&[1, 2, 3, 4, 5]);
    assert!(b.capacity() >= 10);
    let _ = Chunk::split_off(&mut b, 10);
  }

  // ---- Buffer (Chunk), pure-core-capable holder -----------------------------------------

  #[test]
  fn buffer_chunk_roundtrip() {
    let mut buf = Buffer::try_from(&b"hello world"[..]).unwrap();
    assert_eq!(buf.remaining(), 11);
    assert_eq!(buf.remaining(), buf.buffer().len());
    assert_eq!(buf.read_u8(), b'h');
    assert_eq!(buf.buffer(), b"ello world");
    assert_eq!(buf.remaining(), buf.buffer().len());

    let mut a = Buffer::try_from(&b"hello world"[..]).unwrap();
    let tail = a.split_off(5);
    assert_eq!(a.buffer(), b"hello");
    assert_eq!(tail.buffer(), b" world");

    assert_eq!(Buffer::empty().remaining(), 0);
  }
}

// smol-bytes 0.2 keeps the byte-holder surface parallel to 0.1 while its core `Buffer`
// gains crate-owned, inherent fixed-width readers. Keep this module version-qualified so the
// two optional versions can be enabled together without type or method ambiguity.
#[cfg(test)]
#[cfg(all(feature = "smol_bytes_02", feature = "std"))]
mod smol_bytes_02_tests {
  use buffo::{Chunk, EmptyChunk};
  use smol_bytes_02::{compact, Buffer, Bytes, BytesMut};

  #[test]
  fn shared_chunk_fixed_reads_and_underflow_are_atomic() {
    let mut buf = Bytes::copy_from_slice([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(buf.remaining(), 4);
    assert!(buf.has_remaining());
    assert_eq!(buf.read_u16_le(), 0x3412);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let err = buf.try_read_u32_be().unwrap_err();
    assert_eq!(err.requested().get(), 4);
    assert_eq!(err.available(), 2);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let empty = <Bytes as EmptyChunk>::empty();
    assert_eq!(empty.remaining(), 0);
    assert!(!empty.has_remaining());
  }

  #[test]
  fn shared_segment_and_split() {
    let base = Bytes::copy_from_slice(b"hello world");

    let mut left = base.clone();
    let tail = left.split_off(5);
    assert_eq!(left.buffer(), b"hello");
    assert_eq!(tail.buffer(), b" world");

    let mut right = base.clone();
    let head = right.split_to(5);
    assert_eq!(head.buffer(), b"hello");
    assert_eq!(right.buffer(), b" world");
    assert_eq!(base.segment(6..11).buffer(), b"world");
  }

  #[test]
  fn compact_chunk_fixed_reads_and_underflow_are_atomic() {
    let mut buf = compact::Bytes::copy_from_slice([0x12, 0x34, 0x56, 0x78]);
    assert_eq!(buf.remaining(), 4);
    assert_eq!(buf.read_u16_le(), 0x3412);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let err = buf.try_read_u32_be().unwrap_err();
    assert_eq!(err.requested().get(), 4);
    assert_eq!(err.available(), 2);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let empty = <compact::Bytes as EmptyChunk>::empty();
    assert_eq!(empty.remaining(), 0);
    assert!(!empty.has_remaining());
  }

  #[test]
  fn compact_segment_and_split() {
    let base = compact::Bytes::copy_from_slice(b"hello world");

    let mut left = base.clone();
    let tail = left.split_off(5);
    assert_eq!(left.buffer(), b"hello");
    assert_eq!(tail.buffer(), b" world");

    let mut right = base.clone();
    let head = right.split_to(5);
    assert_eq!(head.buffer(), b"hello");
    assert_eq!(right.buffer(), b" world");
    assert_eq!(base.segment(0..5).buffer(), b"hello");
  }

  #[test]
  fn bytes_mut_fixed_reads_and_underflow_are_atomic() {
    let mut buf = BytesMut::from(&[0x12, 0x34, 0x56, 0x78][..]);
    assert_eq!(buf.remaining(), 4);
    assert_eq!(buf.read_u16_le(), 0x3412);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let err = buf.try_read_u32_be().unwrap_err();
    assert_eq!(err.requested().get(), 4);
    assert_eq!(err.available(), 2);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let empty = <BytesMut as EmptyChunk>::empty();
    assert_eq!(empty.remaining(), 0);
    assert!(!empty.has_remaining());
  }

  #[test]
  fn bytes_mut_split_inline_and_heap_paths() {
    let mut inline = BytesMut::from(&b"hello world"[..]);
    assert!(inline.is_inline());
    let tail = Chunk::split_off(&mut inline, 5);
    assert_eq!(inline.buffer(), b"hello");
    assert_eq!(tail.buffer(), b" world");

    let mut inline_to = BytesMut::from(&b"hello world"[..]);
    let head = Chunk::split_to(&mut inline_to, 5);
    assert_eq!(head.buffer(), b"hello");
    assert_eq!(inline_to.buffer(), b" world");

    let mut heap = BytesMut::with_capacity(128);
    heap.extend_from_slice(b"hello world");
    assert!(heap.is_heap());
    let tail = Chunk::split_off(&mut heap, 5);
    assert_eq!(heap.buffer(), b"hello");
    assert_eq!(tail.buffer(), b" world");
  }

  #[test]
  #[should_panic]
  fn bytes_mut_split_off_between_len_and_capacity_panics() {
    let mut buf = BytesMut::with_capacity(128);
    buf.extend_from_slice(&[1, 2, 3, 4, 5]);
    assert!(buf.capacity() >= 10);
    let _ = Chunk::split_off(&mut buf, 10);
  }

  #[test]
  fn buffer_chunk_fixed_reads_and_underflow_are_atomic() {
    let mut buf = Buffer::try_from(&[0x12, 0x34, 0x56, 0x78][..]).unwrap();
    assert_eq!(buf.remaining(), 4);
    assert_eq!(buf.read_u16_le(), 0x3412);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let err = buf.try_read_u32_be().unwrap_err();
    assert_eq!(err.requested().get(), 4);
    assert_eq!(err.available(), 2);
    assert_eq!(buf.buffer(), &[0x56, 0x78]);

    let empty = <Buffer as EmptyChunk>::empty();
    assert_eq!(empty.remaining(), 0);
    assert!(!empty.has_remaining());
  }

  #[test]
  fn buffer_segment_and_split() {
    let mut buf = Buffer::try_from(&b"hello world"[..]).unwrap();
    assert_eq!(buf.segment(0..5).buffer(), b"hello");
    let tail = buf.split_off(5);
    assert_eq!(buf.buffer(), b"hello");
    assert_eq!(tail.buffer(), b" world");
  }

  #[test]
  fn version_02_conversion_return_types() {
    let source = &b"hello world"[..];
    let bytes: Bytes = source.to_smol_bytes_02();
    let bytes_mut: BytesMut = source.to_smol_bytes_mut_02();
    assert_eq!(bytes.buffer(), b"hello world");
    assert_eq!(bytes_mut.buffer(), b"hello world");

    let shared = Bytes::copy_from_slice(b"hello world");
    let converted: Bytes = shared.to_smol_bytes_02();
    assert_eq!(converted.buffer(), b"hello world");
  }
}

#[cfg(test)]
#[cfg(all(feature = "smol_bytes_01", feature = "smol_bytes_02", feature = "std"))]
mod smol_bytes_versions_coexist {
  use buffo::Chunk;

  #[test]
  fn version_qualified_conversions_are_unambiguous() {
    let source = &b"coexist"[..];

    let bytes_01: smol_bytes_01::Bytes = source.to_smol_bytes();
    let bytes_mut_01: smol_bytes_01::BytesMut = source.to_smol_bytes_mut();
    let bytes_02: smol_bytes_02::Bytes = source.to_smol_bytes_02();
    let bytes_mut_02: smol_bytes_02::BytesMut = source.to_smol_bytes_mut_02();

    assert_eq!(bytes_01.as_slice(), b"coexist");
    assert_eq!(bytes_mut_01.as_slice(), b"coexist");
    assert_eq!(bytes_02.as_slice(), b"coexist");
    assert_eq!(bytes_mut_02.as_slice(), b"coexist");
  }
}
