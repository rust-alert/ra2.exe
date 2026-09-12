//! 自顶层 `terrain_paint.rs`。

use std::collections::HashMap;

use ra_map::{ArtRules,
             MapInfo, TerrainImage, TerrainObject, TerrainPaintMode, collect_ore_tree_anim_bank, collect_terrain_anim_bank,
             format_terrain_anim_layer_diag, ore_tree_frame_count_hints, paint_map_terrain_objects, paint_ore_tree_frames, paint_terrain_anim_bank,
             terrain_anim_frame, terrain_animation_rate_ms,
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

fn clock(ms: u64) -> TerrainPaintMode {
    TerrainPaintMode::AllWithClock { anim_clock_ms: ms }
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
    assert_eq!(paint_map_terrain_objects(&EmptySource, &map, &mut image, &ArtRules::default(), clock(0)), 0);
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(0)), 1);
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(0)), 1);
    let red = first_opaque(&image);
    assert!(red[0] > red[1] && red[0] > red[2], "expected unittem red fallback, got {red:?}");
}

#[test]
fn spawns_tiberium_uses_unittem_palette() {
    // `SpawnsTiberium=yes` 矿柱使用单位调色板，不是等距剧院色板。
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nSpawnsTiberium=yes\n".to_vec());
    files.insert("temperat.pal".into(), solid_index_pal(5, 63, 50, 0));
    files.insert("isotem.pal".into(), solid_index_pal(5, 40, 40, 40));
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("tibtre01.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = tibtre_map();
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(0)), 1);
    let red = first_opaque(&image);
    assert!(red[0] > red[1] && red[0] > red[2], "expected unittem red, got {red:?}");
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(0)), 1);
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
fn looping_animated_terrain_selects_body_frame_by_clock() {
    // 非产矿旗帜类：帧 0 绿、帧 1 红。`AnimationRate=3` 时 200ms 切到第 1 帧。
    let mut pal = solid_index_pal(5, 0, 63, 0);
    pal[6 * 3] = 63;
    pal[6 * 3 + 1] = 0;
    pal[6 * 3 + 2] = 0;

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[FLAG01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[FLAG01]\nIsAnimated=yes\nAnimationRate=3\n".to_vec());
    files.insert("isotem.pal".into(), pal);
    files.insert("flag01.tem".into(), multi_frame_shp(&[5, 6]));
    let source = MapSource { files };
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.terrain_objects = vec![TerrainObject { x: 5, y: 0, name: "FLAG01".into() }];

    let mut image0 = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image0, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(0)), 1);
    let green = first_opaque(&image0);
    assert!(green[1] > green[0], "clock 0 should paint frame 0 green, got {green:?}");

    let mut image1 = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image1, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(200)), 1);
    let red = first_opaque(&image1);
    assert!(red[0] > red[1], "clock 200ms should paint frame 1 red, got {red:?}");
}

#[test]
fn spawns_tiberium_stays_on_idle_frame_zero() {
    // 矿柱是条件动画：时钟不循环；StaticOnly 底图不画，由 ore-tree 银行叠 Idle 0。
    let mut pal = solid_index_pal(5, 0, 63, 0);
    pal[6 * 3] = 63;
    pal[6 * 3 + 1] = 0;
    pal[6 * 3 + 2] = 0;

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nIsAnimated=yes\nAnimationRate=3\nSpawnsTiberium=yes\nAnimationProbability=.5\n".to_vec());
    files.insert("unittem.pal".into(), pal);
    files.insert("tibtre01.tem".into(), multi_frame_shp(&[5, 6]));
    let source = MapSource { files };
    let map = tibtre_map();

    assert!(collect_terrain_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini")).is_empty());
    let bank = collect_ore_tree_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini"));
    assert_eq!(bank.layers.len(), 1);
    assert_eq!(bank.layers[0].frames.len(), 2);
    assert_eq!(ore_tree_frame_count_hints(&bank), vec![(5, 0, 2)]);

    let mut image0 = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image0, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(0)), 1);
    let green0 = first_opaque(&image0);
    assert!(green0[1] > green0[0], "idle frame 0 green, got {green0:?}");

    let mut image1 = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image1, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(200)), 1);
    let green1 = first_opaque(&image1);
    assert!(green1[1] > green1[0], "clock must not advance ore-tree idle frame, got {green1:?}");

    let mut image_static = TerrainImage::blank(256, 256);
    assert_eq!(
        paint_map_terrain_objects(&source, &map, &mut image_static, &ArtRules::load(&source, "art.ini", "rules.ini"), TerrainPaintMode::StaticOnly),
        0,
        "StaticOnly must leave ore trees to OreTree bank"
    );

    let mut image_bank = TerrainImage::blank(256, 256);
    assert_eq!(paint_ore_tree_frames(&mut image_bank, &bank, &[(5, 0, 0)]), 1);
    let green_bank = first_opaque(&image_bank);
    assert!(green_bank[1] > green_bank[0], "bank frame 0 green, got {green_bank:?}");

    let mut image_f1 = TerrainImage::blank(256, 256);
    assert_eq!(paint_ore_tree_frames(&mut image_f1, &bank, &[(5, 0, 1)]), 1);
    let red = first_opaque(&image_f1);
    assert!(red[0] > red[1], "bank frame 1 red from state machine, got {red:?}");
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
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(200)), 1);
    let green = first_opaque(&image);
    assert!(green[1] > green[0], "static terrain must stay on frame 0, got {green:?}");
}

