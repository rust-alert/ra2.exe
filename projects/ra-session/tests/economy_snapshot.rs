//! 快照中的经济与生产队列。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_session::Session;
use ra_types::{GameEdition, PlayerId};
use ra_world::{CommandRejectReason, GameCommand, World};

#[test]
fn snapshot_exposes_funds_power_queue_and_rejects() {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\n\
[GAPILE]\nStrength=500\nSight=5\nCost=500\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "hud-snap");
    map.width = 12;
    map.height = 12;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPILE".into(),
        health: 256,
        x: 3,
        y: 3,
        facing: 0,
        sub_cell: 0,
    }];
    let mut world = World::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 5_000));
    world.players[0].power_output = 200;
    world.players[0].power_drain = 20;
    let mut session = Session::new(world, "hud");
    session.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "E1".into(),
    });
    session.tick();
    let snap = session.snapshot();
    assert_eq!(snap.players.len(), 1);
    assert_eq!(snap.players[0].funds, 4_800);
    assert_eq!(snap.players[0].power_output, 200);
    assert_eq!(snap.players[0].power_drain, 20);
    assert!(!snap.players[0].low_power);
    assert_eq!(snap.produce_queues.len(), 1);
    assert_eq!(snap.produce_queues[0].type_id, "E1");
    assert!(snap.produce_queues[0].remaining_ticks > 0);
    assert!(snap.last_rejects.is_empty());

    session.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "E1".into(),
    });
    session.tick();
    let snap = session.snapshot();
    assert_eq!(snap.last_rejects[0].reason, CommandRejectReason::QueueFull);
}
