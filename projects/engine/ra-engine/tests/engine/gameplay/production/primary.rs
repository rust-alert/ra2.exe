//! 生产厂主厂（PRI）标记与入队优先。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{GameCommand, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, TechnoClass};

fn two_barracks_world() -> ra_engine::BattleState {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=64\nSight=5\nCost=200\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "primary");
    map.width = 24;
    map.height = 24;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 2,
            y: 2,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 6,
            y: 2,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world
}

#[test]
fn map_spawn_assigns_first_barracks_primary() {
    let world = two_barracks_world();
    let a = world.entity_id_at(0).expect("a");
    let b = world.entity_id_at(1).expect("b");
    assert_eq!(world.ecs_is_primary(a), Some(true));
    assert_eq!(world.ecs_is_primary(b), Some(false));
}

#[test]
fn set_primary_factory_switches_exclusive_flag() {
    let mut world = two_barracks_world();
    let a = world.entity_id_at(0).expect("a");
    let b = world.entity_id_at(1).expect("b");
    world.push_command(GameCommand::SetPrimaryFactory { factory: b });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_is_primary(a), Some(false));
    assert_eq!(world.ecs_is_primary(b), Some(true));
}

#[test]
fn enqueue_prefers_primary_factory() {
    let mut world = two_barracks_world();
    let a = world.entity_id_at(0).expect("a");
    let b = world.entity_id_at(1).expect("b");
    world.push_command(GameCommand::SetPrimaryFactory { factory: b });
    world.advance_tick();
    let index = world.find_unit_factory_for_enqueue("AMERICANS", TechnoClass::Infantry).expect("factory");
    assert_eq!(world.entity_id_at(index), Some(b));
    assert_ne!(world.entity_id_at(index), Some(a));
}

#[test]
fn primary_lost_reassigns_to_peer() {
    let mut world = two_barracks_world();
    let a = world.entity_id_at(0).expect("a");
    let b = world.entity_id_at(1).expect("b");
    assert_eq!(world.ecs_is_primary(a), Some(true));
    world.push_command(GameCommand::Delete { entity: a });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_is_primary(a), Some(false));
    assert_eq!(world.ecs_is_primary(b), Some(true));
}

#[test]
fn snapshot_projects_rally_as_move_goal_for_factory() {
    let mut world = two_barracks_world();
    let a = world.entity_id_at(0).expect("a");
    world.push_command(GameCommand::SetRallyPoint { factory: a, x: 14, y: 10 });
    world.advance_tick();
    let session = Session::from_state(world, "primary-snap");
    let unit = session.expect_battle().project_units(&[a]).into_iter().next().expect("unit");
    assert!(unit.is_structure());
    assert!(unit.is_primary);
    assert!(unit.move_goal_screen.is_some(), "集结格应投影为行动线终点");
}

#[test]
fn order_set_primary_on_session() {
    let world = two_barracks_world();
    let a = world.entity_id_at(0).expect("a");
    let b = world.entity_id_at(1).expect("b");
    let mut session = Session::from_state(world, "primary-order");
    assert!(session.expect_battle().selection_has_primary_factory(&[b]));
    session.expect_battle_mut().order_set_primary(&[b]);
    // 命令入队后需推进；此处直接 advance 世界。
    session.expect_battle_mut().world.advance_tick();
    assert_eq!(session.expect_battle().world.ecs_is_primary(a), Some(false));
    assert_eq!(session.expect_battle().world.ecs_is_primary(b), Some(true));
}
