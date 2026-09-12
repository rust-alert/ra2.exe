//! 矿车采集矿格后邻接矿场卸货入账。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo, OverlayCell};
use ra_types::GameEdition;

const MINING_RULES: &[u8] = b"[BuildingTypes]\n0=GAREFN\n\
[VehicleTypes]\n0=CMIN\n\
[OverlayTypes]\n0=TIB01\n\
[TIB01]\nTiberium=yes\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\n\
[CMIN]\nHarvester=yes\nOwner=Americans\nStrength=1000\nSpeed=4\nSight=4\nCost=1400\n";

fn mining_world() -> BattleState {
    let defs = defs_from_rules_ini(MINING_RULES);
    let mut map = MapInfo::empty(GameEdition::Ra2, "ore-income");
    map.width = 8;
    map.height = 8;
    map.overlays = vec![OverlayCell { x: 3, y: 2, overlay_id: 0, data: 2 }];
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAREFN".into(),
            health: 256,
            x: 2,
            y: 2,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "AMERICANS".into(),
            type_id: "CMIN".into(),
            health: 256,
            x: 3,
            y: 2,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 1_000));
    world
}

#[test]
fn harvester_on_ore_delivers_at_adjacent_refinery() {
    let mut world = mining_world();
    assert_eq!(world.harvestable_ore_at(3, 2), Some(2));

    for _ in 0..(ORE_TRIP_TICKS - 1) {
        world.advance_tick();
        assert_eq!(world.house_funds("AMERICANS"), Some(1_000));
        assert_eq!(world.harvestable_ore_at(3, 2), Some(2));
    }
    // 采集完成：扣密度并装载，本 tick 不卸货。
    world.advance_tick();
    assert_eq!(world.harvestable_ore_at(3, 2), Some(1));
    assert_eq!(world.take_overlay_paint_dirty(), vec![(3, 2)]);
    assert_eq!(world.house_funds("AMERICANS"), Some(1_000));

    // 邻接矿场：下一 tick 卸货入账。
    world.advance_tick();
    assert_eq!(world.house_funds("AMERICANS"), Some(1_000 + ORE_INCOME_PER_TRIP as i32));

    for _ in 0..(ORE_TRIP_TICKS - 1) {
        world.advance_tick();
        assert_eq!(world.house_funds("AMERICANS"), Some(1_000 + ORE_INCOME_PER_TRIP as i32));
    }
    world.advance_tick();
    assert_eq!(world.harvestable_ore_at(3, 2), None);
    assert_eq!(world.take_overlay_paint_dirty(), vec![(3, 2)]);
    assert_eq!(world.house_funds("AMERICANS"), Some(1_000 + ORE_INCOME_PER_TRIP as i32));
    world.advance_tick();
    assert_eq!(world.house_funds("AMERICANS"), Some(1_000 + 2 * ORE_INCOME_PER_TRIP as i32));
}

#[test]
fn dead_harvester_stops_ore_income() {
    let mut world = mining_world();
    let id = world.entity_id_at(1).expect("harvester");
    let max = world.ecs_health(id).expect("health").1;
    assert!(world.set_ecs_health(id, 0, max, true));
    for _ in 0..ORE_TRIP_TICKS {
        world.advance_tick();
    }
    assert_eq!(world.house_funds("AMERICANS"), Some(1_000));
    assert_eq!(world.harvestable_ore_at(3, 2), Some(2));
}

#[test]
fn idle_harvester_seeks_ore_then_returns_to_refinery() {
    let defs = defs_from_rules_ini(MINING_RULES);
    let mut map = MapInfo::empty(GameEdition::Ra2, "ore-seek");
    map.width = 8;
    map.height = 8;
    map.overlays = vec![OverlayCell { x: 3, y: 2, overlay_id: 0, data: 1 }];
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAREFN".into(),
            health: 256,
            x: 2,
            y: 2,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "AMERICANS".into(),
            type_id: "CMIN".into(),
            health: 256,
            x: 5,
            y: 2,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 1_000));
    let id = world.entity_id_at(1).expect("harvester");

    world.advance_tick();
    assert_eq!(world.ecs_move_destination(id).expect("dest"), (Some(3), Some(2)), "空闲空载应指向最近矿格");

    // Speed=4 · CELL_MOVE_COST=64 → 每格 16 tick；两格约 32 tick，再加采集与卸货。
    for _ in 0..200 {
        world.advance_tick();
        if world.house_funds("AMERICANS") == Some(1_000 + ORE_INCOME_PER_TRIP as i32) {
            return;
        }
    }
    panic!("expected one ore delivery after auto seek/return, funds={:?} pos={:?}", world.house_funds("AMERICANS"), world.ecs_transform(id));
}
