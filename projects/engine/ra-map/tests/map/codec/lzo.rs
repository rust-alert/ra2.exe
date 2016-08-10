//! 自顶层 `lzo.rs`。

use ra_map::lzo::{decompress_chunks, lzo1x_decompress};

#[test]
fn test_literal_only_stream() {
    // A stream with just literals: first byte = 17 + count, then literal bytes,
    // then end-of-stream marker (0x11, 0x00, 0x00).
    let input: Vec<u8> = vec![
        17 + 5, // 5 literal bytes follow
        b'H',
        b'e',
        b'l',
        b'l',
        b'o',
        0x11,
        0x00,
        0x00, // end of stream
    ];
    let mut output: [u8; 64] = [0u8; 64];
    let written: usize = lzo1x_decompress(&input, &mut output).expect("Should decompress");
    assert_eq!(written, 5);
    assert_eq!(&output[..5], b"Hello");
}

#[test]
fn test_chunk_wrapper() {
    // Single chunk containing a literal-only LZO stream.
    let lzo_data: Vec<u8> = vec![17 + 3, b'A', b'B', b'C', 0x11, 0x00, 0x00];
    let mut chunks: Vec<u8> = Vec::new();
    let src_len: u16 = lzo_data.len() as u16;
    let dst_len: u16 = 3;
    chunks.extend_from_slice(&src_len.to_le_bytes());
    chunks.extend_from_slice(&dst_len.to_le_bytes());
    chunks.extend_from_slice(&lzo_data);

    let result: Vec<u8> = decompress_chunks(&chunks).expect("Should decompress chunks");
    assert_eq!(result, b"ABC");
}

#[test]
fn test_empty_input() {
    let mut output: [u8; 16] = [0u8; 16];
    let written: usize = lzo1x_decompress(&[], &mut output).expect("Empty OK");
    assert_eq!(written, 0);
}
