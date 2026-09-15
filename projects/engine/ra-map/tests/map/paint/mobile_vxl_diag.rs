//! 载具 VXL 分图层诊断。

use std::collections::HashMap;

use ra_assets::Palette;
use ra_map::{
    diagnose_mobile_vxl, diagnose_mobile_vxl_sweep_turret, mobile_vxl_diag_facing_sweep_bytes, MOBILE_VXL_TURRET_SUFFIXES,
};
use ra_types::{AssetSource, RaError, RaResult};

struct MapSource {
    files: HashMap<String, Vec<u8>>,
}

impl AssetSource for MapSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files.get(&relative.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

const VXL_MAGIC: &[u8; 16] = b"Voxel Animation\0";
const SECTION_TAILER: usize = 92;

/// 最小单 voxel VXL（2×2×4 节，一格有色）。
fn minimal_vxl_bytes() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(VXL_MAGIC);
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    let body_size_at = data.len();
    data.extend_from_slice(&0u32.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&[128u8; 768]);
    data.push(0);
    data.extend_from_slice(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    let body_start = data.len();
    data.extend_from_slice(&0i32.to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    let span_end_rel = (data.len() - body_start) as u32;
    data.extend_from_slice(&[0u8; 16]);
    let data_span_rel = (data.len() - body_start) as u32;
    data.extend_from_slice(&[0, 1, 5, 1, 0]);

    let body_size = (data.len() - body_start) as u32;
    data[body_size_at..body_size_at + 4].copy_from_slice(&body_size.to_le_bytes());

    let mut tail = vec![0u8; SECTION_TAILER];
    tail[0..4].copy_from_slice(&0u32.to_le_bytes());
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

fn identity_hva_bytes(tx: f32, ty: f32, tz: f32) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"test.hva\0\0\0\0\0\0\0\0");
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
    let m = [
        1.0f32, 0.0, 0.0, tx, //
        0.0, 1.0, 0.0, ty, //
        0.0, 0.0, 1.0, tz,
    ];
    for f in m {
        data.extend_from_slice(&f.to_le_bytes());
    }
    data
}

fn solid_index_pal(index: usize, r6: u8, g6: u8, b6: u8) -> Vec<u8> {
    let mut data = vec![0u8; 768];
    let o = index * 3;
    data[o] = r6;
    data[o + 1] = g6;
    data[o + 2] = b6;
    data
}

fn layer<'a>(report: &'a ra_map::MobileVxlDiagReport, role: &str) -> &'a ra_map::MobileVxlLayerDiag {
    report.layers.iter().find(|l| l.role == role).unwrap_or_else(|| panic!("missing role {role}"))
}

#[test]
fn turret_suffix_candidates_are_stable() {
    assert_eq!(MOBILE_VXL_TURRET_SUFFIXES, &["tur", "barl", "barrel"]);
}

#[test]
fn diagnose_reports_body_turret_barrel_shadow_metrics() {
    let mut files = HashMap::new();
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("mtnk.vxl".into(), minimal_vxl_bytes());
    files.insert("mtnk.hva".into(), identity_hva_bytes(0.0, 0.0, 0.0));
    files.insert("mtnktur.vxl".into(), minimal_vxl_bytes());
    files.insert("mtnktur.hva".into(), identity_hva_bytes(8.0, 0.0, 0.0));
    files.insert("mtnkbarl.vxl".into(), minimal_vxl_bytes());
    files.insert("mtnkbarl.hva".into(), identity_hva_bytes(12.0, 0.0, 0.0));
    let source = MapSource { files };

    let report = diagnose_mobile_vxl(&source, "MTNK", 0, 64, 0);
    assert_eq!(report.stem, "mtnk");
    assert!(layer(&report, "body").vxl_hit);
    assert!(layer(&report, "tur").vxl_hit);
    assert!(layer(&report, "barl").vxl_hit);
    assert!(layer(&report, "composed").width.is_some());
    assert!(layer(&report, "shadow").width.is_some());

    let body = layer(&report, "body");
    assert_eq!(body.facing, Some(0));
    assert_eq!(body.origin_px, body.offset_x.map(|x| -x));
    assert_eq!(body.origin_py, body.offset_y.map(|y| -y));
    assert_eq!(body.cell_offset_x, body.offset_x.map(|x| x + 30));
    assert_eq!(body.cell_offset_y, body.offset_y.map(|y| y + 15));

    let tur = layer(&report, "tur");
    assert_eq!(tur.facing, Some(64));
    // 炮塔 HVA 平移后，独立图层 offset 应与车身不同（挂点不在原点）。
    assert_ne!((tur.offset_x, tur.offset_y), (body.offset_x, body.offset_y));
}

#[test]
fn diagnose_marks_missing_body() {
    let source = MapSource { files: HashMap::new() };
    let report = diagnose_mobile_vxl(&source, "gone", 0, 0, 0);
    assert!(!layer(&report, "body").vxl_hit);
    assert!(report.notes.iter().any(|n| n.contains("缺少车身")));
}

#[test]
fn diagnose_stops_after_first_bar_suffix() {
    let mut files = HashMap::new();
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("tank.vxl".into(), minimal_vxl_bytes());
    files.insert("tanktur.vxl".into(), minimal_vxl_bytes());
    files.insert("tankbarl.vxl".into(), minimal_vxl_bytes());
    files.insert("tankbarrel.vxl".into(), minimal_vxl_bytes());
    let source = MapSource { files };
    let report = diagnose_mobile_vxl(&source, "tank", 64, 64, 0);
    assert!(report.layers.iter().any(|l| l.role == "barl"));
    assert!(!report.layers.iter().any(|l| l.role == "barrel"));
}

#[test]
fn turret_facing_sweep_moves_translated_turret_offsets() {
    // 当前实现：HVA 平移后再按炮塔绝对 facing 旋转 → 挂点随 turret_facing 绕模型原点转。
    let mut files = HashMap::new();
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("htk.vxl".into(), minimal_vxl_bytes());
    files.insert("htktur.vxl".into(), minimal_vxl_bytes());
    files.insert("htktur.hva".into(), identity_hva_bytes(10.0, 0.0, 0.0));
    let source = MapSource { files };

    let reports = diagnose_mobile_vxl_sweep_turret(&source, "htk", 64, 0);
    assert_eq!(reports.len(), mobile_vxl_diag_facing_sweep_bytes().len());
    let origins: Vec<_> = reports
        .iter()
        .map(|r| {
            let tur = layer(r, "tur");
            (tur.offset_x, tur.offset_y)
        })
        .collect();
    let uniq: std::collections::HashSet<_> = origins.iter().copied().collect();
    assert!(
        uniq.len() > 1,
        "translated turret offsets must change across turret facing under absolute-yaw composition: {origins:?}"
    );

    // 车身 facing 固定时，body 行 offset 应保持不变。
    let body_origins: Vec<_> = reports
        .iter()
        .map(|r| {
            let body = layer(r, "body");
            (body.offset_x, body.offset_y)
        })
        .collect();
    assert!(body_origins.windows(2).all(|w| w[0] == w[1]), "body offsets must stay fixed: {body_origins:?}");
}

#[test]
fn palette_parse_smoke_for_diag_fixture() {
    let bytes = solid_index_pal(5, 63, 0, 0);
    let pal = Palette::parse(&bytes).unwrap();
    assert_eq!(pal.colors[5].r, 252);
    assert_eq!(pal.colors[5].a, 255);
}
