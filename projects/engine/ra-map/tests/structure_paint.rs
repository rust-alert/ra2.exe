use std::collections::HashMap;

use ra_map::{
    MapEntity, MapEntityKind, MapInfo, StructureAnimMode, TerrainImage, buildup_frame_index, paint_map_structures,
    structure_anim_frame,
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

#[test]
fn empty_structures_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    assert_eq!(
        paint_map_structures(&EmptySource, &map, &mut image, "art.ini", "rules.ini", &|p, _| p.clone(), StructureAnimMode::BodyOnly),
        0
    );
}

#[test]
fn structure_anim_frame_loops_by_rate() {
    assert_eq!(structure_anim_frame(0, 300, 0, 16), 0);
    assert_eq!(structure_anim_frame(299, 300, 0, 16), 0);
    assert_eq!(structure_anim_frame(300, 300, 0, 16), 1);
    assert_eq!(structure_anim_frame(300 * 16, 300, 0, 16), 0);
    assert_eq!(structure_anim_frame(1000, 220, 33, 64), 33 + ((1000 / 220) % 31) as u16);
}

#[test]
fn oil_derrick_paints_active_anim_two_flag() {
    let art = b"\
[CAOILD]\n\
Remapable=no\n\
NewTheater=yes\n\
ActiveAnim=CAOILD_A\n\
ActiveAnimTwo=CAOILD_F\n\
ActiveAnimTwoZAdjust=-50\n\
\n\
[CAOILD_A]\n\
Image=CAOILD_A\n\
NewTheater=yes\n\
Start=0\n\
LoopStart=0\n\
LoopEnd=1\n\
Rate=220\n\
\n\
[CAOILD_F]\n\
Image=CAOILD_F\n\
NewTheater=yes\n\
Start=0\n\
LoopStart=0\n\
LoopEnd=2\n\
Rate=300\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 63, 0));
    files.insert("ctoild.shp".into(), raw_one_pixel_shp(5));
    files.insert("ctoild_a.shp".into(), raw_one_pixel_shp(5));
    files.insert("ctoild_f.shp".into(), multi_frame_shp(&[5, 5]));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "CAOILD".into(),
        health: 256,
        x: 5,
        y: 0,
        facing: 0,
        sub_cell: 0,
    });

    let source = MapSource { files };
    let mut image = TerrainImage::blank(256, 256);
    let painted = paint_map_structures(
        &source,
        &map,
        &mut image,
        "art.ini",
        "rules.ini",
        &|p, _| p.clone(),
        StructureAnimMode::BodyAndAnims { clock_ms: 300 },
    );
    assert_eq!(painted, 3, "body + pump ActiveAnim + flag ActiveAnimTwo");
}

#[test]
fn yellow_health_collects_damage_fire_layers() {
    use ra_map::collect_structure_anim_bank;

    let art = b"\
[CAGAS01]\n\
Remapable=no\n\
DamageFireOffset0=10,-5\n\
\n\
[FIRE01]\n\
Rate=50\n\
";
    let rules = b"\
[General]\n\
DamageFireTypes=FIRE01\n\
[AudioVisual]\n\
ConditionYellow=50%\n\
ConditionRed=25%\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("rules.ini".into(), rules.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("cagas01.shp".into(), raw_one_pixel_shp(5));
    files.insert("fire01.shp".into(), multi_frame_shp(&[5, 5]));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "CAGAS01".into(),
        health: 64,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
    });
    let source = MapSource { files };
    let bank = collect_structure_anim_bank(&source, &map, "art.ini", "rules.ini", &|p, _| p.clone());
    assert_eq!(bank.layers.len(), 1, "yellow HP should bake one fire layer");
    assert_eq!(bank.layers[0].frames.len(), 2);
    // frame_to_blit 锚点 (+TILE_W/2, -H/2) 再加 DamageFireOffset。
    assert_eq!(bank.layers[0].frames[0].offset_x, 30 + 10);
    assert_eq!(bank.layers[0].frames[0].offset_y, -5);
}

