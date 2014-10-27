//! 集成测试：原 `src/image/bink_video.rs` 内联测试迁出。

use ra_assets::{
    image::bink::{BinkColorRange, BinkVersion, parse_bink_header},
    *,
};

fn tiny_header() -> ra_assets::image::bink::BinkHeader {
    let mut data = vec![0u8; 0x2C];
    data[0..4].copy_from_slice(&0x694B_4942u32.to_le_bytes());
    data[4..8].copy_from_slice(&(100u32 - 8).to_le_bytes());
    data[8..12].copy_from_slice(&1u32.to_le_bytes());
    data[12..16].copy_from_slice(&10u32.to_le_bytes());
    data[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
    data[0x18..0x1C].copy_from_slice(&8u32.to_le_bytes());
    data[0x1C..0x20].copy_from_slice(&10u32.to_le_bytes());
    data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
    parse_bink_header(&data).unwrap()
}

fn tiny_bikk_header() -> ra_assets::image::bink::BinkHeader {
    let mut data = vec![0u8; 0x30];
    data[0..4].copy_from_slice(&0x6B4B_4942u32.to_le_bytes()); // BIKk
    data[4..8].copy_from_slice(&(100u32 - 8).to_le_bytes());
    data[8..12].copy_from_slice(&1u32.to_le_bytes());
    data[12..16].copy_from_slice(&10u32.to_le_bytes());
    data[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
    data[0x18..0x1C].copy_from_slice(&8u32.to_le_bytes());
    data[0x1C..0x20].copy_from_slice(&10u32.to_le_bytes());
    data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
    // BIKk 额外 4 字节；无音轨时帧索引从 0x30 起。
    parse_bink_header(&data).unwrap()
}

#[test]
fn decoder_new_builds_vlc_and_rejects_short_packet() {
    let mut d = BinkVideoDecoder::new(&tiny_header()).unwrap();
    assert_eq!((d.width(), d.height()), (8, 8));
    assert_eq!(d.version(), BinkVersion::BikI);
    assert_eq!(d.vlc_tables().len(), 16);
    assert!(matches!(d.decode_packet(&[], true).unwrap_err(), BinkVideoError::Msg(_)));
    // 4 字节 = 32 位，跳过对齐槽后读树时码流耗尽。
    assert!(matches!(d.decode_packet(&[0, 0, 0, 0], true).unwrap_err(), BinkVideoError::Msg(_)));
}

#[test]
fn bikk_solid_fill_planes_return_frame() {
    let mut d = BinkVideoDecoder::new(&tiny_bikk_header()).unwrap();
    // 位流 LSB 优先拼进字节缓冲。
    let mut bytes = Vec::new();
    let mut bit_buf: u32 = 0;
    let mut bit_n: u32 = 0;
    let push = |val: u32, len: u32, bytes: &mut Vec<u8>, bit_buf: &mut u32, bit_n: &mut u32| {
        *bit_buf |= val << *bit_n;
        *bit_n += len;
        while *bit_n >= 8 {
            bytes.push((*bit_buf & 0xFF) as u8);
            *bit_buf >>= 8;
            *bit_n -= 8;
        }
    };
    push(0, 32, &mut bytes, &mut bit_buf, &mut bit_n);
    let mut n_total = 32u32;
    // BIKk 码流平面顺序为 Y/V/U，解码写入时对调为 Y/U/V。
    for color in [16u32, 200u32, 128u32] {
        push(1, 1, &mut bytes, &mut bit_buf, &mut bit_n);
        push(color, 8, &mut bytes, &mut bit_buf, &mut bit_n);
        n_total += 9;
        let pad = (32 - (n_total % 32)) % 32;
        push(0, pad, &mut bytes, &mut bit_buf, &mut bit_n);
        n_total += pad;
    }
    if bit_n > 0 {
        bytes.push((bit_buf & 0xFF) as u8);
    }
    let frame = d.decode_packet(&bytes, true).unwrap();
    assert_eq!(frame.y[0], 16);
    assert_eq!(frame.u[0], 128);
    assert_eq!(frame.v[0], 200);
    assert!(d.has_prev());
}

#[test]
fn blank_yuv_to_rgba_is_opaque_black() {
    let frame = BinkYuvFrame::blank(4, 4, false).unwrap();
    let rgba = frame.to_rgba8();
    assert_eq!(rgba.len(), 4 * 4 * 4);
    assert_eq!(&rgba[0..4], &[0, 0, 0, 255]);
}

#[test]
fn biki_uses_mpeg_range_so_y16_is_black() {
    assert_eq!(BinkVersion::BikI.color_range(), BinkColorRange::Mpeg);
    assert_eq!(BinkVersion::BikK.color_range(), BinkColorRange::Jpeg);
    let mut frame = BinkYuvFrame::blank_with_range(2, 2, false, BinkColorRange::Mpeg).unwrap();
    frame.y.fill(16);
    frame.u.fill(128);
    frame.v.fill(128);
    let rgba = frame.to_rgba8();
    assert_eq!(&rgba[0..4], &[0, 0, 0, 255]);
}

#[test]
fn mpeg_y16_as_jpeg_would_lift_black() {
    // 误用 full 色域解 studio 样本时，Y16 会抬成深灰——即主菜单 CRT 暗部发灰。
    let grey = yuv420_planes_to_rgba8(2, 2, &[16, 16, 16, 16], &[128], &[128], None, BinkColorRange::Jpeg);
    assert!(grey[0] > 8, "jpeg path leaves Y16 as lifted grey, got {}", grey[0]);
}
