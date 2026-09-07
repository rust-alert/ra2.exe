//! 建筑放置与资金扣除。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};
use ra_world::{CommandRejectReason, GameCommand, World};

fn yard_world() -> World {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n\
[AMCV]\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\n\
[GAPOWR]\nStrength=600\nSight=4\nCost=600\n\
[GAREFN]\nStrength=900\nSight=4\nCost=2000\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
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
    let mut world = World::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn place_power_deducts_funds_and_spawns_structure() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600));
    assert_eq!(world.entities.len(), 2);
    let power = &world.entities[1];
    assert_eq!(power.id, EntityId(2));
    assert_eq!(power.kind, MapEntityKind::Structure);
    assert_eq!(power.type_id, "GAPOWR");
    assert_eq!(power.owner, "Americans");
    assert_eq!((power.x, power.y), (6, 4));
    assert!(!world.pass_grid.is_passable(6, 4));
    assert_eq!(world.players[0].power_output, 200);
}

#[test]
fn place_building_rejects_insufficient_funds() {
    let mut world = yard_world();
    assert!(world.set_house_funds("Americans", 100));
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientFunds);
    assert_eq!(world.entities.len(), 1);
    assert_eq!(world.house_funds("Americans"), Some(100));
}

#[test]
fn place_building_rejects_missing_yard() {
    let mut world = yard_world();
    world.entities[0].dead = true;
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
    assert_eq!(world.entities.len(), 1);
}

#[test]
fn place_building_rejects_occupied_cell() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 4,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidPlacement);
    assert_eq!(world.entities.len(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
}

#[test]
fn place_refinery_rejects_without_power_plant() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAREFN".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
    assert_eq!(world.entities.len(), 1);
    assert_eq!(world.house_funds("Americans"), Some(10_000));
}

#[test]
fn place_refinery_after_power_deducts_and_drains() {
    let mut world = yard_world();
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAREFN".into(),
        x: 8,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 600 - 2000));
    assert_eq!(world.entities[2].type_id, "GAREFN");
    assert_eq!(world.players[0].power_output, 200);
    assert_eq!(world.players[0].power_drain, 50);
}
