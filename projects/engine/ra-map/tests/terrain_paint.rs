use std::collections::HashMap;

use ra_map::{MapInfo, TerrainImage, TerrainObject, paint_map_terrain_objects};
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

fn tree_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    // 选正屏幕坐标格，避免 iso 原点附近被画布裁掉。
    map.terrain_objects = vec![TerrainObject { x: 5, y: 0, name: "TREE01".into() }];
    map
}

fn tibtre_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.terrain_objects = vec![TerrainObject { x: 5, y: 0, name: "TIBTRE01".into() }];
    map
}

#[test]
fn empty_terrain_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    assert_eq!(paint_map_terrain_objects(&EmptySource, &map, &mut image, "art.ini", "rules.ini"), 0);
}

#[test]
fn prefers_theater_palette_over_unittem() {
    // 剧院 pal 索引 5 = 满绿；单位 pal 索引 5 = 满红。应用剧院色则像素为绿。
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TREE01]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("tree01.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = tree_map();
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini"), 1);
    let px = image.image.as_raw();
    let green = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted pixel");
    assert!(green[1] > green[0] && green[1] > green[2], "expected theater green, got {green:?}");
}

#[test]
fn falls_back_to_unittem_when_theater_palette_missing() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TREE01]\nTheater=yes\n".to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("tree01.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = tree_map();
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini"), 1);
    let px = image.image.as_raw();
    let red = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted pixel");
    assert!(red[0] > red[1] && red[0] > red[2], "expected unittem red fallback, got {red:?}");
}

#[test]
fn spawns_tiberium_uses_temperat_not_isotem() {
    // 矿柱 SpawnsTiberium：temperat 索引 5 = 金黄；isotem 同索引 = 灰。须用金黄。
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nSpawnsTiberium=yes\n".to_vec());
    files.insert("temperat.pal".into(), solid_index_pal(5, 63, 50, 0));
    files.insert("isotem.pal".into(), solid_index_pal(5, 40, 40, 40));
    files.insert("tibtre01.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = tibtre_map();
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini"), 1);
    let px = image.image.as_raw();
    let gold = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted pixel");
    assert!(gold[0] > gold[2] && gold[1] > gold[2], "expected temperat gold, got {gold:?}");
    assert!(gold[0] > 180, "must not use washed isotem grey, got {gold:?}");
}

#[test]
fn terrain_object_centers_on_iso_diamond() {
    // 60×60 画布、子帧在 (0,0) 的 1×1 → 相对 iso 原点偏移应为 (+30, +12)（中心 −3 Y）。
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&60u16.to_le_bytes());
    data.extend_from_slice(&60u16.to_le_bytes());
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
    data.push(5);

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TREE01]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("tree01.tem".into(), data);
    let source = MapSource { files };
    let map = tree_map();
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini"), 1);
    let (sx, sy) = ra_map::iso_to_screen(5, 0, 0);
    let expect_x = (sx + 30 - image.origin_x) as u32;
    let expect_y = (sy + 12 - image.origin_y) as u32;
    let w = image.image.width();
    let px = image.image.as_raw();
    let di = ((expect_y * w + expect_x) * 4) as usize;
    assert!(px[di + 3] > 0, "expected pixel at diamond-centered ({expect_x},{expect_y})");
}
