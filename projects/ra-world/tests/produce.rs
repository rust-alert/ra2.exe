//! 兵营 / 战车工厂生产。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};
use ra_world::{CommandRejectReason, GameCommand, PRODUCE_TICKS, World};

fn factory_world() -> World {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GAPILE\n1=GAWEAP\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\n\
[GAPILE]\nStrength=500\nSight=5\nCost=500\n\
[GAWEAP]\nStrength=1000\nSight=5\nCost=2000\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "produce");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GAWEAP".into(),
            health: 256,
            x: 8,
            y: 4,
            facing: 0,
            sub_cell: 0,
        },
    ];
    let mut world = World::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn produce_infantry_spawns_after_queue_ticks() {
    let mut world = factory_world();
    world.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "E1".into(),
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 200));
    assert_eq!(world.entities.len(), 2);
    for _ in 0..(PRODUCE_TICKS - 1) {
        assert_eq!(world.entities.len(), 2);
        world.advance_tick();
    }
    assert_eq!(world.entities.len(), 3);
    let unit = &world.entities[2];
    assert_eq!(unit.id, EntityId(3));
    assert_eq!(unit.kind, MapEntityKind::Infantry);
    assert_eq!(unit.type_id, "E1");
    assert_eq!(unit.owner, "Americans");
}

#[test]
fn produce_rejects_insufficient_funds() {
    let mut world = factory_world();
    assert!(world.set_house_funds("Americans", 50));
    world.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "E1".into(),
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientFunds);
    assert_eq!(world.entities.len(), 2);
}

#[test]
fn produce_rejects_when_queue_busy() {
    let mut world = factory_world();
    world.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "E1".into(),
    });
    world.advance_tick();
    world.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "E1".into(),
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::QueueFull);
}

#[test]
fn produce_rejects_without_matching_factory() {
    let mut world = factory_world();
    world.entities[0].dead = true;
    world.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "E1".into(),
    });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
}
