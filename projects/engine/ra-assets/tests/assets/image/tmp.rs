//! 集成测试：原 `src/tmp.rs` 内联测试迁出。

use ra_assets::*;

fn put_u32(buf: &mut [u8], o: usize, v: u32) {
    buf[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

#[test]
fn parse_one_cell_no_extra() {
    // 1x1 模板，8x4 单元，钻石 16 字节，无 Z/附加。
    let tile_w = 8u32;
    let tile_h = 4u32;
    let diamond = diamond_byte_count(tile_w, tile_h).unwrap();
    assert_eq!(diamond, 16);

    let cell_off = 20usize;
    let mut data = vec![0u8; cell_off + TILE_HEADER_SIZE + diamond];
    put_u32(&mut data, 0, 1);
    put_u32(&mut data, 4, 1);
    put_u32(&mut data, 8, tile_w);
    put_u32(&mut data, 12, tile_h);
    put_u32(&mut data, 16, cell_off as u32);
    // flags = 0
    for i in 0..diamond {
        data[cell_off + TILE_HEADER_SIZE + i] = (i as u8).wrapping_add(1);
    }

    let tmp = TmpFile::parse(&data).unwrap();
    assert_eq!(tmp.cell_count(), 1);
    let tile = tmp.tiles[0].as_ref().unwrap();
    assert_eq!(tile.pixel_width, 8);
    assert_eq!(tile.pixel_height, 4);
    // 第一行宽 4，居中写到 x=2..6
    assert_eq!(&tile.pixels[2..6], &[1, 2, 3, 4]);
}

#[test]
fn reject_tiny_header() {
    assert!(TmpFile::parse(&[0u8; 8]).is_err());
}

#[test]
fn tile_to_rgba_keeps_index_zero_transparent_inside_diamond() {
    let tile_w = 8u32;
    let tile_h = 4u32;
    let diamond = diamond_byte_count(tile_w, tile_h).unwrap();
    let cell_off = 20usize;
    let mut data = vec![0u8; cell_off + TILE_HEADER_SIZE + diamond];
    put_u32(&mut data, 0, 1);
    put_u32(&mut data, 4, 1);
    put_u32(&mut data, 8, tile_w);
    put_u32(&mut data, 12, tile_h);
    put_u32(&mut data, 16, cell_off as u32);
    // 钻石像素全为索引 0；调色板 0 为深蓝透明键。
    let mut pal = vec![0u8; 768];
    pal[2] = 31; // b≈125 after full expand
    let tmp = TmpFile::parse(&data).unwrap();
    let palette = Palette::parse(&pal).unwrap();
    assert_eq!(palette.colors[0].a, 0);
    let rgba = tmp.tile_to_rgba(0, &palette).unwrap();
    assert!(rgba.chunks_exact(4).all(|px| px[3] == 0), "index 0 must stay transparent");
}
