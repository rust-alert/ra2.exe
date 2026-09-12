//! 自顶层 `pass_grid.rs`。

use ra_map::{IsoCell, MapEntity, MapEntityKind, MapInfo, MapSmudge, PassGrid, TerrainObject};
use ra_types::GameEdition;

#[test]
fn structures_block_and_bfs_detours() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 5;
    map.height = 3;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "GAWALL".into(),
        health: 256,
        x: 2,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let grid = PassGrid::from_map(&map);
    assert!(!grid.is_passable(2, 1));
    assert_eq!(grid.blocked_count(), 1);
    let path = grid.find_path(0, 1, 4, 1).unwrap();
    assert_eq!(path.first(), Some(&(0, 1)));
    assert_eq!(path.last(), Some(&(4, 1)));
    assert!(!path.iter().any(|&(x, y)| x == 2 && y == 1));
}

#[test]
fn diagonal_path_is_shorter() {
    let grid = PassGrid::open(5, 5);
    let ortho = grid.find_path(0, 0, 2, 2).unwrap();
    let diag = grid.find_path_diag(0, 0, 2, 2).unwrap();
    assert_eq!(ortho.len(), 5); // (0,0)(1,0)(2,0)(2,1)(2,2) or similar
    assert_eq!(diag.len(), 3); // (0,0)(1,1)(2,2)
}

#[test]
fn seal_water_land_types() {
    let mut grid = PassGrid::open(3, 3);
    // TMP terrain_type 9 → Water；0 → Clear。
    let sealed = grid.seal_land_types(&[(1, 1, 9), (0, 0, 0)]);
    assert_eq!(sealed, 1);
    assert!(!grid.is_passable(1, 1));
    assert!(grid.is_passable(0, 0));
}

#[test]
fn cliff_blocks_path_ramp_allows() {
    let mut cliff = PassGrid::open(3, 1);
    cliff.set_height(0, 0, 0);
    cliff.set_height(1, 0, 2);
    cliff.set_height(2, 0, 2);
    assert!(cliff.find_path(0, 0, 2, 0).is_none());

    let mut ramp = PassGrid::open(3, 1);
    ramp.set_height(0, 0, 0);
    ramp.set_height(1, 0, 1);
    ramp.set_height(2, 0, 2);
    assert!(ramp.find_path(0, 0, 2, 0).is_some());
}

#[test]
fn from_map_loads_iso_heights() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 2;
    map.height = 1;
    map.cells.push(IsoCell { x: 0, y: 0, tile_num: 0, sub_tile: 0, z: 3, flags: 0 });
    map.cells.push(IsoCell { x: 1, y: 0, tile_num: 0, sub_tile: 0, z: 4, flags: 0 });
    let grid = PassGrid::from_map(&map);
    assert_eq!(grid.cell_height(0, 0), 3);
    assert_eq!(grid.cell_height(1, 0), 4);
    assert!(grid.find_path(0, 0, 1, 0).is_some());
}

#[test]
fn prepared_map_skeleton_seeds_anchor_occupancy() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 3;
    map.height = 2;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 1,
        y: 0,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.terrain_objects.push(TerrainObject { x: 2, y: 1, name: "TREE1".into() });
    map.smudges.push(MapSmudge { x: 0, y: 1, name: "CRATER1".into() });
    let prepared = map.to_prepared_map_skeleton();
    assert_eq!(prepared.occupancy.len(), 6);
    assert_eq!(prepared.occupancy[1], 1);
    assert_eq!(prepared.occupancy[1 * 3 + 1], 0);
    assert_eq!(prepared.occupancy[1 * 3 + 2], 2);
    assert_eq!(prepared.occupancy[1 * 3], 3);
}
