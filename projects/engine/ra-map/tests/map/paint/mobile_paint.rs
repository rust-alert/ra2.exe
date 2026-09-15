//! 自顶层 `mobile_paint.rs`。

use ra_map::{MapInfo, MobilePaintPose, PaintDefinitions, TerrainImage, paint_map_mobiles};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_mobiles_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    let mut paint = PaintDefinitions::default();
    assert_eq!(paint_map_mobiles(&EmptySource, &map, &mut image, &mut paint, &|p, _| p.clone(), &|_, _| MobilePaintPose::default()), 0);
}

// 自顶层 `mobile_paint_unit.rs` 并入。

// 自 engine/ra-map/src/mobile_paint.rs :: tests
use ra_map::{ShadowBlit, TileBlit, mobile_paint::*};

#[test]
fn walk_sequence_frame_matches_gi_layout() {
    // Walk=8,6,6 → start 8, count 6, multiplier 6；朝向槽 0 + 步 2 → 帧 10。
    let start = 8u16;
    let mult = 6u16;
    let slot = 0u16;
    let step = 2u16;
    assert_eq!(start + slot * mult + step, 10);
    assert!(infantry_facing_slot(0) < 8);
}

#[test]
fn infantry_facing_slot_follows_move_axis() {
    // `facing_toward`：+X=0 屏幕东南，+Y=64 西南，−X=128 西北，−Y=192 东北。
    // 序列槽从北起逆时针：东南=5、南=4、西南=3、西=2、西北=1、北=0、东北=7、东=6。
    assert_eq!(infantry_facing_slot(0), 5);
    assert_eq!(infantry_facing_slot(32), 4);
    assert_eq!(infantry_facing_slot(64), 3);
    assert_eq!(infantry_facing_slot(96), 2);
    assert_eq!(infantry_facing_slot(128), 1);
    assert_eq!(infantry_facing_slot(160), 0);
    assert_eq!(infantry_facing_slot(192), 7);
    assert_eq!(infantry_facing_slot(224), 6);
}

#[test]
fn parse_walk_triple() {
    assert_eq!(parse_sequence_triple("8,6,6"), Some((8, 6, 6)));
    assert_eq!(parse_sequence_triple("0,1,1"), Some((0, 1, 1)));
}

#[test]
fn fire_sequence_preferred_over_walk() {
    // Fire=52,6,6；facing 0（+X / 东南）→ 槽 5，步 1 → 帧 52+5*6+1=83；即使 moving=true 也优先开火。
    let frame = infantry_shp_frame_from_triples(0, 1, true, true, Some((8, 6, 6)), Some((0, 1, 6)), Some((52, 6, 6)));
    assert_eq!(frame, 83);
    let walk_frame = infantry_shp_frame_from_triples(0, 1, true, false, Some((8, 6, 6)), Some((0, 1, 6)), Some((52, 6, 6)));
    assert_eq!(walk_frame, 39); // Walk start 8 + 5*6 + 1
}

#[test]
fn vxl_hva_frame_follows_fire_and_walk_anim() {
    let idle = MobilePaintPose { anim_frame: 9, moving: false, firing: false, hit_flash: false, offset_x: 0, offset_y: 0, turret_facing: None };
    assert_eq!(mobile_vxl_hva_frame(idle), 0);
    let firing =
        MobilePaintPose { anim_frame: 2, moving: false, firing: true, hit_flash: false, offset_x: 0, offset_y: 0, turret_facing: Some(64) };
    assert_eq!(mobile_vxl_hva_frame(firing), 2);
    let walking =
        MobilePaintPose { anim_frame: 5, moving: true, firing: false, hit_flash: false, offset_x: 0, offset_y: 0, turret_facing: None };
    assert_eq!(mobile_vxl_hva_frame(walking), 5);
}

#[test]
fn vehicle_walk_frames_animate_per_facing() {
    // WalkFrames=6；facing 64 → 槽 2；移动步 3 → 帧 2*6+3=15；待机同向帧 12。
    assert_eq!(vehicle_shp_frame_from_walk_frames(64, 3, true, false, Some(6), None), 15);
    assert_eq!(vehicle_shp_frame_from_walk_frames(64, 3, false, false, Some(6), None), 12);
    // FiringFrames=4 接在 8*6 行走块后：槽 2 步 1 → 48+8+1=57。
    assert_eq!(vehicle_shp_frame_from_walk_frames(64, 1, true, true, Some(6), Some(4)), 57);
    // 无 WalkFrames 时回退朝向桶。
    assert_eq!(vehicle_shp_frame_from_walk_frames(64, 9, true, false, None, None), 2);
}

