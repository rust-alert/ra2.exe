//! 会话层部署 / 建造 / 生产命令。

use crate::common::test_engine;
use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{MatchState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

fn economy_session() -> Session {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAPILE\n\
[InfantryTypes]\n0=E1\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=4\nSight=5\nCost=200\n";
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
    let mut map = MapInfo::empty(GameEdition::Ra2, "orders");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "AMCV".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
    }];
    let mut world = MatchState::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    Session::from_state(world, "orders")
}

#[test]
fn order_deploy_and_place_power() {
    let engine = test_engine();
    let mut session = economy_session();
    let mcv = session.expect_game().world.entities[0].id;
    session.expect_game_mut().order_deploy(&[mcv]);
    session.tick(&engine.runtime());
    assert_eq!(session.expect_game().world.entities[0].type_id.as_ref(), "GACNST");
    assert_eq!(session.expect_game().world.entities[0].kind, MapEntityKind::Structure);
    session.expect_game_mut().order_place_building("GAPOWR", 6, 4);
    session.tick(&engine.runtime());
    assert!(session.expect_game().world.last_rejects().is_empty());
    assert_eq!(session.expect_game().world.entities[1].type_id.as_ref(), "GAPOWR");
    assert_eq!(session.expect_game().world.house_funds("Americans"), Some(10_000 - 600));
}

#[test]
fn pick_entity_at_finds_structure() {
    let engine = test_engine();
    let mut session = economy_session();
    let mcv = session.expect_game().world.entities[0].id;
    session.expect_game_mut().order_deploy(&[mcv]);
    session.tick(&engine.runtime());
    assert_eq!(session.expect_game().pick_entity_at(4, 4), Some(EntityId(1)));
    assert_eq!(session.expect_game().pick_structure_at(4, 4), Some(EntityId(1)));
}

#[test]
fn order_rally_on_barracks() {
    let engine = test_engine();
    let mut session = economy_session();
    let mcv = session.expect_game().world.entities[0].id;
    session.expect_game_mut().order_deploy(&[mcv]);
    session.tick(&engine.runtime());
    session.expect_game_mut().order_place_building("GAPOWR", 6, 4);
    session.tick(&engine.runtime());
    session.expect_game_mut().order_place_building("GAPILE", 8, 4);
    session.tick(&engine.runtime());
    let barracks = session
        .expect_game()
        .world
        .entities
        .iter()
        .find(|e| e.type_id.as_ref() == "GAPILE")
        .map(|e| e.id)
        .expect("应有兵营");
    assert!(session.expect_game().selection_has_structure(&[barracks]));
    session.expect_game_mut().order_rally(&[barracks], 12, 8);
    session.tick(&engine.runtime());
    assert!(session.expect_game().world.last_rejects().is_empty());
    let barracks_i = session.expect_game().world.entity_index(barracks).unwrap();
    assert_eq!(session.expect_game().world.entities[barracks_i].rally_x, Some(12));
    assert_eq!(session.expect_game().world.entities[barracks_i].rally_y, Some(8));
}
