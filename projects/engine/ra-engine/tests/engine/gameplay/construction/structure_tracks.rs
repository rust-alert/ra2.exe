//! 建造场建筑栏 / 防御栏分轨并发（`BuildCat=Combat`）。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId};

fn yard_with_tabs() -> BattleState {
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAPILL\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n\
[GAPILL]\nBuildCat=Combat\nOwner=Americans\nStrength=400\nSight=5\nCost=400\nTechLevel=1\nFoundation=1x1\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "structure-tracks");
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
    world
}

#[test]
fn building_and_defense_tracks_produce_concurrently() {
    let mut world = yard_with_tabs();
    let power = world.definitions.techno.get("GAPOWR").expect("GAPOWR").id;
    let pill = world.definitions.techno.get("GAPILL").expect("GAPILL").id;
    let yard = world.entity_id_at(0).expect("yard");

    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: power });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "{:?}", world.last_rejects());

    // 建筑栏在产时，防御栏仍可开单。
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: pill });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "defense track should be independent: {:?}", world.last_rejects());
    // 边造边扣：两 tick 内建筑轨两步 + 防御轨一步。
    let power_step = 600 / PRODUCE_TICKS as i32;
    let pill_step = 400 / PRODUCE_TICKS as i32;
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 2 * power_step - pill_step));
    assert!(world.ecs_produce_item(yard).expect("build track").is_some());
    assert_eq!(world.ecs_produce_defense_busy(yard), Some(true));

    // 同轨再开应满。
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: power });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::QueueFull);

    for _ in 0..=PRODUCE_TICKS {
        world.advance_tick();
    }
    assert!(world.house_has_ready_building("AMERICANS", power));
    assert!(world.house_has_ready_building("AMERICANS", pill));
}
