//! 集成测试：原 `src/image/fnt.rs` 内联测试迁出。

use ra_assets::image::fnt::{FONT_MAGIC, FntFile};

const LOOKUP_TABLE_BYTES: usize = 65536 * 2;
const HEADER_BYTES: usize = 4 + 6 * 4;

fn tiny_fnt_with_letter_a() -> Vec<u8> {
    let bytes_per_row = 1u32;
    let bitmap_rows = 1u32;
    let cell_height = 2u32;
    let num_slots = 1u32;
    let glyph_stride = 1 + bytes_per_row * bitmap_rows;
    let mut data = Vec::new();
    data.extend_from_slice(&FONT_MAGIC.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes()); // field0
    data.extend_from_slice(&bytes_per_row.to_le_bytes());
    data.extend_from_slice(&bitmap_rows.to_le_bytes());
    data.extend_from_slice(&cell_height.to_le_bytes());
    data.extend_from_slice(&num_slots.to_le_bytes());
    data.extend_from_slice(&glyph_stride.to_le_bytes());
    let mut lut = vec![0u8; LOOKUP_TABLE_BYTES];
    // 'A' -> glyph index 1
    let off = (b'A' as usize) * 2;
    lut[off] = 1;
    lut[off + 1] = 0;
    data.extend_from_slice(&lut);
    // glyph: width 1, one pixel on
    data.push(1);
    data.push(0b1000_0000);
    data
}

#[test]
fn parse_rejects_bad_magic() {
    let err = FntFile::parse(&[0u8; HEADER_BYTES + LOOKUP_TABLE_BYTES]).unwrap_err();
    assert!(err.to_string().contains("魔数"));
}

#[test]
fn parse_tiny_letter_a() {
    let fnt = FntFile::parse(&tiny_fnt_with_letter_a()).unwrap();
    assert_eq!(fnt.glyph_count(), 1);
    let g = fnt.glyph(u16::from(b'A')).unwrap();
    assert_eq!(g.width, 1);
    assert_eq!(g.rgba, vec![255, 255, 255, 255]);
    assert_eq!(fnt.text_width("A"), 1);
    assert_eq!(fnt.text_width("AA"), 3);
}
