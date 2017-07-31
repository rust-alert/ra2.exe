//! 采矿场等建筑 `FreeUnit=`：落位后白送单位。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, MissionKind, PlayerId};

fn refinery_world() -> BattleState {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n1=CMIN\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[CMIN]\nHarvester=yes\nOwner=Americans\nStrength=1000\nSpeed=4\nSight=4\nCost=1400\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nFreeUnit=CMIN\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\nFoundation=3x4\n";
    let defs = defs_from_rules_ini(rules_text);
    assert_eq!(defs.structures.get("GAREFN").and_then(|s| s.free_unit), Some(defs.techno.get("CMIN").expect("CMIN").id));
    let mut map = MapInfo::empty(GameEdition::Ra2, "free-unit");
    map.width = 24;
    map.height = 24;
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

fn queue_until_ready(world: &mut BattleState, type_id: &str) {
    let tid = world.definitions.techno.get(type_id).expect(type_id).id;
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: tid });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "Produce {type_id}: {:?}", world.last_rejects());
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("AMERICANS") == Some(tid) {
            return;
        }
        world.advance_tick();
    }
    panic!("expected {type_id} ready");
}

#[test]
fn place_refinery_spawns_free_unit_once() {
    let mut world = refinery_world();
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 7,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.entity_count(), 2);

    queue_until_ready(&mut world, "GAREFN");
    let funds_before = world.house_funds("AMERICANS").expect("funds");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAREFN").expect("GAREFN").id,
        x: 10,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    // 建造场 + 电厂 + 矿场 + FreeUnit 矿车
    assert_eq!(world.entity_count(), 4);
    assert_eq!(world.house_funds("AMERICANS"), Some(funds_before), "FreeUnit must not charge again");

    let miner = world.find_entity_id_by_owner_type("AMERICANS", "CMIN").expect("free CMIN");
    let identity = world.ecs_identity(miner).expect("identity");
    assert_eq!(identity.0.as_ref(), "CMIN");
    assert_eq!(identity.1, MapEntityKind::Unit);
    assert_eq!(world.ecs_mission(miner), Some(MissionKind::Harvest));
    let (mx, my) = world.ecs_transform(miner).map(|t| (t.0, t.1)).expect("pos");
    // 3x4 占地封死舱位后，矿车必须落在占地外。
    assert!(mx < 10 || mx >= 13 || my < 4 || my >= 8, "free unit must leave foundation ({mx},{my})");
}

#[test]
fn place_refinery_without_free_unit_spawns_nothing_extra() {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\nFoundation=3x4\n";
    let defs = defs_from_rules_ini(rules_text);
    assert!(defs.structures.get("GAREFN").unwrap().free_unit.is_none());
    let mut map = MapInfo::empty(GameEdition::Ra2, "no-free");
    map.width = 24;
    map.height = 24;
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
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 7,
        y: 4,
    });
    world.advance_tick();
    queue_until_ready(&mut world, "GAREFN");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAREFN").expect("GAREFN").id,
        x: 10,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.entity_count(), 3);
    assert!(world.find_entity_id_by_owner_type("AMERICANS", "CMIN").is_none());
}

#[test]
fn place_soviet_refinery_spawns_harv() {
    let rules_text = b"[VehicleTypes]\n0=SMCV\n1=HARV\n\
[BuildingTypes]\n0=NACNST\n1=NAPOWR\n2=NAREFN\n\
[SMCV]\nDeploysInto=NACNST\nOwner=Russians\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[HARV]\nHarvester=yes\nOwner=Russians\nStrength=1000\nSpeed=4\nSight=4\nCost=1400\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Russians\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=1x1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nFreeUnit=HARV\nOwner=Russians\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\nFoundation=2x2\n";
    let defs = defs_from_rules_ini(rules_text);
    assert_eq!(defs.structures.get("NAREFN").and_then(|s| s.free_unit), Some(defs.techno.get("HARV").expect("HARV").id));
    let mut map = MapInfo::empty(GameEdition::Ra2, "free-harv");
    map.width = 24;
    map.height = 24;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "RUSSIANS".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("RUSSIANS", 10_000));

    let napowr = world.definitions.techno.get("NAPOWR").expect("NAPOWR").id;
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: napowr });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("RUSSIANS") == Some(napowr) {
            break;
        }
        world.advance_tick();
    }
    assert_eq!(world.house_ready_building("RUSSIANS"), Some(napowr));
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: napowr, x: 7, y: 4 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());

    let narefn = world.definitions.techno.get("NAREFN").expect("NAREFN").id;
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: narefn });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("RUSSIANS") == Some(narefn) {
            break;
        }
        world.advance_tick();
    }
    assert_eq!(world.house_ready_building("RUSSIANS"), Some(narefn));
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: narefn, x: 10, y: 4 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    let miner = world.find_entity_id_by_owner_type("RUSSIANS", "HARV").expect("free HARV");
    assert_eq!(world.ecs_mission(miner), Some(MissionKind::Harvest));
}

#[test]
fn free_unit_refunds_when_no_open_cell() {
    // 3×2 图：右列建造场+电厂，左 2×2 放满矿场 → 无空格可放 FreeUnit，退还 Cost。
    let rules_text = b"[VehicleTypes]\n0=AMCV\n1=CMIN\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[CMIN]\nHarvester=yes\nOwner=Americans\nStrength=1000\nSpeed=4\nSight=4\nCost=1400\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=1x1\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nFreeUnit=CMIN\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\nFoundation=2x2\n";
    let defs = defs_from_rules_ini(rules_text);
    let cmin_cost = defs.techno.get("CMIN").expect("CMIN").cost;
    let mut map = MapInfo::empty(GameEdition::Ra2, "free-refund");
    map.width = 3;
    map.height = 2;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 2,
        y: 0,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));

    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 2,
        y: 1,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "{:?}", world.last_rejects());

    queue_until_ready(&mut world, "GAREFN");
    let funds_before = world.house_funds("AMERICANS").expect("funds");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAREFN").expect("GAREFN").id,
        x: 0,
        y: 0,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "{:?}", world.last_rejects());
    // 矿场 + 建造场 + 电厂，无 FreeUnit
    assert_eq!(world.entity_count(), 3);
    assert!(world.find_entity_id_by_owner_type("AMERICANS", "CMIN").is_none());
    assert_eq!(world.house_funds("AMERICANS"), Some(funds_before + cmin_cost));
}
