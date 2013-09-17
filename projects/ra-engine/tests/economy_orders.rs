//! 会话层部署 / 建造 / 生产命令。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{Session, World};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

fn economy_session() -> Session {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAPILE\n\
[InfantryTypes]\n0=E1\n\
[AMCV]\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\n\
[GAPOWR]\nStrength=600\nSight=4\nCost=600\n\
[GAPILE]\nStrength=500\nSight=5\nCost=500\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\n";
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
    let mut world = World::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    Session::new(world, "orders")
}

#[test]
fn order_selected_deploy_and_place_power() {
    let mut session = economy_session();
    session.select_only(0);
    session.order_selected_deploy();
    session.tick();
    assert_eq!(session.world.entities[0].type_id, "GACNST");
    assert_eq!(session.world.entities[0].kind, MapEntityKind::Structure);
    session.order_place_building("GAPOWR", 6, 4);
    session.tick();
    assert!(session.world.last_rejects().is_empty());
    assert_eq!(session.world.entities[1].type_id, "GAPOWR");
    assert_eq!(session.world.house_funds("Americans"), Some(10_000 - 600));
}

#[test]
fn pick_entity_at_finds_structure() {
    let mut session = economy_session();
    session.select_only(0);
    session.order_selected_deploy();
    session.tick();
    assert_eq!(session.pick_entity_at(4, 4), Some(0));
    assert_eq!(session.pick_structure_at(4, 4), Some(0));
}

#[test]
fn order_selected_rally_on_barracks() {
    let mut session = economy_session();
    session.select_only(0);
    session.order_selected_deploy();
    session.tick();
    session.order_place_building("GAPOWR", 6, 4);
    session.tick();
    session.order_place_building("GAPILE", 8, 4);
    session.tick();
    let barracks = session.world.entities.iter().position(|e| e.type_id == "GAPILE").expect("应有兵营");
    session.select_only(barracks);
    assert!(session.selection_has_structure());
    session.order_selected_rally(12, 8);
    session.tick();
    assert!(session.world.last_rejects().is_empty());
    assert_eq!(session.world.entities[barracks].rally_x, Some(12));
    assert_eq!(session.world.entities[barracks].rally_y, Some(8));
}
