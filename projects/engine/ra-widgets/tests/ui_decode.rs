//! 集成测试：原 `src/ui_decode.rs` 内联测试迁出。

use ra_assets::{Palette, ShpFile, mix_hash};
use ra_widgets::ui_decode::*;

fn tiny_pal() -> Vec<u8> {
    let mut pal = vec![0u8; 768];
    // index 1 = opaque red (6-bit style scaled by parser — raw bytes fine for smoke)
    pal[3] = 63;
    pal
}

fn tiny_shp_one_pixel_frame() -> Vec<u8> {
    // SHP(TS): zero, w=2, h=2, frames=1, then 24-byte header, then 1 pixel?
    // Simpler: 2x2 canvas, frame 0 at (0,0) size 2x2 raw pixels [1,0,0,1]
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    let data_offset = 8 + 24;
    // frame header
    data.extend_from_slice(&0u16.to_le_bytes()); // x
    data.extend_from_slice(&0u16.to_le_bytes()); // y
    data.extend_from_slice(&2u16.to_le_bytes()); // w
    data.extend_from_slice(&2u16.to_le_bytes()); // h
    data.push(0); // format raw
    data.extend_from_slice(&[0u8; 3]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(data_offset as u32).to_le_bytes());
    data.extend_from_slice(&[1, 0, 0, 1]);
    data
}

#[test]
fn frame_to_canvas_keeps_shp_canvas_size() {
    let shp = ShpFile::parse(&tiny_shp_one_pixel_frame()).unwrap();
    let pal = Palette::parse(&tiny_pal()).unwrap();
    let img = frame_to_canvas_rgba(&shp, &shp.frames[0], &pal).unwrap();
    assert_eq!((img.width(), img.height()), (2, 2));
    assert_eq!(img.as_raw().len(), 16);
}

#[test]
fn mix_hash_smoke_for_future_mem_source() {
    assert_ne!(mix_hash("a.shp"), mix_hash("b.shp"));
}
