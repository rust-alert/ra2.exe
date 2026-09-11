//! MCV 部署与资金播种。

use ra_adaptor::RulesSystem;
use crate::common::battle_from_rules;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, CommandRejectReason, GameCommand};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

fn mcv_world() -> BattleState {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n1=MTNK\n\
[BuildingTypes]\n0=GACNST\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n";
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
    let mut map = MapInfo::empty(GameEdition::Ra2, "mcv-deploy");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "AMCV".into(),
        health: 256,
        x: 5,
        y: 5,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    }];
    battle_from_rules(&rules_db, map)
}

#[test]
fn set_house_funds_updates_player_state() {
    let mut world = mcv_world();
    assert_eq!(world.house_funds("Americans"), Some(0));
    assert!(world.set_house_funds("Americans", 10_000));
    assert_eq!(world.house_funds("Americans"), Some(10_000));
    assert!(!world.set_house_funds("Nobody", 1));
}

#[test]
fn set_all_players_funds_seeds_every_house() {
    let mut world = mcv_world();
    world.set_all_players_funds(8_000);
    assert_eq!(world.house_funds("Americans"), Some(8_000));
    for player in &world.players {
        assert_eq!(player.funds, 8_000);
    }
}

#[test]
fn deploy_mcv_becomes_construction_yard() {
    let mut world = mcv_world();
    let id = world.entity_id_at(0).expect("entity");
    world.push_command(GameCommand::Deploy { entity: EntityId(1) });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.entity_id_at(0).expect("entity"), id);
    assert_eq!(world.entity_id_at(0).expect("entity"), EntityId(1));
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").1, MapEntityKind::Structure);
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").0.as_ref(), "GACNST");
    assert_eq!(world.ecs_speed(world.entity_id_at(0).expect("entity")).expect("speed"), 0);
    assert!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0.is_none());
    assert!(world.take_eva_cues().iter().all(|c| c.event != "EVA_UnitLost"), "Deploy must not announce unit lost");
}

#[test]
fn deploy_rejects_non_mcv_unit() {
    let mut world = mcv_world();
    let id = world.entity_id_at(0).expect("entity");
    assert!(world.set_ecs_type_id(id, "MTNK", MapEntityKind::Unit));
    world.push_command(GameCommand::Deploy { entity: EntityId(1) });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::CannotDeploy);
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").1, MapEntityKind::Unit);
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").0.as_ref(), "MTNK");
}
