//! 建筑：先 `Produce` 完工，再 `PlaceBuilding` 落位。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId, occupancy_kind};

fn yard_world() -> BattleState {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\nFoundation=3x4\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "place-building");
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

/// 排队建造并推进至建造场 `ready`。
fn queue_until_ready(world: &mut BattleState, type_id: &str) {
    let tid = world.definitions.techno.get(type_id).expect(type_id).id;
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: tid });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "Produce {type_id} should start: {:?}", world.last_rejects());
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("AMERICANS") == Some(tid) {
            return;
        }
        world.advance_tick();
    }
    panic!("expected {type_id} ready after {PRODUCE_TICKS} ticks");
}

#[test]
fn place_power_deducts_funds_and_spawns_structure() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600));
    assert_eq!(world.entity_count(), 1);
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600));
    assert_eq!(world.entity_count(), 2);
    let power = world.entity_id_at(1).expect("entity");
    assert_eq!(power, EntityId(2));
    let identity = world.ecs_identity(power).expect("id");
    assert_eq!(identity.1, MapEntityKind::Structure);
    assert_eq!(identity.0.as_ref(), "GAPOWR");
    assert_eq!(world.ecs_owner(power).expect("owner").as_ref(), "AMERICANS");
    assert_eq!(world.ecs_transform(power).map(|t| (t.0, t.1)), Some((6, 4)));
    assert!(!world.pass_grid.is_passable(6, 4));
    assert!(!world.pass_grid.is_passable(7, 4));
    assert!(!world.pass_grid.is_passable(6, 5));
    assert!(!world.pass_grid.is_passable(7, 5));
    let idx = |x: u16, y: u16| (y as usize) * (world.prepared.pass_width as usize) + (x as usize);
    assert_eq!(world.prepared.passable[idx(6, 4)], 0);
    assert_eq!(world.prepared.passable[idx(7, 5)], 0);
    assert_eq!(world.prepared.occupancy[idx(6, 4)], occupancy_kind::STRUCTURE);
    assert_eq!(world.prepared.occupancy[idx(7, 5)], occupancy_kind::STRUCTURE);
    assert_eq!(world.players[0].power_output, 200);
    assert!(world.house_ready_building("AMERICANS").is_none());
    let paint = world.take_structure_buildup_dirty();
    assert!(paint.contains(&power), "PlaceBuilding must dirty structure buildup for host preview");
}

#[test]
fn place_building_rejects_without_ready_queue() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000));
    let idx = |x: u16, y: u16| (y as usize) * (world.prepared.pass_width as usize) + (x as usize);
    assert_eq!(world.prepared.occupancy[idx(6, 4)], occupancy_kind::EMPTY);
}

#[test]
fn map_seeded_structure_seals_full_foundation() {
    let rules_text = b"[BuildingTypes]\n0=GACNST\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\nFoundation=3x3\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "seed-foundation");
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
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(!world.pass_grid.is_passable(4, 4));
    assert!(!world.pass_grid.is_passable(6, 6), "map seed must seal full Foundation");
    assert!(world.pass_grid.is_passable(7, 7));
    let idx = |x: u16, y: u16| (y as usize) * (world.prepared.pass_width as usize) + (x as usize);
    assert_eq!(world.prepared.passable[idx(4, 4)], 0);
    assert_eq!(world.prepared.passable[idx(6, 6)], 0);
    assert_eq!(world.prepared.occupancy[idx(4, 4)], 1);
    assert_eq!(world.prepared.definition.name.as_str(), "seed-foundation");
}

#[test]
fn place_building_rejects_when_footprint_overlaps_obstacle() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    // 电厂 2x2：左上 (3,3) 会盖住已有建造场 (4,4)。
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 3,
        y: 3,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600));
    assert!(world.house_ready_building("AMERICANS").is_some());
}

#[test]
fn place_building_rejects_overlap_even_if_pass_grid_unsealed() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    // 故意只放开通行、不拆实体：旧逻辑只查锚点格会漏掉非原点重叠。
    world.pass_grid.set_passable(6, 4, true);
    world.pass_grid.set_passable(7, 4, true);
    world.pass_grid.set_passable(6, 5, true);
    world.pass_grid.set_passable(7, 5, true);
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 7,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn produce_building_rejects_insufficient_funds() {
    let mut world = yard_world();
    assert!(world.set_house_funds("AMERICANS", 100));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientFunds);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("AMERICANS"), Some(100));
}

