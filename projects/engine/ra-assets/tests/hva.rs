//! 集成测试：原 `src/hva.rs` 内联测试迁出。

use ra_assets::*;

fn identity_row_major() -> [f32; 12] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0,
    ]
}

fn sample_hva() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"test.hva\0\0\0\0\0\0\0\0");
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
    for f in identity_row_major() {
        data.extend_from_slice(&f.to_le_bytes());
    }
    // frame 1: translate (1,2,3)
    let mut m = identity_row_major();
    m[3] = 1.0;
    m[7] = 2.0;
    m[11] = 3.0;
    for f in m {
        data.extend_from_slice(&f.to_le_bytes());
    }
    data
}

#[test]
fn parse_two_frames() {
    let hva = HvaFile::parse(&sample_hva()).unwrap();
    assert_eq!(hva.frame_count, 2);
    assert_eq!(hva.section_count, 1);
    assert_eq!(hva.section_names[0], "body");
    let t0 = hva.get_transform(0, 0).unwrap();
    assert_eq!(t0[0], 1.0);
    let t1 = hva.get_transform(1, 0).unwrap();
    assert_eq!(t1[3], 1.0);
    assert_eq!(t1[7], 2.0);
    assert_eq!(t1[11], 3.0);
}

#[test]
fn reject_too_small() {
    assert!(HvaFile::parse(&[0u8; 8]).is_err());
}
