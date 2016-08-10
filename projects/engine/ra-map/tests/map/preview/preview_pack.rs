//! `[Preview]` / `[PreviewPack]` 解码单元测试。

use ra_map::{base64::base64_encode, decode_preview_from_map_bytes, decode_preview_pack, lzo, parse_preview_size};

fn lzo_literal_chunk(rgb: &[u8]) -> Vec<u8> {
    // 字面量流：首字节 17+n，随后 n 字节，再 0x11 0x00 0x00 结束。
    assert!(rgb.len() <= 238);
    let mut lzo_data = Vec::with_capacity(rgb.len() + 4);
    lzo_data.push(17 + rgb.len() as u8);
    lzo_data.extend_from_slice(rgb);
    lzo_data.extend_from_slice(&[0x11, 0x00, 0x00]);
    let mut chunks = Vec::new();
    chunks.extend_from_slice(&(lzo_data.len() as u16).to_le_bytes());
    chunks.extend_from_slice(&(rgb.len() as u16).to_le_bytes());
    chunks.extend_from_slice(&lzo_data);
    chunks
}

#[test]
fn parse_preview_size_wh_and_xywh() {
    assert_eq!(parse_preview_size("168,156"), Some((168, 156)));
    assert_eq!(parse_preview_size("0,0,120,90"), Some((120, 90)));
    assert_eq!(parse_preview_size("bad"), None);
}

#[test]
fn decode_preview_pack_rgb_to_rgba() {
    let rgb = [10u8, 20, 30, 40, 50, 60];
    let chunks = lzo_literal_chunk(&rgb);
    let b64 = base64_encode(&chunks);
    let img = decode_preview_pack(&b64, 2, 1).expect("decode");
    assert_eq!(img.width, 2);
    assert_eq!(img.height, 1);
    assert_eq!(img.rgba, vec![10, 20, 30, 255, 40, 50, 60, 255]);
}

#[test]
fn decode_preview_from_map_ini_xywh() {
    let rgb = [1u8, 2, 3];
    let chunks = lzo_literal_chunk(&rgb);
    let b64 = base64_encode(&chunks);
    let map = format!("[Preview]\nSize=0,0,1,1\n\n[PreviewPack]\n1={b64}\n");
    let img = decode_preview_from_map_bytes(map.as_bytes()).expect("parse").expect("some");
    assert_eq!(img.width, 1);
    assert_eq!(img.height, 1);
    assert_eq!(img.rgba, vec![1, 2, 3, 255]);
}

#[test]
fn decode_preview_missing_section_is_none() {
    let map = b"[Basic]\nName=test\n";
    assert!(decode_preview_from_map_bytes(map).expect("parse").is_none());
}

#[test]
fn decompress_roundtrip_matches_helper() {
    let rgb = [9u8, 8, 7, 6, 5, 4];
    let chunks = lzo_literal_chunk(&rgb);
    let out = lzo::decompress_chunks(&chunks).expect("lzo");
    assert_eq!(out, rgb);
}
