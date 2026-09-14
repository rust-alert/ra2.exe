//! 侧栏出售：残血比例半价退款、拆除脏集与主厂移交。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, MissionKind, PlayerId, occupancy_kind};

fn yard_with_power() -> BattleState {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "sell-building");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("AMERICANS").is_some() {
            break;
        }
        world.advance_tick();
    }
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    // 放置会推 Buildup 脏集；出售测例不关心呈现队列。
    let _ = world.take_structure_buildup_dirty();
    world
}

#[test]
fn sell_building_refunds_half_cost_and_frees_footprint() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600));
    assert_eq!(world.players[0].power_output, 200);
    assert!(!world.pass_grid.is_passable(6, 4));
    assert!(!world.pass_grid.is_passable(7, 5));

    world.push_command(GameCommand::SellBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(world.ecs_health(power).expect("health").2);
    assert_eq!(world.ecs_mission(power), Some(MissionKind::Selling));
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600 + 300));
    assert_eq!(world.players[0].power_output, 0);
    assert!(world.pass_grid.is_passable(6, 4));
    assert!(world.pass_grid.is_passable(7, 4));
    assert!(world.pass_grid.is_passable(6, 5));
    assert!(world.pass_grid.is_passable(7, 5));
    let idx = |x: u16, y: u16| (y as usize) * (world.prepared.pass_width as usize) + (x as usize);
    assert_eq!(world.prepared.passable[idx(6, 4)], 1);
    assert_eq!(world.prepared.passable[idx(7, 5)], 1);
    assert_eq!(world.prepared.occupancy[idx(6, 4)], occupancy_kind::EMPTY);
    assert_eq!(world.prepared.occupancy[idx(7, 5)], occupancy_kind::EMPTY);
    let teardown = world.take_structure_teardown_dirty();
    assert_eq!(teardown, vec![power]);
    let sfx = world.take_battle_sfx_cues();
    assert!(sfx.iter().any(|c| c.event == "BuildingSold"));
}

#[test]
fn sell_building_refunds_scaled_by_remaining_health() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    let max = world.ecs_health(power).expect("health").1;
    assert!(world.set_ecs_health(power, max / 2, max, false));
    let funds_before = world.house_funds("AMERICANS").expect("funds");
    world.push_command(GameCommand::SellBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    // cost 600 · 半血 → (600 * 0.5) / 2 = 150
    assert_eq!(world.house_funds("AMERICANS"), Some(funds_before + 150));
}

#[test]
fn sell_building_rejects_unsellable() {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\nUnsellable=yes\n";
    let defs = defs_from_rules_ini(rules_text);
    assert!(defs.structures.get("GAPOWR").expect("GAPOWR").unsellable);
    assert!(!defs.structures.get("GAPOWR").expect("GAPOWR").capabilities.contains(&ra_types::BuiltinCapability::Sellable));
    let mut map = MapInfo::empty(GameEdition::Ra2, "sell-unsellable");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GACNST".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPOWR".into(),
            health: 256,
            x: 6,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    let power = world.entity_id_at(1).expect("power");
    world.push_command(GameCommand::SellBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidTarget);
    assert!(!world.ecs_health(power).expect("health").2);
    assert!(world.take_structure_teardown_dirty().is_empty());
}

#[test]
fn sell_building_rejects_wrong_owner() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    world.push_command(GameCommand::SellBuilding { player: PlayerId(1), building: power });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert!(!world.ecs_health(power).expect("health").2);
    assert!(world.take_structure_teardown_dirty().is_empty());
}

#[test]
fn sell_primary_factory_reassigns_successor() {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAWEAP\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAWEAP]\nFactory=UnitType\nOwner=Americans\nStrength=1000\nSight=4\nCost=2000\nTechLevel=1\nFoundation=3x2\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "sell-primary");
    map.width = 24;
    map.height = 24;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GACNST".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAWEAP".into(),
            health: 256,
            x: 8,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAWEAP".into(),
            health: 256,
            x: 12,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 20_000));
    let first = world.entity_id_at(1).expect("weap0");
    let second = world.entity_id_at(2).expect("weap1");
    assert_eq!(world.ecs_is_primary(first), Some(true));
    assert_eq!(world.ecs_is_primary(second), Some(false));

    world.push_command(GameCommand::SellBuilding { player: PlayerId(0), building: first });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(world.ecs_health(first).expect("health").2);
    assert_eq!(world.ecs_is_primary(second), Some(true));
}
