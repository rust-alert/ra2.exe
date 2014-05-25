//! 集成测试：原 `src/shp/mod.rs` 内联测试迁出。

use ra_assets::*;

fn raw_frame_shp() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&[0, 0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    let offset = (8 + 24) as u32;
    data.extend_from_slice(&offset.to_le_bytes());
    data.push(5);
    data
}

#[test]
fn parse_raw_one_pixel() {
    let shp = ShpFile::parse(&raw_frame_shp()).unwrap();
    assert_eq!(shp.frame_count(), 1);
    assert_eq!(shp.frames[0].pixels, vec![5]);
}