#[test]
fn place_building_rejects_missing_yard() {
    let mut world = yard_world();
    let id = world.entity_id_at(0).expect("entity");
    let max = world.ecs_health(world.entity_id_at(0).expect("entity")).expect("health").1;
    assert!(world.set_ecs_health(id, 0, max, true));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
    assert_eq!(world.entity_count(), 1);
}

#[test]
fn place_building_rejects_occupied_cell() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 4,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600));
}

#[test]
fn produce_refinery_rejects_without_power_plant() {
    let mut world = yard_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("GAREFN").expect("GAREFN").id });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientPower);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000));
}

#[test]
fn place_refinery_after_power_deducts_and_drains() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 6,
        y: 4,
    });
    world.advance_tick();
    queue_until_ready(&mut world, "GAREFN");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAREFN").expect("GAREFN").id,
        x: 8,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600 - 2000));
    assert_eq!(world.ecs_identity(world.entity_id_at(2).expect("entity")).expect("id").0.as_ref(), "GAREFN");
    assert_eq!(world.players[0].power_output, 200);
    assert_eq!(world.players[0].power_drain, 50);
}

#[test]
fn place_building_rejects_envelope_player_mismatch() {
    use ra_types::{CommandId, ScheduledCommand, Tick};

    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    let before = world.entity_count();
    world.push_scheduled(ScheduledCommand::new(
        CommandId(77),
        PlayerId(0),
        Tick(world.tick + 1),
        GameCommand::PlaceBuilding { player: PlayerId(1), type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id, x: 6, y: 4 },
    ));
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert_eq!(world.entity_count(), before);
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600));
}

#[test]
fn push_command_cannot_spoof_place_building_player_via_body() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    assert_eq!(world.local_player, PlayerId(0));
    let before = world.entity_count();
    // 载荷声称 PlayerId(1)，但 push_command 信封必须仍是本地玩家 0 → 应用时拒绝。
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(1),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert_eq!(world.entity_count(), before);
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 600));
    let frame = world.last_input_frame();
    assert_eq!(frame.commands.len(), 1);
    assert_eq!(frame.commands[0].player, PlayerId(0));
}

#[test]
fn water_bound_shipyard_places_on_water_rejects_land() {
    use ra_map::LandType;

    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAYARD\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAYARD]\nWaterBound=yes\nFactory=UnitType\nOwner=Americans\nStrength=1000\nSight=4\nCost=1000\nTechLevel=1\nFoundation=2x2\n";
    let defs = defs_from_rules_ini(rules_text);
    assert!(defs.structures.get("GAYARD").expect("GAYARD").water_bound);
    let mut map = MapInfo::empty(GameEdition::Ra2, "water-bound-place");
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

    // 模拟 TMP 封水：一块 2x2 水域，陆地仍可走。
    for (x, y) in [(8u16, 8u16), (9, 8), (8, 9), (9, 9)] {
        world.pass_grid.set_land_type(x, y, LandType::Water);
        world.pass_grid.set_passable(x, y, false);
    }
    world.sync_prepared_pass_layers();

    queue_until_ready(&mut world, "GAYARD");
    let yard_id = world.definitions.techno.get("GAYARD").expect("GAYARD").id;

    // 陆地应拒绝。
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: yard_id, x: 6, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 1);
    // 完工件仍在，可继续落水。
    assert_eq!(world.house_ready_building("AMERICANS"), Some(yard_id));

    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: yard_id, x: 8, y: 8 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "water place rejects: {:?}", world.last_rejects());
    assert_eq!(world.entity_count(), 2);
    let shipyard = world.entity_id_at(1).expect("shipyard");
    let identity = world.ecs_identity(shipyard).expect("id");
    assert_eq!(identity.1, MapEntityKind::Structure);
    assert_eq!(identity.0.as_ref(), "GAYARD");
    assert_eq!(world.ecs_transform(shipyard).map(|t| (t.0, t.1)), Some((8, 8)));
    assert!(!world.pass_grid.is_passable(8, 8));
    assert_eq!(world.pass_grid.land_type(8, 8), LandType::Water);
}

#[test]
fn place_building_rejects_outside_base_normal_zone() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    // 建造场在 (4,4)；默认 Adjacent=3 → 最大切比雪夫 4。远处置空地应拒。
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 12,
        y: 12,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 1);
    assert!(world.house_ready_building("AMERICANS").is_some());
}

