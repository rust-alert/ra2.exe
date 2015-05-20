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

#[test]
fn empty_terrain_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    assert_eq!(paint_map_terrain_objects(&EmptySource, &map, &mut image, "art.ini"), 0);
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini"), 1);
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini"), 1);
    let px = image.image.as_raw();
    let red = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted pixel");
    assert!(red[0] > red[1] && red[0] > red[2], "expected unittem red fallback, got {red:?}");
}
