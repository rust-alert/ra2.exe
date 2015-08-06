use std::collections::HashMap;

use ra_map::{MapInfo, OverlayCell, TerrainImage, flat_tiberium_display_type_name, paint_map_overlays};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

struct MapSource {
    files: HashMap<String, Vec<u8>>,
}

impl AssetSource for MapSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files.get(&relative.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

fn raw_one_pixel_shp(index: u8) -> Vec<u8> {
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
    data.extend_from_slice(&(32u32).to_le_bytes());
    data.push(index);
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

fn overlay_map(id: u8, data: u8) -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.overlays = vec![OverlayCell { x: 5, y: 0, overlay_id: id, data }];
    map
}

#[test]
fn empty_overlays_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    assert_eq!(paint_map_overlays(&EmptySource, &map, &mut image, "art.ini", &|_| None, &|_| false, &|_| None), (0, 0));
}

#[test]
fn theater_overlay_uses_theater_palette() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[LOBRDG26]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("temperat.pal".into(), solid_index_pal(5, 0, 0, 63));
    files.insert("lobrdg26.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = overlay_map(102, 0);
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|id| (id == 102).then(|| "LOBRDG26".into()),
        &|_| false,
        &|_| None,
    );
    assert_eq!((shp, mark), (1, 0));
    let px = image.image.as_raw();
    let green = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted");
    assert!(green[1] > green[0] && green[1] > green[2], "expected theater green, got {green:?}");
}

#[test]
fn tiberium_overlay_uses_temperat_palette() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIB01]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("temperat.pal".into(), solid_index_pal(5, 0, 0, 63));
    files.insert("tib01.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = overlay_map(102, 0);
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|id| (id == 102).then(|| "TIB01".into()),
        &|id| id == 102,
        &|_| None,
    );
    assert_eq!((shp, mark), (1, 0));
    let px = image.image.as_raw();
    let blue = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted");
    assert!(blue[2] > blue[0] && blue[2] > blue[1], "expected temperat blue for ore, got {blue:?}");
}

fn two_frame_shp_frame0_empty_frame1_drawable() -> Vec<u8> {
    // SHP(TS)：8 字节头 + 每帧 24 字节帧头；帧 0 空，帧 1 一像素。
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    // frame 0 empty (24 bytes)
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    // frame 1 drawable (24 bytes)
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(56u32).to_le_bytes()); // 8 + 24*2
    data.push(5);
    data
}

#[test]
fn empty_footprint_skips_when_same_image_anchor_neighbor_draws() {
    // 同图两格：data=0 空帧 footprint，data=1 锚点可画 → 只画锚点。
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[BRIDGE1]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("bridge1.tem".into(), two_frame_shp_frame0_empty_frame1_drawable());
    let source = MapSource { files };
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.overlays = vec![
        OverlayCell { x: 5, y: 0, overlay_id: 1, data: 0 },
        OverlayCell { x: 6, y: 0, overlay_id: 1, data: 1 },
    ];
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|id| (id == 1).then(|| "BRIDGE1".into()),
        &|_| false,
        &|_| None,
    );
    assert_eq!((shp, mark), (1, 0), "footprint must not double-draw");
}

#[test]
fn empty_frame_falls_back_without_same_image_anchor_neighbor() {
    // 断桥端头：仅 data=0 空帧，无同图锚点邻格 → 回退首个可画帧。
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[LOBRDG10]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("lobrdg10.tem".into(), two_frame_shp_frame0_empty_frame1_drawable());
    let source = MapSource { files };
    let map = overlay_map(1, 0);
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|id| (id == 1).then(|| "LOBRDG10".into()),
        &|_| false,
        &|_| None,
    );
    assert_eq!((shp, mark), (1, 0), "stub without anchor must fall back to drawable frame");
}

#[test]
fn empty_preferred_does_not_paint_markers() {
    // 首选空且 SHP 全空：不画、不回退色块。
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    // one empty frame (24 bytes)
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[BRIDGE1]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("bridge1.tem".into(), data);
    let source = MapSource { files };
    let map = overlay_map(1, 0);
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|id| (id == 1).then(|| "BRIDGE1".into()),
        &|_| false,
        &|_| None,
    );
    assert_eq!(shp, 0);
    assert_eq!(mark, 0, "empty frame must not fall back to color markers");
}

#[test]
fn flat_tiberium_display_picks_variant_from_cell_xy() {
    // (7,1): 7*1 % 12 = 7 → TIB08 / GEM08
    assert_eq!(flat_tiberium_display_type_name("TIB01", 7, 1), "TIB08");
    assert_eq!(flat_tiberium_display_type_name("GEM03", 7, 1), "GEM08");
    assert_eq!(flat_tiberium_display_type_name("TIB2_01", 7, 1), "TIB2_08");
    // (5,0): 乘积为 0 → 仍为族首
    assert_eq!(flat_tiberium_display_type_name("TIB01", 5, 0), "TIB01");
}

#[test]
fn tiberium_paint_loads_coordinate_display_shp() {
    // pack 身份仍是 TIB01，但 (7,1) 应画 TIB08；仅提供 tib08.tem 证伪。
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIB01]\nTheater=yes\n".to_vec());
    files.insert("temperat.pal".into(), solid_index_pal(5, 0, 0, 63));
    files.insert("tib08.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.overlays = vec![OverlayCell { x: 7, y: 1, overlay_id: 102, data: 0 }];
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|id| (id == 102).then(|| "TIB01".into()),
        &|id| id == 102,
        &|_| None,
    );
    assert_eq!((shp, mark), (1, 0), "must paint TIB08 from flat display remap");
}

#[test]
fn new_theater_wall_uses_unittem_palette() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[NAWALL]\nNewTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("ntwall.shp".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = overlay_map(27, 0);
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|id| (id == 27).then(|| "NAWALL".into()),
        &|_| false,
        &|_| None,
    );
    assert_eq!((shp, mark), (1, 0));
    let px = image.image.as_raw();
    let red = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted");
    assert!(red[0] > red[1] && red[0] > red[2], "expected unittem red for wall, got {red:?}");
}
