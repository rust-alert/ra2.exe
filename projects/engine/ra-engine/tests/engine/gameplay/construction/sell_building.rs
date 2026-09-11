//! 侧栏出售：退半价并移除己方建筑。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId};

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

#[test]
fn sell_building_refunds_half_cost_and_frees_footprint() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
    assert_eq!(world.players[0].power_output, 200);
    assert!(!world.pass_grid.is_passable(6, 4));
    assert!(!world.pass_grid.is_passable(7, 5));

    world.push_command(GameCommand::SellBuilding { player: PlayerId(0), building: power });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(world.ecs_health(power).expect("health").2);
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600 + 300));
    assert_eq!(world.players[0].power_output, 0);
    assert!(world.pass_grid.is_passable(6, 4));
    assert!(world.pass_grid.is_passable(7, 4));
    assert!(world.pass_grid.is_passable(6, 5));
    assert!(world.pass_grid.is_passable(7, 5));
}

#[test]
fn sell_building_rejects_wrong_owner() {
    let mut world = yard_with_power();
    let power = world.entity_id_at(1).expect("power");
    world.push_command(GameCommand::SellBuilding { player: PlayerId(1), building: power });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert!(!world.ecs_health(power).expect("health").2);
}
