//! 建筑：先 `Produce` 完工，再 `PlaceBuilding` 落位。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};

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
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: Default::default(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

/// 排队建造并推进至建造场 `ready`。
fn queue_until_ready(world: &mut BattleState, type_id: &str) {
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: type_id.into() });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "Produce {type_id} should start: {:?}", world.last_rejects());
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("Americans").is_some_and(|r| r.as_ref().eq_ignore_ascii_case(type_id)) {
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
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
    assert_eq!(world.entity_count(), 1);
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
    assert_eq!(world.entity_count(), 2);
    let power = world.entity_id_at(1).expect("entity");
    assert_eq!(power, EntityId(2));
    let identity = world.ecs_identity(power).expect("id");
    assert_eq!(identity.1, MapEntityKind::Structure);
    assert_eq!(identity.0.as_ref(), "GAPOWR");
    assert_eq!(world.ecs_owner(power).expect("owner").as_ref(), "Americans");
    assert_eq!(world.ecs_transform(power).map(|t| (t.0, t.1)), Some((6, 4)));
    assert!(!world.pass_grid.is_passable(6, 4));
    assert!(!world.pass_grid.is_passable(7, 4));
    assert!(!world.pass_grid.is_passable(6, 5));
    assert!(!world.pass_grid.is_passable(7, 5));
    assert_eq!(world.players[0].power_output, 200);
    assert!(world.house_ready_building("Americans").is_none());
    let paint = world.take_structure_buildup_dirty();
    assert!(paint.contains(&power), "PlaceBuilding must dirty structure buildup for host preview");
}

#[test]
fn place_building_rejects_without_ready_queue() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
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
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: Default::default(),
    }];
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(!world.pass_grid.is_passable(4, 4));
    assert!(!world.pass_grid.is_passable(6, 6), "map seed must seal full Foundation");
    assert!(world.pass_grid.is_passable(7, 7));
}

#[test]
fn place_building_rejects_when_footprint_overlaps_obstacle() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    // 电厂 2x2：左上 (3,3) 会盖住已有建造场 (4,4)。
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 3, y: 3 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
    assert!(world.house_ready_building("Americans").is_some());
}

#[test]
fn place_building_rejects_overlap_even_if_pass_grid_unsealed() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    // 故意只放开通行、不拆实体：旧逻辑只查锚点格会漏掉非原点重叠。
    world.pass_grid.set_passable(6, 4, true);
    world.pass_grid.set_passable(7, 4, true);
    world.pass_grid.set_passable(6, 5, true);
    world.pass_grid.set_passable(7, 5, true);
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 7, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn produce_building_rejects_insufficient_funds() {
    let mut world = yard_world();
    assert!(world.set_house_funds("Americans", 100));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "GAPOWR".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientFunds);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("Americans"), Some(100));
}

#[test]
fn place_building_rejects_missing_yard() {
    let mut world = yard_world();
    let id = world.entity_id_at(0).expect("entity");
    let max = world.ecs_health(world.entity_id_at(0).expect("entity")).expect("health").1;
    assert!(world.set_ecs_health(id, 0, max, true));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "GAPOWR".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
    assert_eq!(world.entity_count(), 1);
}

#[test]
fn place_building_rejects_occupied_cell() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 4, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
}

#[test]
fn produce_refinery_rejects_without_power_plant() {
    let mut world = yard_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "GAREFN".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientPower);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
}

#[test]
fn place_refinery_after_power_deducts_and_drains() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    queue_until_ready(&mut world, "GAREFN");
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAREFN".into(), x: 8, y: 4 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600 - 2000));
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
        GameCommand::PlaceBuilding { player: PlayerId(1), type_id: "GAPOWR".into(), x: 6, y: 4 },
    ));
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert_eq!(world.entity_count(), before);
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
}

#[test]
fn push_command_cannot_spoof_place_building_player_via_body() {
    let mut world = yard_world();
    queue_until_ready(&mut world, "GAPOWR");
    assert_eq!(world.local_player, PlayerId(0));
    let before = world.entity_count();
    // 载荷声称 PlayerId(1)，但 push_command 信封必须仍是本地玩家 0 → 应用时拒绝。
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(1), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert_eq!(world.entity_count(), before);
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
    let frame = world.last_input_frame();
    assert_eq!(frame.commands.len(), 1);
    assert_eq!(frame.commands[0].player, PlayerId(0));
}
