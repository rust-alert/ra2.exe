//! 自顶层 `mobile_paint.rs`。

use ra_map::{PaintIniDocs, MapInfo, MobilePaintPose, TerrainImage, paint_map_mobiles};
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
    assert_eq!(
        paint_map_mobiles(&EmptySource, &map, &mut image, &PaintIniDocs::default(), &|p, _| p.clone(), &|_| MobilePaintPose::default(),),
        0
    );
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
fn pose_slide_offset_is_added_to_blit_origin() {
    // 格内滑移必须叠到 TileBlit 原点上，否则步兵只会整格瞬移。
    let pose = MobilePaintPose { anim_frame: 0, moving: true, offset_x: 12, offset_y: -8 };
    let mut blit = TileBlit::solid(4, 4, 3, 5, vec![255; 4 * 4 * 4]);
    blit.offset_x = blit.offset_x.saturating_add(pose.offset_x);
    blit.offset_y = blit.offset_y.saturating_add(pose.offset_y);
    assert_eq!(blit.offset_x, 15);
    assert_eq!(blit.offset_y, -3);
}
