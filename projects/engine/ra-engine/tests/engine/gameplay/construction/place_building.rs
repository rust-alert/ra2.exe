//! 建筑放置与资金扣除。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{CommandRejectReason, GameCommand, BattleState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};

fn yard_world() -> BattleState {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
    };
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
    }];
    let mut world = BattleState::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn place_power_deducts_funds_and_spawns_structure() {
    let mut world = yard_world();
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
    assert_eq!(world.players[0].power_output, 200);
}

#[test]
fn place_building_rejects_insufficient_funds() {
    let mut world = yard_world();
    assert!(world.set_house_funds("Americans", 100));
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
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
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
    assert_eq!(world.entity_count(), 1);
}

#[test]
fn place_building_rejects_occupied_cell() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 4, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
}

#[test]
fn place_refinery_rejects_without_power_plant() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAREFN".into(), x: 6, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientPower);
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
}

#[test]
fn place_refinery_after_power_deducts_and_drains() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
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
    let before = world.entity_count();
    world.push_scheduled(ScheduledCommand::new(
        CommandId(77),
        PlayerId(0),
        Tick(1),
        GameCommand::PlaceBuilding { player: PlayerId(1), type_id: "GAPOWR".into(), x: 6, y: 4 },
    ));
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert_eq!(world.entity_count(), before);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
}

#[test]
fn push_command_cannot_spoof_place_building_player_via_body() {
    let mut world = yard_world();
    assert_eq!(world.local_player, PlayerId(0));
    let before = world.entity_count();
    // 载荷声称 PlayerId(1)，但 push_command 信封必须仍是本地玩家 0 → 应用时拒绝。
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(1), type_id: "GAPOWR".into(), x: 6, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::WrongOwner);
    assert_eq!(world.entity_count(), before);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
    let frame = world.last_input_frame();
    assert_eq!(frame.commands.len(), 1);
    assert_eq!(frame.commands[0].player, PlayerId(0));
}
