//! 侧栏修理：切换持续修理标记，按 RepairRate / RepairStep / RepairPercent 步进。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId};

fn yard_with_power() -> BattleState {
    let rules_text = b"[General]\nRepairPercent=15\nRepairStep=8\nRepairRate=.016\n\
[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "repair-building");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "GAPOWR".into() });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("Americans").is_some() {
            break;
        }
        world.advance_tick();
    }
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    world
}

fn is_repairing(world: &BattleState, id: ra_types::EntityId) -> bool {
    world.is_repairing(id)
}

#[test]
fn repair_building_toggles_repairing_marker() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    assert!(world.set_ecs_health(power, 300, 600, false));

    world.push_command(GameCommand::RepairBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(is_repairing(&world, power));

    world.push_command(GameCommand::RepairBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(!is_repairing(&world, power));
}

#[test]
fn repair_building_heals_over_pulses_with_repair_percent_fee() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    assert!(world.set_ecs_health(power, 300, 600, false));
    let funds_before = world.house_funds("Americans").expect("funds");

    world.push_command(GameCommand::RepairBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert!(is_repairing(&world, power));

    // 推进到下一个修理脉冲（间隔 14 tick）。
    for _ in 0..13 {
        world.advance_tick();
    }
    let (cur, max, _) = world.ecs_health(power).expect("health");
    assert_eq!(max, 600);
    assert_eq!(cur, 308); // +8
    // fee = 600 * 15 * 8 / (100 * 600) = 1（整数除法）
    assert_eq!(world.house_funds("Americans"), Some(funds_before - 1));
    assert!(is_repairing(&world, power));
}

#[test]
fn repair_building_stops_when_funds_run_out() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    assert!(world.set_ecs_health(power, 300, 600, false));
    assert!(world.set_house_funds("Americans", 0));

    world.push_command(GameCommand::RepairBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert!(is_repairing(&world, power));

    for _ in 0..13 {
        world.advance_tick();
    }
    assert!(!is_repairing(&world, power));
    assert_eq!(world.ecs_health(power).map(|h| h.0), Some(300));
    assert_eq!(world.house_funds("Americans"), Some(0));
}

#[test]
fn repair_building_rejects_wrong_owner() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    world.push_command(GameCommand::RepairBuilding { player: PlayerId(1), building: power });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert!(!is_repairing(&world, power));
}
