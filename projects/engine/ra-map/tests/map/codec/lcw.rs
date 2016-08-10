//! 自顶层 `lcw.rs`。

use ra_map::lcw::{decompress_chunks, lcw_decompress};

#[test]
fn literal_copy() {
    let src = [0x83, 0xAA, 0xBB, 0xCC, 0x80];
    let mut dest = [0u8; 16];
    let n = lcw_decompress(&src, &mut dest).unwrap();
    assert_eq!(n, 3);
    assert_eq!(&dest[..3], &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn rle_fill() {
    let src = [0xFE, 0x05, 0x00, 0x42, 0x80];
    let mut dest = [0u8; 16];
    let n = lcw_decompress(&src, &mut dest).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&dest[..5], &[0x42; 5]);
}

#[test]
fn relative_backref() {
    let src = [0x84, 10, 20, 30, 40, 0x00, 0x04, 0x80];
    let mut dest = [0u8; 16];
    let n = lcw_decompress(&src, &mut dest).unwrap();
    assert_eq!(n, 7);
    assert_eq!(&dest[..7], &[10, 20, 30, 40, 10, 20, 30]);
}

#[test]
fn distance_one_rle() {
    let src = [0x81, 0xFF, 0x20, 0x01, 0x80];
    let mut dest = [0u8; 16];
    let n = lcw_decompress(&src, &mut dest).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&dest[..6], &[0xFF; 6]);
}

#[test]
fn end_marker() {
    let mut dest = [0u8; 4];
    assert_eq!(lcw_decompress(&[0x80], &mut dest).unwrap(), 0);
}

#[test]
fn chunk_frame() {
    let payload = [0x83, 0xAA, 0xBB, 0xCC, 0x80];
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&5u16.to_le_bytes());
    chunk.extend_from_slice(&3u16.to_le_bytes());
    chunk.extend_from_slice(&payload);
    assert_eq!(decompress_chunks(&chunk).unwrap(), vec![0xAA, 0xBB, 0xCC]);
}
