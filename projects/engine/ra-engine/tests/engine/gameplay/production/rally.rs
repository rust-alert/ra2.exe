//! 工厂集结点。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};

fn barracks_world() -> BattleState {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=64\nSight=5\nCost=200\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "rally");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPILE".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn set_rally_point_on_factory() {
    let mut world = barracks_world();
    world.push_command(GameCommand::SetRallyPoint { factory: EntityId(1), x: 10, y: 8 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_rally(world.entity_id_at(0).expect("entity")).expect("rally").0, Some(10));
    assert_eq!(world.ecs_rally(world.entity_id_at(0).expect("entity")).expect("rally").1, Some(8));
}

#[test]
fn produced_unit_paths_toward_rally_point() {
    let mut world = barracks_world();
    world.push_command(GameCommand::SetRallyPoint { factory: EntityId(1), x: 10, y: 2 });
    world.advance_tick();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    for _ in 0..(PRODUCE_TICKS - 1) {
        world.advance_tick();
    }
    assert_eq!(world.entity_count(), 2);
    let unit = world.entity_id_at(1).expect("unit");
    assert_eq!(world.ecs_identity(unit).expect("id").0.as_ref(), "E1");
    assert_eq!(world.ecs_move_destination(unit).expect("dest").0, Some(10));
    assert_eq!(world.ecs_move_destination(unit).expect("dest").1, Some(2));
    assert!(!world.ecs_path(unit).expect("path").is_empty());
}

#[test]
fn set_rally_rejects_non_factory() {
    let mut world = barracks_world();
    let id = world.entity_id_at(0).expect("entity");
    assert!(world.set_ecs_type_id(id, "GACNST", MapEntityKind::Structure));
    world.push_command(GameCommand::SetRallyPoint { factory: EntityId(1), x: 5, y: 5 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidTarget);
}
