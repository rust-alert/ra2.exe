//! 海军单位只走水面，拒绝上岸。

use crate::common::{battle_from_defs, defs_from_rules_ini, map_with_size};
use ra_engine::GameCommand;
use ra_map::{LandType, MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

fn defs_with_destroyer() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[VehicleTypes]\n0=DEST\n\
[DEST]\nStrength=600\nSpeed=64\nSight=7\nCost=1000\nArmor=heavy\nNaval=yes\nOwner=Americans\nPrimary=55mm\n\
[55mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    )
}

fn paint_water(world: &mut ra_engine::BattleState, cells: &[(u16, u16)]) {
    for &(x, y) in cells {
        world.pass_grid.set_land_type(x, y, LandType::Water);
        world.pass_grid.set_passable(x, y, false);
    }
    world.sync_prepared_pass_layers();
}

#[test]
fn naval_destroyer_moves_on_water_not_land() {
    let defs = defs_with_destroyer();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "DEST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    // 水面走廊 (8,8)-(12,8)；陆地目标 (12,4)。
    paint_water(&mut world, &[(8, 8), (9, 8), (10, 8), (11, 8), (12, 8)]);
    let id = world.entity_id_at(0).expect("entity");
    assert!(world.entity_is_naval(id));
    assert!(world.set_ecs_speed(id, 64));

    // 点陆地：应回落到最近水面，不得上岸。
    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 4 });
    for _ in 0..20 {
        world.advance_tick();
    }
    let (x, y, _) = world.ecs_transform(id).expect("xf");
    assert_eq!(y, 8, "destroyer must stay on water row, got ({x},{y})");
    assert!(world.pass_grid.is_naval_passable(x, y), "final cell must be water");
    assert!(!world.pass_grid.is_passable(x, y), "water remains ground-impassable");
}

#[test]
fn naval_path_stays_on_water_corridor() {
    let defs = defs_with_destroyer();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "DEST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    paint_water(&mut world, &[(8, 8), (9, 8), (10, 8), (11, 8), (12, 8)]);
    let id = world.entity_id_at(0).expect("entity");
    assert!(world.set_ecs_speed(id, 64));

    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 8 });
    world.advance_tick();
    let path = world.ecs_path(id).expect("path");
    assert!(!path.is_empty(), "must path along water");
    for &(px, py) in &path {
        assert!(
            world.pass_grid.is_naval_passable(px, py),
            "path cell ({px},{py}) must be water"
        );
        assert_eq!(py, 8);
    }
}

#[test]
fn naval_yard_spawn_cell_picks_water_not_land() {
    let rules = b"[BuildingTypes]\n0=GAYARD\n\
[VehicleTypes]\n0=DEST\n\
[GAYARD]\nWaterBound=yes\nFactory=UnitType\nOwner=Americans\nStrength=1000\nSight=4\nCost=1000\nTechLevel=1\nFoundation=2x2\n\
[DEST]\nStrength=600\nSpeed=64\nSight=7\nCost=1000\nArmor=heavy\nNaval=yes\nOwner=Americans\nPrimary=55mm\n\
[55mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let defs = defs_from_rules_ini(rules);
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GAYARD".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    // 船厂 2x2 与外沿一圈水面；更外一圈保持陆地可走。
    for y in 7u16..=10 {
        for x in 7u16..=10 {
            world.pass_grid.set_land_type(x, y, LandType::Water);
            world.pass_grid.set_passable(x, y, false);
        }
    }
    world.sync_prepared_pass_layers();
    let yard = world.entity_id_at(0).expect("yard");
    let cell = world.find_spawn_cell(8, 8, yard, true).expect("naval spawn");
    assert!(
        world.pass_grid.is_naval_passable(cell.0, cell.1),
        "spawn {:?} must be water",
        cell
    );
    // 地面出兵谓词不得把水面当可用邻格。
    assert!(world.find_spawn_cell(8, 8, yard, false).is_none());
}

#[test]
fn ground_tank_still_rejects_water() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=100\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    paint_water(&mut world, &[(10, 8), (11, 8), (12, 8)]);
    let id = world.entity_id_at(0).expect("entity");
    assert!(!world.entity_is_naval(id));
    assert!(world.set_ecs_speed(id, 64));

    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 8 });
    for _ in 0..20 {
        world.advance_tick();
    }
    let (x, y, _) = world.ecs_transform(id).expect("xf");
    // 坦克不得停在水上。
    assert_ne!(world.pass_grid.land_type(x, y), LandType::Water, "tank must not end on water at ({x},{y})");
}
