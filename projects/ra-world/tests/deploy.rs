//! MCV 部署与资金播种。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};
use ra_world::{CommandRejectReason, GameCommand, World};

fn mcv_world() -> World {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n1=MTNK\n\
[BuildingTypes]\n0=GACNST\n\
[AMCV]\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\n";
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
    }];
    World::new(GameEdition::Ra2, &rules_db, map)
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
fn deploy_mcv_becomes_construction_yard() {
    let mut world = mcv_world();
    let id = world.entities[0].id;
    world.push_command(GameCommand::Deploy { entity_index: 0 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.entities[0].id, id);
    assert_eq!(world.entities[0].id, EntityId(1));
    assert_eq!(world.entities[0].kind, MapEntityKind::Structure);
    assert_eq!(world.entities[0].type_id, "GACNST");
    assert_eq!(world.entities[0].speed, 0);
    assert!(world.entities[0].attack_target.is_none());
}

#[test]
fn deploy_rejects_non_mcv_unit() {
    let mut world = mcv_world();
    world.entities[0].type_id = "MTNK".into();
    world.push_command(GameCommand::Deploy { entity_index: 0 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::CannotDeploy);
    assert_eq!(world.entities[0].kind, MapEntityKind::Unit);
    assert_eq!(world.entities[0].type_id, "MTNK");
}