#[test]
fn hit_flash_brightens_opaque_pixels_toward_white() {
    let mut rgba = vec![10u8, 20, 30, 255, 0, 0, 0, 0];
    apply_hit_flash_rgba(&mut rgba);
    assert_eq!(rgba[0], 10 + (255 - 10) / 2);
    assert_eq!(rgba[1], 20 + (255 - 20) / 2);
    assert_eq!(rgba[2], 30 + (255 - 30) / 2);
    assert_eq!(&rgba[4..8], &[0, 0, 0, 0]);
}

#[test]
fn pose_slide_offset_is_added_to_blit_origin() {
    // 格内滑移必须叠到 TileBlit 原点上，否则步兵只会整格瞬移。
    let pose =
        MobilePaintPose { anim_frame: 0, moving: true, firing: false, hit_flash: false, offset_x: 12, offset_y: -8, turret_facing: None };
    let mut blit = TileBlit::solid(4, 4, 3, 5, vec![255; 4 * 4 * 4]);
    blit.shadow = Some(ShadowBlit { width: 2, height: 2, offset_x: 1, offset_y: 2, mask: vec![1; 4] });
    apply_mobile_pose_offset(&mut blit, pose);
    assert_eq!(blit.offset_x, 15);
    assert_eq!(blit.offset_y, -3);
    let shadow = blit.shadow.as_ref().expect("shadow");
    assert_eq!(shadow.offset_x, 13);
    assert_eq!(shadow.offset_y, -6);
}

#[test]
fn mobile_shp_offsets_anchor_to_cell_center() {
    // 画布 40×40、裁切原点 (8, 10) → 相对钻石中心，而非裸 frame_x/y。
    // ox = 8 - 20 + 30 = 18；oy = 10 - 20 + 15 = 5。
    assert_eq!(mobile_shp_cell_offsets(8, 10, 40, 40), (18, 5));
    // 禁止回退到裸偏移（那会是 (8, 10)）。
    assert_ne!(mobile_shp_cell_offsets(8, 10, 40, 40), (8, 10));
}

#[test]
fn infantry_sub_cell_offsets_spread_around_center() {
    assert_eq!(infantry_sub_cell_offsets(0), (0, 0));
    assert_eq!(infantry_sub_cell_offsets(1), (-14, -7));
    assert_eq!(infantry_sub_cell_offsets(2), (14, -7));
    assert_eq!(infantry_sub_cell_offsets(3), (-14, 7));
    assert_eq!(infantry_sub_cell_offsets(4), (14, 7));
    assert_eq!(infantry_sub_cell_offsets(9), (0, 0));
}

#[test]
fn slide_offset_moves_toward_next_cell() {
    let path = [(2u16, 1u16)];
    let (ox0, oy0) = slide_offset_along_path(1, 1, &path, 0, 0, 0.0, 64, |_, _| 0);
    assert_eq!((ox0, oy0), (0, 0));
    let (ox, oy) = slide_offset_along_path(1, 1, &path, 32, 0, 0.0, 64, |_, _| 0);
    assert!(ox != 0 || oy != 0, "mid-cell slide must leave cell origin");
}

#[test]
fn missing_mobile_body_notes_type_key() {
    use std::collections::HashMap;

    use ra_map::{MapEntity, MapEntityKind, PaintDefinitionsLoader};

    struct MapSource {
        files: HashMap<String, Vec<u8>>,
    }
    impl AssetSource for MapSource {
        fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
            self.files.get(&relative.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::MissingFile(relative.to_string()))
        }
    }

    fn solid_index_pal(index: usize, r6: u8, g6: u8, b6: u8) -> Vec<u8> {
        let mut data = vec![0u8; 768];
        let o = index * 3;
        data[o] = r6;
        data[o + 1] = g6;
        data[o + 2] = b6;
        data
    }

    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[E1]\nImage=E1\n".to_vec());
    files.insert("rules.ini".into(), b"[General]\n".to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    let source = MapSource { files };

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Americans".into(),
        type_id: "E1".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });

    let mut paint = PaintDefinitionsLoader::load_sealed(&source, "art.ini", "rules.ini", &Default::default(), &map);
    let mut image = TerrainImage::blank(64, 64);
    let painted = paint_map_mobiles(&source, &map, &mut image, &mut paint, &|p, _| p.clone(), &|_, _| MobilePaintPose::default());
    assert_eq!(painted, 0);
    assert!(paint.mobile_types_missing_shp().contains("E1"));
    assert_eq!(paint.mobile_types_missing_shp().len(), 1);
}
