//! 自顶层 `overlay_pass.rs`。

use ra_assets::IniDocument;
use ra_map::{MapInfo, OverlayCell, PassGrid, apply_overlay_land_to_pass_grid};
use ra_types::GameEdition;

#[test]
fn low_bridge_opens_sealed_water_cell() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 4;
    map.height = 4;
    map.overlays.push(OverlayCell { x: 1, y: 2, overlay_id: 0, data: 0 });
    let rules = IniDocument::parse(b"[LOBRDG01]\nLand=Road\nNoUseTileLandType=yes\n").expect("rules");
    let mut grid = PassGrid::open(4, 4);
    grid.set_passable(1, 2, false);
    let n = apply_overlay_land_to_pass_grid(&map, &rules, &|id| (id == 0).then(|| "LOBRDG01".into()), &mut grid);
    assert_eq!(n, 1);
    assert!(grid.is_passable(1, 2));
}

#[test]
fn overlay_without_no_use_leaves_cell() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 2;
    map.height = 2;
    map.overlays.push(OverlayCell { x: 0, y: 0, overlay_id: 1, data: 0 });
    let rules = IniDocument::parse(b"[BRIDGE1]\nLand=Road\nNoUseTileLandType=false\n").expect("rules");
    let mut grid = PassGrid::open(2, 2);
    grid.set_passable(0, 0, false);
    let n = apply_overlay_land_to_pass_grid(&map, &rules, &|id| (id == 1).then(|| "BRIDGE1".into()), &mut grid);
    assert_eq!(n, 0);
    assert!(!grid.is_passable(0, 0));
}
