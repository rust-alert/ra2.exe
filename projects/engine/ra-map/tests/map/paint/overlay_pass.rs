//! 自顶层 `overlay_pass.rs`。

use ra_assets::{IniDocument, overlay_types_from_rules};
use ra_map::{MapInfo, OverlayCell, PassGrid, apply_overlay_land_to_pass_grid, finalize_battle_pass_grid};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

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

/// 空资源源：无剧院 TMP，仅跑 overlay land 半段。
struct EmptyAssetSource;
impl AssetSource for EmptyAssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn finalize_battle_pass_grid_reopens_bridge_after_tmp_seal() {
    // 对局装载序：Foundation 种子 → TMP 封水 → overlay land 重开桥面。
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 4;
    map.height = 4;
    map.overlays.push(OverlayCell { x: 1, y: 2, overlay_id: 0, data: 0 });
    let rules = IniDocument::parse(b"[OverlayTypes]\n0=LOBRDG01\n[LOBRDG01]\nLand=Road\nNoUseTileLandType=yes\n").expect("rules");
    let overlays = overlay_types_from_rules(&rules);

    let prepared = map.to_prepared_map_skeleton_with_structures(&Default::default());
    let mut grid = PassGrid::from_prepared_pass_layers(prepared.pass_width, prepared.pass_height, &prepared.passable, &prepared.cell_heights, &prepared.land_types);
    // 模拟 TMP 把桥下格子封成水（本夹具无真实剧院文件）。
    grid.set_passable(1, 2, false);
    assert!(!grid.is_passable(1, 2));

    let stats = finalize_battle_pass_grid(&EmptyAssetSource, &map, &overlays, &mut grid);
    assert_eq!(stats.tmp_sealed, 0, "empty source must skip TMP");
    assert_eq!(stats.overlay_land, 1);
    assert!(grid.is_passable(1, 2));
}
