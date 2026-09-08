//! 集成测试：原 `src/vxl.rs` 内联测试迁出。

use ra_assets::*;

const MAGIC: &[u8; 16] = b"Voxel Animation\0";
const SECTION_TAILER: usize = 92;

fn minimal_vxl() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(MAGIC);
    data.extend_from_slice(&1u32.to_le_bytes()); // palette_count
    data.extend_from_slice(&1u32.to_le_bytes()); // limb_count
    data.extend_from_slice(&1u32.to_le_bytes()); // tailer_count
    let body_size_at = data.len();
    data.extend_from_slice(&0u32.to_le_bytes()); // body_size patch later
    data.push(0);
    data.extend_from_slice(&[128u8; 768]);
    data.push(0);
    // section header
    data.extend_from_slice(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    let body_start = data.len();
    // 2x2 columns: one voxel in col0
    let span_start_rel = 0u32;
    data.extend_from_slice(&0i32.to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    let span_end_rel = (data.len() - body_start) as u32;
    data.extend_from_slice(&[0u8; 16]);
    let data_span_rel = (data.len() - body_start) as u32;
    // column 0: z_skip=0, count=1, color=5, normal=1, dup=0
    data.extend_from_slice(&[0, 1, 5, 1, 0]);

    let body_size = (data.len() - body_start) as u32;
    data[body_size_at..body_size_at + 4].copy_from_slice(&body_size.to_le_bytes());

    let mut tail = vec![0u8; SECTION_TAILER];
    tail[0..4].copy_from_slice(&span_start_rel.to_le_bytes());
    tail[4..8].copy_from_slice(&span_end_rel.to_le_bytes());
    tail[8..12].copy_from_slice(&data_span_rel.to_le_bytes());
    tail[12..16].copy_from_slice(&1.0f32.to_le_bytes());
    for (i, diag) in [(0, 1.0f32), (5, 1.0), (10, 1.0)] {
        tail[16 + i * 4..20 + i * 4].copy_from_slice(&diag.to_le_bytes());
    }
    for (i, v) in [0.0f32, 0.0, 0.0, 2.0, 2.0, 4.0].into_iter().enumerate() {
        tail[64 + i * 4..68 + i * 4].copy_from_slice(&v.to_le_bytes());
    }
    tail[88] = 2;
    tail[89] = 2;
    tail[90] = 4;
    tail[91] = 4;
    data.extend_from_slice(&tail);
    data
}

#[test]
fn parse_minimal_one_voxel() {
    let vxl = VxlFile::parse(&minimal_vxl()).unwrap();
    assert_eq!(vxl.limb_count, 1);
    assert_eq!(vxl.limbs[0].name, "body");
    assert_eq!(vxl.limbs[0].size_x, 2);
    assert_eq!(vxl.limbs[0].transform[0], 1.0);
    assert_eq!(vxl.limbs[0].bounds[3], 2.0);
    assert_eq!(vxl.total_voxels(), 1);
    assert_eq!(vxl.limbs[0].voxels[0].color_index, 5);
}

#[test]
fn reject_bad_magic() {
    assert!(VxlFile::parse(b"not a vxl file!!!!!!!!!!!").is_err());
}
