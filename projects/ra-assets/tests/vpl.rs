//! 集成测试：原 `src/vpl.rs` 内联测试迁出。

use ra_assets::*;

const PALETTE_BYTES: usize = 768;
const PAGE_SIZE: usize = 256;

fn sample_vpl() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&31u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&[0u8; PALETTE_BYTES]);
    data.extend_from_slice(&(0..=255u8).collect::<Vec<_>>());
    data.extend_from_slice(&[42u8; PAGE_SIZE]);
    data
}

#[test]
fn parse_two_pages() {
    let vpl = VplFile::parse(&sample_vpl()).unwrap();
    assert_eq!(vpl.first_remap, 16);
    assert_eq!(vpl.last_remap, 31);
    assert_eq!(vpl.page_count(), 2);
    assert_eq!(vpl.remap_color(0, 100), 100);
    assert_eq!(vpl.remap_color(1, 200), 42);
    assert_eq!(vpl.remap_color(9, 7), 42);
}

#[test]
fn reject_tiny() {
    assert!(VplFile::parse(&[0u8; 8]).is_err());
}