#[test]
fn yellow_health_uses_active_anim_damaged() {
    use ra_map::collect_structure_anim_bank;

    let art = b"\
[GATECH]\n\
Remapable=no\n\
ActiveAnim=GATECH_A\n\
ActiveAnimDamaged=GATECH_AD\n\
DamageFireOffset0=1,1\n\
\n\
[GATECH_A]\n\
Image=GATECH_A\n\
LoopStart=0\n\
LoopEnd=2\n\
Rate=200\n\
\n\
[GATECH_AD]\n\
Image=GATECH_AD\n\
LoopStart=0\n\
LoopEnd=3\n\
Rate=200\n\
\n\
[FIRE01]\n\
Rate=50\n\
";
    let rules = b"\
[General]\n\
DamageFireTypes=FIRE01\n\
[AudioVisual]\n\
ConditionYellow=50%\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("rules.ini".into(), rules.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("gatech.shp".into(), raw_one_pixel_shp(5));
    files.insert("gatech_a.shp".into(), multi_frame_shp(&[5, 5]));
    files.insert("gatech_ad.shp".into(), multi_frame_shp(&[5, 5, 5]));
    files.insert("fire01.shp".into(), multi_frame_shp(&[5, 5]));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GATECH".into(),
        health: 64,
        x: 3,
        y: 3,
        facing: 0,
        sub_cell: 0,
    });
    let source = MapSource { files };
    let bank = collect_structure_anim_bank(&source, &map, "art.ini", "rules.ini", &|p, _| p.clone());
    // 受损活动层 3 帧 + 火焰层。
    assert!(bank.layers.len() >= 2, "expected damaged anim + fire, got {}", bank.layers.len());
    let damaged = bank.layers.iter().find(|l| l.frames.len() == 3).expect("ActiveAnimDamaged 3 frames");
    assert_eq!(damaged.loop_end - damaged.loop_start, 3);
}

#[test]
fn fire_offset_reads_type_section_when_image_redirects() {
    use ra_map::collect_structure_anim_bank;

    let art = b"\
[CAGAS01]\n\
Image=CAGAS_SHARED\n\
DamageFireOffset0=7,-3\n\
\n\
[CAGAS_SHARED]\n\
Remapable=no\n\
\n\
[FIRE01]\n\
Rate=50\n\
";
    let rules = b"\
[General]\n\
DamageFireTypes=FIRE01\n\
[AudioVisual]\n\
ConditionYellow=50%\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("rules.ini".into(), rules.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("cagas_shared.shp".into(), raw_one_pixel_shp(5));
    files.insert("fire01.shp".into(), multi_frame_shp(&[5]));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "CAGAS01".into(),
        health: 64,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
    });
    let source = MapSource { files };
    let bank = collect_structure_anim_bank(&source, &map, "art.ini", "rules.ini", &|p, _| p.clone());
    assert_eq!(bank.layers.len(), 1);
    assert_eq!(bank.layers[0].frames[0].offset_x, 30 + 7);
    assert_eq!(bank.layers[0].frames[0].offset_y, -3);
}

#[test]
fn damage_fire_uses_anim_palette_not_unittem() {
    use ra_map::collect_structure_anim_bank;

    let art = b"\
[CAGAS01]\n\
DamageFireOffset0=0,0\n\
\n\
[FIRE01]\n\
Rate=50\n\
";
    let rules = b"\
[General]\n\
DamageFireTypes=FIRE01\n\
[AudioVisual]\n\
ConditionYellow=50%\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("rules.ini".into(), rules.to_vec());
    // unittem：索引 5 = 青绿；anim：索引 5 = 纯红。火焰像素应取 anim。
    files.insert("unittem.pal".into(), solid_index_pal(5, 0, 63, 63));
    files.insert("anim.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("cagas01.shp".into(), raw_one_pixel_shp(5));
    files.insert("fire01.shp".into(), raw_one_pixel_shp(5));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "CAGAS01".into(),
        health: 64,
        x: 0,
        y: 0,
        facing: 0,
        sub_cell: 0,
    });
    let source = MapSource { files };
    let bank = collect_structure_anim_bank(&source, &map, "art.ini", "rules.ini", &|p, _| p.clone());
    assert_eq!(bank.layers.len(), 1);
    let px = &bank.layers[0].frames[0].rgba;
    assert!(px.len() >= 4, "expected at least one RGBA pixel");
    // VGA 6-bit 63 → 8-bit 扩展后接近 255；只断言红通道远高于绿/蓝。
    assert!(px[0] > 200 && px[1] < 40 && px[2] < 40, "fire must use anim.pal red, got {:?}", &px[0..4]);
}

