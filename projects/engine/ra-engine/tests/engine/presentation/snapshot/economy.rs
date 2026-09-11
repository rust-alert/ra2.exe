//! 快照中的经济与生产队列。

use crate::common::{test_engine, battle_from_rules};
use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{CommandRejectReason, GameCommand, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId, TerrainSpawnerDefinitions};

#[test]
fn snapshot_exposes_funds_power_queue_and_rejects() {
    let engine = test_engine();
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
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
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = battle_from_rules(&rules_db, map);
    assert!(world.set_house_funds("Americans", 5_000));
    world.players[0].power_output = 200;
    world.players[0].power_drain = 20;
    let mut session = Session::from_state(world, "hud");
    session.expect_battle_mut().push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    assert_eq!(snap.players.len(), 1);
    assert_eq!(snap.players[0].funds, 4_800);
    assert_eq!(snap.players[0].power_output, 200);
    assert_eq!(snap.players[0].power_drain, 20);
    assert!(!snap.players[0].low_power);
    assert_eq!(snap.produce_queues.len(), 1);
    assert_eq!(snap.produce_queues[0].type_id.as_ref(), "E1");
    assert!(snap.produce_queues[0].remaining_ticks > 0);
    assert!(snap.last_rejects.is_empty());

    session.expect_battle_mut().push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    assert_eq!(snap.last_rejects[0].reason, CommandRejectReason::QueueFull);
}