#[test]
fn place_building_respects_adjacent_gap() {
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAPOWR\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\nBaseNormal=yes\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=1x1\nAdjacent=0\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "adjacent-zero");
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
    assert_eq!(world.definitions.structures.get("GAPOWR").map(|s| s.adjacent), Some(0));

    queue_until_ready(&mut world, "GAPOWR");
    // Adjacent=0：隔一格应拒。
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);

    // 贴边应通过。
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 5,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "touching place rejects: {:?}", world.last_rejects());
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn place_building_ignores_non_base_normal_as_anchor() {
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAPILL\n2=GAPOWR\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\nBaseNormal=yes\n\
[GAPILL]\nPower=-10\nOwner=Americans\nStrength=400\nSight=5\nCost=500\nTechLevel=1\nFoundation=1x1\nBaseNormal=no\nAdjacent=0\nBuildCat=Combat\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=1x1\nAdjacent=0\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "base-normal-no");
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
            type_id: "GAPILL".into(),
            health: 256,
            x: 10,
            y: 10,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    assert_eq!(world.definitions.structures.get("GAPILL").map(|s| s.base_normal), Some(false));

    queue_until_ready(&mut world, "GAPOWR");
    // 仅挨着 BaseNormal=no 的防御塔、远离建造场 → 应拒。
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAPOWR").expect("GAPOWR").id,
        x: 11,
        y: 10,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn wall_chain_places_within_guard_range_and_autofills() {
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAWALL\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\nBaseNormal=yes\n\
[GAWALL]\nWall=yes\nBaseNormal=no\nAdjacent=0\nGuardRange=4\nOwner=Americans\nStrength=100\nSight=1\nCost=50\nTechLevel=1\nFoundation=1x1\nBuildCat=Combat\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "wall-chain");
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
        // 已有墙枢纽：贴建造场，作为链起点。
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAWALL".into(),
            health: 256,
            x: 5,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    assert_eq!(world.definitions.structures.get("GAWALL").map(|s| (s.wall, s.guard_range, s.base_normal)), Some((true, 4, false)));

    queue_until_ready(&mut world, "GAWALL");
    // 沿墙正交延到轴距 4：中间三格应免费补齐。
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAWALL").expect("GAWALL").id,
        x: 9,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "wall chain rejects: {:?}", world.last_rejects());
    // 原 2 + 中间 (6,7,8) + 点击 (9) = 6
    assert_eq!(world.entity_count(), 6);
    for x in 6u16..=9 {
        assert!(!world.pass_grid.is_passable(x, 4), "wall segment sealed @({x},4)");
    }
    // 一次完工件，费用只扣一截墙。
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 50));
    assert!(world.house_ready_building("AMERICANS").is_none());
}

#[test]
fn wall_chain_rejects_beyond_guard_range_and_diagonal() {
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAWALL\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\nBaseNormal=yes\n\
[GAWALL]\nWall=yes\nBaseNormal=no\nAdjacent=0\nGuardRange=4\nOwner=Americans\nStrength=100\nSight=1\nCost=50\nTechLevel=1\nFoundation=1x1\nBuildCat=Combat\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "wall-chain-reject");
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
            type_id: "GAWALL".into(),
            health: 256,
            x: 5,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    let wall_id = world.definitions.techno.get("GAWALL").expect("GAWALL").id;

    queue_until_ready(&mut world, "GAWALL");
    // 轴距 5 > GuardRange=4，且已超出建造场 Adjacent=0。
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: wall_id, x: 10, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 2);

    // 斜向不可链。
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: wall_id, x: 7, y: 6 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn wall_chain_blocked_path_rejects() {
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAWALL\n2=GAPOWR\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\nBaseNormal=yes\n\
[GAWALL]\nWall=yes\nBaseNormal=no\nAdjacent=0\nGuardRange=4\nOwner=Americans\nStrength=100\nSight=1\nCost=50\nTechLevel=1\nFoundation=1x1\nBuildCat=Combat\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=1x1\nAdjacent=0\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "wall-chain-block");
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
            type_id: "GAWALL".into(),
            health: 256,
            x: 5,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        // 挡在墙链中间。
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPOWR".into(),
            health: 256,
            x: 7,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    queue_until_ready(&mut world, "GAWALL");
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: world.definitions.techno.get("GAWALL").expect("GAWALL").id,
        x: 9,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 3);
}
