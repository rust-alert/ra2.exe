//! 集成测试：原 `src/image/bink.rs` 内联测试迁出。

use ra_assets::*;

fn synth_biki_header() -> Vec<u8> {
    let mut data = vec![0u8; 0x2C];
    data[0..4].copy_from_slice(&0x694B_4942u32.to_le_bytes()); // BIKi
    data[4..8].copy_from_slice(&(1000u32 - 8).to_le_bytes());
    data[8..12].copy_from_slice(&30u32.to_le_bytes());
    data[12..16].copy_from_slice(&200u32.to_le_bytes());
    data[0x14..0x18].copy_from_slice(&640u32.to_le_bytes());
    data[0x18..0x1C].copy_from_slice(&480u32.to_le_bytes());
    data[0x1C..0x20].copy_from_slice(&15u32.to_le_bytes());
    data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
    data[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
    data[0x28..0x2C].copy_from_slice(&0u32.to_le_bytes());
    data
}

fn synth_biki_with_two_frames() -> Vec<u8> {
    // 文件布局：头 0x2C + 两帧索引 + 两段假包。
    let frame0 = 0x2C + 8; // after index
    let frame1 = frame0 + 16;
    let file_end = frame1 + 32;
    let mut data = vec![0u8; file_end];
    data[0..4].copy_from_slice(&0x694B_4942u32.to_le_bytes());
    data[4..8].copy_from_slice(&((file_end as u32) - 8).to_le_bytes());
    data[8..12].copy_from_slice(&2u32.to_le_bytes());
    data[12..16].copy_from_slice(&32u32.to_le_bytes());
    data[0x14..0x18].copy_from_slice(&16u32.to_le_bytes());
    data[0x18..0x1C].copy_from_slice(&16u32.to_le_bytes());
    data[0x1C..0x20].copy_from_slice(&10u32.to_le_bytes());
    data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
    data[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
    data[0x28..0x2C].copy_from_slice(&0u32.to_le_bytes());
    // index: frame0 keyframe bit cleared in stored offset low bit for i>0
    data[0x2C..0x30].copy_from_slice(&(frame0 as u32).to_le_bytes());
    data[0x30..0x34].copy_from_slice(&((frame1 as u32) | 1).to_le_bytes());
    data
}

#[test]
fn parses_synthetic_biki_header() {
    let h = parse_bink_header(&synth_biki_header()).unwrap();
    assert_eq!(h.version, BinkVersion::BikI);
    assert_eq!(h.file_size, 1000);
    assert_eq!(h.num_frames, 30);
    assert_eq!((h.width, h.height), (640, 480));
    assert!((h.fps() - 15.0).abs() < f64::EPSILON);
    assert_eq!(h.audio_section_offset, 0x2C);
}

#[test]
fn rejects_unknown_signature() {
    let mut data = synth_biki_header();
    data[0..4].copy_from_slice(b"BIKb");
    assert!(parse_bink_header(&data).is_err());
}

#[test]
fn parses_frame_index_for_two_frames() {
    let file = parse_bink_file(&synth_biki_with_two_frames()).unwrap();
    assert_eq!(file.frames.len(), 2);
    assert!(file.frames[0].is_keyframe);
    assert!(file.frames[1].is_keyframe);
    assert_eq!(file.frames[0].size, 16);
    assert_eq!(file.frames[1].size, 32);
    assert!(file.audio_tracks.is_empty());
}

#[test]
fn frame_packet_splits_whole_body_when_no_audio() {
    let data = synth_biki_with_two_frames();
    let file = parse_bink_file(&data).unwrap();
    let p0 = file.frame_packet(&data, 0).unwrap();
    assert!(p0.audio.is_none());
    assert_eq!(p0.video.len(), 16);
    assert_eq!(file.frame_duration_us(), 100_000);
    let p1 = file.frame_packet(&data, 1).unwrap();
    assert_eq!(p1.video.len(), 32);
}

#[test]
fn frame_packet_splits_audio_prefix() {
    // 布局：固定头 + 1 轨描述 + 1 帧索引 + 包(aud_len + audio + video)
    let packet_size = 4 + 4 + 8;
    let tracks_start = 0x2C;
    let index_at = tracks_start + 12;
    let frame0 = index_at + 4;
    let file_end = frame0 + packet_size;
    let mut data = vec![0u8; file_end];
    data[0..4].copy_from_slice(&0x694B_4942u32.to_le_bytes());
    data[4..8].copy_from_slice(&((file_end as u32) - 8).to_le_bytes());
    data[8..12].copy_from_slice(&1u32.to_le_bytes());
    data[12..16].copy_from_slice(&(packet_size as u32).to_le_bytes());
    data[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
    data[0x18..0x1C].copy_from_slice(&8u32.to_le_bytes());
    data[0x1C..0x20].copy_from_slice(&10u32.to_le_bytes());
    data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
    data[0x28..0x2C].copy_from_slice(&1u32.to_le_bytes());
    data[tracks_start..tracks_start + 4].copy_from_slice(&64u32.to_le_bytes());
    data[tracks_start + 4..tracks_start + 6].copy_from_slice(&22_050u16.to_le_bytes());
    data[tracks_start + 6..tracks_start + 8].copy_from_slice(&0x6000u16.to_le_bytes());
    data[tracks_start + 8..tracks_start + 12].copy_from_slice(&1u32.to_le_bytes());
    data[index_at..index_at + 4].copy_from_slice(&(frame0 as u32).to_le_bytes());
    data[frame0..frame0 + 4].copy_from_slice(&4u32.to_le_bytes());
    data[frame0 + 4..frame0 + 8].copy_from_slice(&[9, 9, 9, 9]);
    data[frame0 + 8..frame0 + 16].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

    let file = parse_bink_file(&data).unwrap();
    assert_eq!(file.audio_tracks.len(), 1);
    let pkt = file.frame_packet(&data, 0).unwrap();
    assert_eq!(pkt.audio, Some(&[9, 9, 9, 9][..]));
    assert_eq!(pkt.video, &[1, 2, 3, 4, 5, 6, 7, 8]);
}