#[test]
fn full_health_skips_damage_fire_layers() {
    use ra_map::collect_structure_anim_bank;

    let art = b"\
[CAGAS01]\n\
DamageFireOffset0=10,-5\n\
";
    let rules = b"\
[General]\n\
DamageFireTypes=FIRE01\n\
[AudioVisual]\n\
ConditionYellow=50%\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("rules.ini".into(), rules.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("cagas01.shp".into(), raw_one_pixel_shp(5));
    files.insert("fire01.shp".into(), multi_frame_shp(&[5, 5]));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "CAGAS01".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
    });
    let source = MapSource { files };
    let bank = collect_structure_anim_bank(&source, &map, "art.ini", "rules.ini", &|p, _| p.clone());
    assert!(bank.layers.is_empty());
}

#[test]
fn buildup_frame_index_one_shot() {
    assert_eq!(buildup_frame_index(0, 100, 3), Some(0));
    assert_eq!(buildup_frame_index(99, 100, 3), Some(0));
    assert_eq!(buildup_frame_index(100, 100, 3), Some(1));
    assert_eq!(buildup_frame_index(299, 100, 3), Some(2));
    assert_eq!(buildup_frame_index(300, 100, 3), None);
    assert_eq!(buildup_frame_index(0, 100, 0), None);
}

#[test]
fn load_structure_buildup_clip_decodes_frames() {
    use ra_map::load_structure_buildup_clip;

    let art = b"\
[GACNST]\n\
Remapable=yes\n\
NewTheater=yes\n\
Buildup=GACNSTMK\n\
\n\
[GACNSTMK]\n\
Rate=50\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 63, 0));
    // 4 帧：前半主体 + 后半落影（索引 1）；Buildup 只应留下 2 帧主体。
    files.insert(
        "gtcnstmk.shp".into(),
        canvas_frame_shp(
            60,
            30,
            &[(0, 0, 60, 30, 5), (10, 5, 40, 20, 5), (0, 0, 60, 30, 1), (10, 5, 40, 20, 1)],
        ),
    );

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.theater = ra_map::Theater::Temperate;
    let source = MapSource { files };
    let clip = load_structure_buildup_clip(&source, &map, "art.ini", "GACNST", "Americans", 3, 4, &|p, _| p.clone())
        .expect("buildup clip");
    assert_eq!(clip.frames.len(), 2, "shadow half must not enter buildup clip");
    assert_eq!(clip.rate_ms, 50);
    // 相对 iso_to_screen：画布中心 → (+TILE_W/2, 0)，再加 FrameX/Y。
    // 帧0：offset = (0 - 30 + 30, 0 - 15) = (0, -15)
    assert_eq!(clip.frames[0].offset_x, 0);
    assert_eq!(clip.frames[0].offset_y, -15);
    // 帧1：offset = (10 - 30 + 30, 5 - 15) = (10, -10)
    assert_eq!(clip.frames[1].offset_x, 10);
    assert_eq!(clip.frames[1].offset_y, -10);
}

/// 构造带整幅画布尺寸与多帧裁切矩形的 SHP（TS/RA2）。
fn canvas_frame_shp(full_w: u16, full_h: u16, frames: &[(u16, u16, u16, u16, u8)]) -> Vec<u8> {
    let frame_count = frames.len() as u16;
    let header_size = 8 + frame_count as usize * 24;
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&full_w.to_le_bytes());
    data.extend_from_slice(&full_h.to_le_bytes());
    data.extend_from_slice(&frame_count.to_le_bytes());
    let mut payload = Vec::new();
    for (i, &(fx, fy, fw, fh, index)) in frames.iter().enumerate() {
        data.extend_from_slice(&fx.to_le_bytes());
        data.extend_from_slice(&fy.to_le_bytes());
        data.extend_from_slice(&fw.to_le_bytes());
        data.extend_from_slice(&fh.to_le_bytes());
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&0u32.to_le_bytes());
        let offset = (header_size + payload.len()) as u32;
        data.extend_from_slice(&offset.to_le_bytes());
        let _ = i;
        payload.extend(std::iter::repeat(index).take((fw as usize) * (fh as usize)));
    }
    data.extend_from_slice(&payload);
    data
}
