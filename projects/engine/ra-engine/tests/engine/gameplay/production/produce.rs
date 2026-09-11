//! 兵营 / 战车工厂生产。

use ra_adaptor::RulesSystem;
use crate::common::battle_from_rules;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};

fn factory_world() -> BattleState {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GAPILE\n1=GAWEAP\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n\
[GAWEAP]\nPower=-30\nPowered=yes\nFactory=UnitType\nOwner=Americans\nStrength=1000\nSight=5\nCost=2000\nTechLevel=1\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
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
            mission: String::new(),
            tag: String::new(),
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
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let mut world = battle_from_rules(&rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn produce_infantry_spawns_after_queue_ticks() {
    let mut world = factory_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 200));
    assert_eq!(world.entity_count(), 2);
    for _ in 0..(PRODUCE_TICKS - 1) {
        assert_eq!(world.entity_count(), 2);
        world.advance_tick();
    }
    assert_eq!(world.entity_count(), 3);
    let unit = world.entity_id_at(2).expect("entity");
    assert_eq!(unit, EntityId(3));
    let identity = world.ecs_identity(unit).expect("id");
    assert_eq!(identity.1, MapEntityKind::Infantry);
    assert_eq!(identity.0.as_ref(), "E1");
    assert_eq!(world.ecs_owner(unit).expect("owner").as_ref(), "Americans");
}

#[test]
fn produce_rejects_insufficient_funds() {
    let mut world = factory_world();
    assert!(world.set_house_funds("Americans", 50));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InsufficientFunds);
    assert_eq!(world.entity_count(), 2);
}

#[test]
fn produce_rejects_when_queue_busy() {
    let mut world = factory_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::QueueFull);
}

#[test]
fn produce_rejects_without_matching_factory() {
    let mut world = factory_world();
    let id = world.entity_id_at(0).expect("entity");
    let max = world.ecs_health(world.entity_id_at(0).expect("entity")).expect("health").1;
    assert!(world.set_ecs_health(id, 0, max, true));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
}

#[test]
fn cancel_produce_refunds_and_clears_queue() {
    let mut world = factory_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000 - 200));
    assert!(world.take_eva_cues().is_empty());

    world.push_command(GameCommand::CancelProduce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("Americans"), Some(10_000));
    let cues = world.take_eva_cues();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].house.as_ref(), "Americans");
    assert_eq!(cues[0].event, "EVA_Canceled");

    // 取消后可再次排队，并在满 tick 后出兵。
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    for _ in 0..(PRODUCE_TICKS - 1) {
        world.advance_tick();
    }
    assert_eq!(world.entity_count(), 3);
}

#[test]
fn cancel_produce_rejects_when_not_queued() {
    let mut world = factory_world();
    world.push_command(GameCommand::CancelProduce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidTarget);
}

#[test]
fn funds_nag_repeats_on_speak_delay_while_broke_with_factory() {
    let rules_text = b"[AudioVisual]\nSpeakDelay=0.003\n\
[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "funds_nag");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPILE".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = battle_from_rules(&rules_db, map);
    assert!(world.set_house_funds("Americans", 50));
    // SpeakDelay=0.003 → ftol(2.7)=2 tick 周期。

    world.advance_tick();
    let first = world.take_eva_cues();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].event, "EVA_InsufficientFunds");

    world.advance_tick();
    assert!(world.take_eva_cues().is_empty(), "冷却中不应再播");

    world.advance_tick();
    let again = world.take_eva_cues();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].event, "EVA_InsufficientFunds");
}

#[test]
fn low_power_halves_production_tick_rate() {
    let mut world = factory_world();
    // 工厂种子未记账电力；显式制造低电。
    world.players[0].power_output = 0;
    world.players[0].power_drain = 50;
    assert!(world.players[0].low_power());

    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    let factory = world.entity_id_at(0).expect("barracks");
    let start = world.ecs_produce_remaining(factory).expect("queue").expect("item");

    // 满电时这两次都会扣 tick；低电时奇 tick 跳过，只扣一次。
    world.advance_tick();
    world.advance_tick();
    let after = world.ecs_produce_remaining(factory).expect("queue").expect("item");
    assert_eq!(after, start.saturating_sub(1), "low power should advance production only on even ticks");
}
