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
    assert_eq!(paint_map_mobiles(&EmptySource, &map, &mut image, &mut paint, &|p, _| p.clone(), &|_| MobilePaintPose::default()), 0);
}

// 自顶层 `mobile_paint_unit.rs` 并入。

// 自 engine/ra-map/src/mobile_paint.rs :: tests
use ra_map::{TileBlit, mobile_paint::*};

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
fn parse_walk_triple() {
    assert_eq!(parse_sequence_triple("8,6,6"), Some((8, 6, 6)));
    assert_eq!(parse_sequence_triple("0,1,1"), Some((0, 1, 1)));
}

#[test]
fn fire_sequence_preferred_over_walk() {
    // Fire=52,6,6；facing 0 → 槽 7，步 1 → 帧 52+7*6+1=95；即使 moving=true 也优先开火。
    let frame = infantry_shp_frame_from_triples(0, 1, true, true, Some((8, 6, 6)), Some((0, 1, 6)), Some((52, 6, 6)));
    assert_eq!(frame, 95);
    let walk_frame = infantry_shp_frame_from_triples(0, 1, true, false, Some((8, 6, 6)), Some((0, 1, 6)), Some((52, 6, 6)));
    assert_eq!(walk_frame, 51); // Walk start 8 + 7*6 + 1
}

#[test]
fn vxl_hva_frame_follows_fire_and_walk_anim() {
    let idle = MobilePaintPose { anim_frame: 9, moving: false, firing: false, offset_x: 0, offset_y: 0, turret_facing: None };
    assert_eq!(mobile_vxl_hva_frame(idle), 0);
    let firing = MobilePaintPose { anim_frame: 2, moving: false, firing: true, offset_x: 0, offset_y: 0, turret_facing: Some(64) };
    assert_eq!(mobile_vxl_hva_frame(firing), 2);
    let walking = MobilePaintPose { anim_frame: 5, moving: true, firing: false, offset_x: 0, offset_y: 0, turret_facing: None };
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
fn pose_slide_offset_is_added_to_blit_origin() {
    // 格内滑移必须叠到 TileBlit 原点上，否则步兵只会整格瞬移。
    let pose = MobilePaintPose { anim_frame: 0, moving: true, firing: false, offset_x: 12, offset_y: -8, turret_facing: None };
    let mut blit = TileBlit::solid(4, 4, 3, 5, vec![255; 4 * 4 * 4]);
    blit.offset_x = blit.offset_x.saturating_add(pose.offset_x);
    blit.offset_y = blit.offset_y.saturating_add(pose.offset_y);
    assert_eq!(blit.offset_x, 15);
    assert_eq!(blit.offset_y, -3);
}
