//! 自顶层 `overlay_pass.rs`。

use ra_assets::{IniDocument, overlay_types_from_rules};
use ra_map::{MapInfo, OverlayCell, PassGrid, apply_overlay_land_to_pass_grid};
use ra_types::GameEdition;

#[test]
fn low_bridge_opens_sealed_water_cell() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 4;
    map.height = 4;
    map.overlays.push(OverlayCell { x: 1, y: 2, overlay_id: 0, data: 0 });
    let rules = IniDocument::parse(b"[OverlayTypes]\n0=LOBRDG01\n[LOBRDG01]\nLand=Road\nNoUseTileLandType=yes\n").expect("rules");
    let overlays = overlay_types_from_rules(&rules);
    let mut grid = PassGrid::open(4, 4);
    grid.set_passable(1, 2, false);
    let n = apply_overlay_land_to_pass_grid(&map, &overlays, &mut grid);
    assert_eq!(n, 1);
    assert!(grid.is_passable(1, 2));
}

#[test]
fn overlay_without_no_use_leaves_cell() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 2;
    map.height = 2;
    map.overlays.push(OverlayCell { x: 0, y: 0, overlay_id: 0, data: 0 });
    let rules = IniDocument::parse(b"[OverlayTypes]\n0=BRIDGE1\n[BRIDGE1]\nLand=Road\nNoUseTileLandType=false\n").expect("rules");
    let overlays = overlay_types_from_rules(&rules);
    let mut grid = PassGrid::open(2, 2);
    grid.set_passable(0, 0, false);
    let n = apply_overlay_land_to_pass_grid(&map, &overlays, &mut grid);
    assert_eq!(n, 0);
    assert!(!grid.is_passable(0, 0));
}

#[test]
fn prepared_skeleton_with_overlays_opens_bridge_cell() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 4;
    map.height = 4;
    // `PassGrid::from_map` 默认全可走；先放建筑封死，再靠桥面 overlay 重开。
    map.entities.push(ra_map::MapEntity {
        kind: ra_map::MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "DUMMY".into(),
        health: 256,
        x: 1,
        y: 2,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.overlays.push(OverlayCell { x: 1, y: 2, overlay_id: 0, data: 0 });
    let rules = IniDocument::parse(b"[OverlayTypes]\n0=LOBRDG01\n[LOBRDG01]\nLand=Road\nNoUseTileLandType=yes\n").expect("rules");
    let overlays = overlay_types_from_rules(&rules);
    let prepared = map.to_prepared_map_skeleton_with_overlays(&overlays);
    let i = 2 * 4 + 1;
    assert_eq!(prepared.passable[i], 1);
    let bare = map.to_prepared_map_skeleton();
    assert_eq!(bare.passable[i], 0);
}
