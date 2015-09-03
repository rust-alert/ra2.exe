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

fn multi_raw_frames(indices: &[&[u8]]) -> Vec<u8> {
    let frame_count = indices.len() as u16;
    let header = 8 + frame_count as usize * 24;
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&frame_count.to_le_bytes());
    let mut payload = Vec::new();
    for pixels in indices {
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&(pixels.len() as u16).to_le_bytes());
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&0u32.to_le_bytes());
        let offset = (header + payload.len()) as u32;
        data.extend_from_slice(&offset.to_le_bytes());
        payload.extend_from_slice(pixels);
    }
    data.extend_from_slice(&payload);
    data
}

#[test]
fn shadow_half_split_and_body_count() {
    assert_eq!(shp_shadow_half_base(0), None);
    assert_eq!(shp_shadow_half_base(5), None);
    assert_eq!(shp_shadow_half_base(6), Some(3));

    let populated = ShpFile::parse(&multi_raw_frames(&[&[10], &[20], &[30], &[1], &[1], &[1]])).unwrap();
    assert!(shp_shadow_half_populated(&populated.frames));
    assert_eq!(shp_body_frame_count(&populated.frames), 3);

    let empty_shadow = ShpFile::parse(&multi_raw_frames(&[&[10], &[20], &[], &[]])).unwrap();
    assert!(!shp_shadow_half_populated(&empty_shadow.frames));
    assert_eq!(shp_body_frame_count(&empty_shadow.frames), 4);

    // 两帧彩色动画：后半不是索引 1，不得当落影切半。
    let color_pair = ShpFile::parse(&multi_raw_frames(&[&[5], &[5]])).unwrap();
    assert!(!shp_shadow_half_populated(&color_pair.frames));
    assert_eq!(shp_body_frame_count(&color_pair.frames), 2);
}
