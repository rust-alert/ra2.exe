use std::collections::HashMap;

use ra_map::{
    MapInfo, TerrainImage, TerrainObject, paint_map_terrain_objects, terrain_anim_frame, terrain_animation_rate_ms,
};
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
    multi_frame_shp(&[index])
}

fn multi_frame_shp(indices: &[u8]) -> Vec<u8> {
    let frame_count = indices.len() as u16;
    let header_size = 8 + frame_count as usize * 24;
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&frame_count.to_le_bytes());
    for i in 0..indices.len() {
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&0u32.to_le_bytes());
        let offset = (header_size + i) as u32;
        data.extend_from_slice(&offset.to_le_bytes());
    }
    data.extend_from_slice(indices);
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

fn first_opaque(image: &TerrainImage) -> [u8; 4] {
    let px = image.image.as_raw();
    let c = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted pixel");
    [c[0], c[1], c[2], c[3]]
}

#[test]
fn empty_terrain_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    assert_eq!(paint_map_terrain_objects(&EmptySource, &map, &mut image, "art.ini", "rules.ini", 0), 0);
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini", 0), 1);
    let green = first_opaque(&image);
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini", 0), 1);
    let red = first_opaque(&image);
    assert!(red[0] > red[1] && red[0] > red[2], "expected unittem red fallback, got {red:?}");
}

#[test]
fn spawns_tiberium_still_uses_isometric_theater_palette() {
    // `SpawnsTiberium` 是玩法属性，不改变 `Theater=yes` 地形 SHP 的调色板族。
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nSpawnsTiberium=yes\n".to_vec());
    files.insert("temperat.pal".into(), solid_index_pal(5, 63, 50, 0));
    files.insert("isotem.pal".into(), solid_index_pal(5, 40, 40, 40));
    files.insert("tibtre01.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = tibtre_map();
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini", 0), 1);
    let grey = first_opaque(&image);
    assert_eq!(grey[0], grey[1], "expected isotem grey, got {grey:?}");
    assert_eq!(grey[1], grey[2], "expected isotem grey, got {grey:?}");
}

#[test]
fn terrain_object_centers_on_iso_diamond() {
    // 60×60 画布、子帧在 (0,0) 的 1×1 → offset = (0−30+30, 0−30+15−3) = (0, −18)。
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini", 0), 1);
    let (sx, sy) = ra_map::iso_to_screen(5, 0, 0);
    let expect_x = (sx - image.origin_x) as u32;
    let expect_y = (sy - 18 - image.origin_y) as u32;
    let w = image.image.width();
    let px = image.image.as_raw();
    let di = ((expect_y * w + expect_x) * 4) as usize;
    assert!(px[di + 3] > 0, "expected pixel at diamond-centered ({expect_x},{expect_y})");
}

#[test]
fn animation_rate_converts_logic_frames_to_ms() {
    // `AnimationRate=3` → 3/15 秒 = 200ms/帧。
    assert_eq!(terrain_animation_rate_ms(3), 200);
    assert_eq!(terrain_animation_rate_ms(0), 66);
    assert_eq!(terrain_anim_frame(0, 3, 22), 0);
    assert_eq!(terrain_anim_frame(199, 3, 22), 0);
    assert_eq!(terrain_anim_frame(200, 3, 22), 1);
    assert_eq!(terrain_anim_frame(2200, 3, 22), 11);
    assert_eq!(terrain_anim_frame(4400, 3, 22), 0);
}

#[test]
fn animated_terrain_selects_body_frame_by_clock() {
    // 帧 0 索引 5→绿；帧 1 索引 6→红。`AnimationRate=3` 时 200ms 切到第 1 帧。
    let mut pal = solid_index_pal(5, 0, 63, 0);
    pal[6 * 3] = 63;
    pal[6 * 3 + 1] = 0;
    pal[6 * 3 + 2] = 0;

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert(
        "rules.ini".into(),
        b"[TIBTRE01]\nIsAnimated=yes\nAnimationRate=3\nSpawnsTiberium=yes\n".to_vec(),
    );
    files.insert("isotem.pal".into(), pal);
    files.insert("tibtre01.tem".into(), multi_frame_shp(&[5, 6]));
    let source = MapSource { files };
    let map = tibtre_map();

    let mut image0 = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image0, "art.ini", "rules.ini", 0), 1);
    let green = first_opaque(&image0);
    assert!(green[1] > green[0], "clock 0 should paint frame 0 green, got {green:?}");

    let mut image1 = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image1, "art.ini", "rules.ini", 200), 1);
    let red = first_opaque(&image1);
    assert!(red[0] > red[1], "clock 200ms should paint frame 1 red, got {red:?}");
}

#[test]
fn static_terrain_ignores_anim_clock() {
    let mut pal = solid_index_pal(5, 0, 63, 0);
    pal[6 * 3] = 63;
    pal[6 * 3 + 1] = 0;
    pal[6 * 3 + 2] = 0;

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TREE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TREE01]\nIsAnimated=no\nAnimationRate=3\n".to_vec());
    files.insert("isotem.pal".into(), pal);
    files.insert("tree01.tem".into(), multi_frame_shp(&[5, 6]));
    let source = MapSource { files };
    let map = tree_map();
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, "art.ini", "rules.ini", 200), 1);
    let green = first_opaque(&image);
    assert!(green[1] > green[0], "static terrain must stay on frame 0, got {green:?}");
}