#[test]
fn static_only_skips_looping_animated_terrain() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[FLAG01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[FLAG01]\nIsAnimated=yes\nAnimationRate=3\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("flag01.tem".into(), multi_frame_shp(&[5, 6]));
    let source = MapSource { files };
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.terrain_objects = vec![TerrainObject { x: 5, y: 0, name: "FLAG01".into() }];
    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, &ArtRules::load(&source, "art.ini", "rules.ini"), TerrainPaintMode::StaticOnly), 0);
}

#[test]
fn terrain_anim_bank_records_looping_hit_and_paints_by_clock() {
    let mut pal = solid_index_pal(5, 0, 63, 0);
    pal[6 * 3] = 63;
    pal[6 * 3 + 1] = 0;
    pal[6 * 3 + 2] = 0;

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[FLAG01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[FLAG01]\nIsAnimated=yes\nAnimationRate=3\n".to_vec());
    files.insert("isotem.pal".into(), pal);
    files.insert("flag01.tem".into(), multi_frame_shp(&[5, 6]));
    let source = MapSource { files };
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.terrain_objects = vec![TerrainObject { x: 5, y: 0, name: "FLAG01".into() }];

    let bank = collect_terrain_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini"));
    assert_eq!(bank.layers.len(), 1);
    let layer = &bank.layers[0];
    assert_eq!(layer.file, "flag01.tem");
    assert_eq!(layer.canvas_width, 1);
    assert_eq!(layer.canvas_height, 1);
    assert_eq!(layer.frames.len(), 2);
    assert_eq!(layer.shp_frames, 2);
    assert_eq!(layer.palette, "isotem.pal");
    assert_eq!(layer.rate_ms, 200);
    assert_eq!(bank.frame_signature(0), bank.frame_signature(199));
    assert_ne!(bank.frame_signature(0), bank.frame_signature(200));

    let mut image0 = TerrainImage::blank(256, 256);
    assert_eq!(paint_terrain_anim_bank(&mut image0, &bank, 0), 1);
    let green = first_opaque(&image0);
    assert!(green[1] > green[0], "bank clock 0 should paint frame 0 green, got {green:?}");

    let mut image1 = TerrainImage::blank(256, 256);
    assert_eq!(paint_terrain_anim_bank(&mut image1, &bank, 200), 1);
    let red = first_opaque(&image1);
    assert!(red[0] > red[1], "bank clock 200ms should paint frame 1 red, got {red:?}");
}

#[test]
fn looping_animated_terrain_skips_shadow_half_frames() {
    // 4 帧：主体绿/红 + 落影半幅（索引 1）。时钟若误用全 4 帧会在 400ms 画到落影蓝。
    let mut pal = solid_index_pal(5, 0, 63, 0);
    pal[6 * 3] = 63;
    pal[6 * 3 + 1] = 0;
    pal[6 * 3 + 2] = 0;
    pal[1 * 3] = 0;
    pal[1 * 3 + 1] = 0;
    pal[1 * 3 + 2] = 63;

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[FLAG01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[FLAG01]\nIsAnimated=yes\nAnimationRate=3\n".to_vec());
    files.insert("isotem.pal".into(), pal);
    files.insert("flag01.tem".into(), multi_frame_shp(&[5, 6, 1, 1]));
    let source = MapSource { files };
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.terrain_objects = vec![TerrainObject { x: 5, y: 0, name: "FLAG01".into() }];

    let bank = collect_terrain_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini"));
    assert_eq!(bank.layers.len(), 1);
    assert_eq!(bank.layers[0].frames.len(), 2, "body must exclude shadow half");
    assert_eq!(bank.layers[0].shp_frames, 4, "shp_frames must keep full count including shadow half");

    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_map_terrain_objects(&source, &map, &mut image, &ArtRules::load(&source, "art.ini", "rules.ini"), clock(400)), 1);
    let green = first_opaque(&image);
    assert!(green[1] > green[0] && green[1] > green[2], "clock 400ms must wrap body frames to green, not shadow blue, got {green:?}");
}

#[test]
fn spawns_tiberium_blits_full_canvas_with_cell_height_y() {
    // 84×56 画布、子帧 (24,4) 的 1×1。完整画布相对钻石中心 (-42, -28-15-3)。
    // 换算到 iso 原点：offset = (30-42, 15-28-18) = (-12, -31)。
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&84u16.to_le_bytes());
    data.extend_from_slice(&56u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&24u16.to_le_bytes());
    data.extend_from_slice(&4u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&[0, 0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(32u32).to_le_bytes());
    data.push(5);

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nIsAnimated=yes\nSpawnsTiberium=yes\nAnimationProbability=.003\n".to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("tibtre01.tem".into(), data);
    let source = MapSource { files };
    let map = tibtre_map();

    assert!(collect_terrain_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini")).is_empty());
    let bank = collect_ore_tree_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini"));
    assert_eq!(bank.layers.len(), 1);
    assert_eq!(bank.layers[0].shp_frames, 1);
    assert_eq!(bank.layers[0].frames[0].width, 84);
    assert_eq!(bank.layers[0].frames[0].height, 56);

    let mut image = TerrainImage::blank(256, 256);
    assert_eq!(paint_ore_tree_frames(&mut image, &bank, &[(5, 0, 0)]), 1);

    let (sx, sy) = ra_map::iso_to_screen(5, 0, 0);
    // 完整画布左上角 + 子帧 (24,4) → 不透明像素相对 iso 原点：
    // (-12+24, -31+4) = (12, -27)
    let expect_x = (sx + 12 - image.origin_x) as u32;
    let expect_y = (sy - 27 - image.origin_y) as u32;
    let w = image.image.width();
    let px = image.image.as_raw();
    let di = ((expect_y * w + expect_x) * 4) as usize;
    assert!(di + 3 < px.len(), "pixel index in bounds");
    assert!(px[di + 3] > 0, "expected ore-tree pixel at CellHeight+FA2 anchor ({expect_x},{expect_y})");
    assert!(px[di] > px[di + 1], "expected unittem red at ore-tree pixel");
}

#[test]
fn ore_tree_diag_reports_static_skip_and_bank_draw() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nIsAnimated=yes\nAnimationRate=3\nSpawnsTiberium=yes\nAnimationProbability=.5\n".to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("tibtre01.tem".into(), multi_frame_shp(&[5, 1]));
    let source = MapSource { files };
    let map = tibtre_map();

    let mut static_img = TerrainImage::blank(64, 64);
    let static_n = paint_map_terrain_objects(&source, &map, &mut static_img, &ArtRules::load(&source, "art.ini", "rules.ini"), TerrainPaintMode::StaticOnly);
    assert_eq!(static_n, 0);

    let bank = collect_ore_tree_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini"));
    assert_eq!(bank.layers.len(), 1);
    let layer = &bank.layers[0];
    assert_eq!(layer.type_name, "TIBTRE01");
    assert_eq!(layer.palette, "unittem.pal");
    assert!(layer.has_shadow_frames, "index-1 second half must count as shadow frames");
    assert!(layer.shadow_blit_attached, "shadow half must attach to TileBlit");
    assert!(layer.frames[0].shadow.is_some());

    let mut bank_img = TerrainImage::blank(256, 256);
    let bank_n = paint_ore_tree_frames(&mut bank_img, &bank, &[(5, 0, 0)]);
    assert_eq!(bank_n, 1);

    let line = format_terrain_anim_layer_diag(layer, 0, static_n > 0, bank_n > 0);
    assert!(line.contains("type=TIBTRE01"));
    assert!(line.contains("file=tibtre01.tem"));
    assert!(line.contains("palette=unittem.pal"));
    assert!(line.contains("static_layer_drawn=false"));
    assert!(line.contains("anim_bank_drawn=true"));
    assert!(line.contains("has_shadow_frames=true"));
    assert!(line.contains("shadow_blit_attached=true"));
}

#[test]
fn tibtre_stock_fixture_runtime_diag_when_present() {
    // 本机 CLI 解出的零售资源：`pnpm exec ra2 extract … -- tibtre01.tem unittem.pal`
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../tmp/tibtre-diag");
    let tem = dir.join("tibtre01.tem");
    let pal = dir.join("unittem.pal");
    if !tem.is_file() || !pal.is_file() {
        return;
    }

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nIsAnimated=yes\nAnimationRate=3\nSpawnsTiberium=yes\nAnimationProbability=.003\n".to_vec());
    files.insert("tibtre01.tem".into(), std::fs::read(&tem).expect("tibtre01.tem"));
    files.insert("unittem.pal".into(), std::fs::read(&pal).expect("unittem.pal"));
    let source = MapSource { files };
    let map = tibtre_map();

    let mut static_img = TerrainImage::blank(64, 64);
    let static_n = paint_map_terrain_objects(&source, &map, &mut static_img, &ArtRules::load(&source, "art.ini", "rules.ini"), TerrainPaintMode::StaticOnly);
    let bank = collect_ore_tree_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini"));
    assert_eq!(bank.layers.len(), 1);
    let layer = &bank.layers[0];
    let mut bank_img = TerrainImage::blank(256, 256);
    let bank_n = paint_ore_tree_frames(&mut bank_img, &bank, &[(5, 0, 0)]);
    let line = format_terrain_anim_layer_diag(layer, 0, static_n > 0, bank_n > 0);
    eprintln!("ore_tree_runtime_diag {line}");

    assert_eq!(layer.canvas_width, 84);
    assert_eq!(layer.canvas_height, 56);
    assert_eq!(layer.shp_frames, 22);
    assert_eq!(layer.frames.len(), 11);
    assert_eq!(layer.palette, "unittem.pal");
    assert!(layer.has_shadow_frames);
    assert!(layer.shadow_blit_attached);
    assert_eq!(static_n, 0);
    assert_eq!(bank_n, 1);
    assert_eq!(layer.rate_ms, terrain_animation_rate_ms(3));
}

#[test]
fn ore_tree_shadow_darkens_underlay_before_body() {
    // 主体索引 5=红；落影索引 1。落影与主体同格时，先压暗底色再盖主体。
    let mut pal = solid_index_pal(5, 63, 0, 0);
    pal[1 * 3] = 0;
    pal[1 * 3 + 1] = 0;
    pal[1 * 3 + 2] = 0;

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[TIBTRE01]\nTheater=yes\n".to_vec());
    files.insert("rules.ini".into(), b"[TIBTRE01]\nIsAnimated=yes\nSpawnsTiberium=yes\nAnimationProbability=.5\n".to_vec());
    files.insert("unittem.pal".into(), pal);
    files.insert("tibtre01.tem".into(), multi_frame_shp(&[5, 1]));
    let source = MapSource { files };
    let map = tibtre_map();
    let bank = collect_ore_tree_anim_bank(&source, &map, &ArtRules::load(&source, "art.ini", "rules.ini"));
    assert!(bank.layers[0].frames[0].shadow.is_some());

    let mut image = TerrainImage::blank(256, 256);
    // 先铺满亮灰底，便于观察落影折半。
    for px in image.image.pixels_mut() {
        *px = image::Rgba([200, 200, 200, 255]);
    }
    assert_eq!(paint_ore_tree_frames(&mut image, &bank, &[(5, 0, 0)]), 1);

    let shadow = bank.layers[0].frames[0].shadow.as_ref().unwrap();
    let (sx, sy) = ra_map::iso_to_screen(5, 0, 0);
    let ox = sx + shadow.offset_x - image.origin_x;
    let oy = sy + shadow.offset_y - image.origin_y;
    let mut dimmed = 0usize;
    for row in 0..shadow.height as i32 {
        for col in 0..shadow.width as i32 {
            let mi = (row as u32 * shadow.width + col as u32) as usize;
            if shadow.mask.get(mi).copied().unwrap_or(0) == 0 {
                continue;
            }
            let x = ox + col;
            let y = oy + row;
            if x < 0 || y < 0 {
                continue;
            }
            let px = image.image.get_pixel(x as u32, y as u32).0;
            // 落影处：底色被折半（约 100）或被主体红盖住。
            if px[0] < 200 || px[1] < 200 || px[2] < 200 {
                dimmed += 1;
            }
        }
    }
    assert!(dimmed > 0, "expected shadow to darken or be covered by body");
}
