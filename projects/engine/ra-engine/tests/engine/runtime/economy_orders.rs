//! 会话层部署 / 建造 / 生产命令。

use crate::common::{test_engine, battle_from_defs, defs_from_rules_ini};
use ra_engine::{PRODUCE_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

fn economy_session() -> Session {
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAPILE\n\
[InfantryTypes]\n0=E1\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\nFoundation=4x4\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n";
    let defs = defs_from_rules_ini(rules_text);
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
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("Americans", 10_000));
    Session::from_state(world, "orders")
}

/// 排队建造并推进至建造场 `ready`（费用在排队时扣除）。
fn produce_until_ready(session: &mut Session, engine: &ra_engine::Engine, type_id: &str) {
    session.expect_battle_mut().order_produce(type_id);
    session.tick(&engine.runtime());
    assert!(
        session.expect_battle().world.last_rejects().is_empty(),
        "Produce {type_id} should start: {:?}",
        session.expect_battle().world.last_rejects()
    );
    for _ in 0..=PRODUCE_TICKS {
        if session.expect_battle().world.house_ready_building("Americans").is_some_and(|r| r.as_ref().eq_ignore_ascii_case(type_id)) {
            return;
        }
        session.tick(&engine.runtime());
    }
    panic!("expected {type_id} ready after {PRODUCE_TICKS} ticks");
}

#[test]
fn order_deploy_and_place_power() {
    let engine = test_engine();
    let mut session = economy_session();
    let mcv = session.expect_battle().world.entity_id_at(0).expect("entity");
    session.expect_battle_mut().order_deploy(&[mcv]);
    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().world.ecs_identity(session.expect_battle().world.entity_id_at(0).expect("entity")).expect("id").0.as_ref(),
        "GACNST"
    );
    assert_eq!(
        session.expect_battle().world.ecs_identity(session.expect_battle().world.entity_id_at(0).expect("entity")).expect("id").1,
        MapEntityKind::Structure
    );
    produce_until_ready(&mut session, &engine, "GAPOWR");
    assert_eq!(session.expect_battle().world.house_funds("Americans"), Some(10_000 - 600));
    // 建造场 `Foundation=4x4` 占 (4,4)–(7,7)，落在占地外。
    session.expect_battle_mut().order_place_building("GAPOWR", 8, 4);
    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.last_rejects().is_empty());
    assert_eq!(
        session.expect_battle().world.ecs_identity(session.expect_battle().world.entity_id_at(1).expect("entity")).expect("id").0.as_ref(),
        "GAPOWR"
    );
}

#[test]
fn pick_entity_at_finds_structure() {
    let engine = test_engine();
    let mut session = economy_session();
    let mcv = session.expect_battle().world.entity_id_at(0).expect("entity");
    session.expect_battle_mut().order_deploy(&[mcv]);
    session.tick(&engine.runtime());
    assert_eq!(session.expect_battle().pick_entity_at(4, 4), Some(EntityId(1)));
    assert_eq!(session.expect_battle().pick_structure_at(4, 4), Some(EntityId(1)));
    // `Foundation=4x4`：点占地内非锚点格也应命中，格外不命中。
    assert_eq!(session.expect_battle().pick_structure_at(7, 7), Some(EntityId(1)));
    assert_eq!(session.expect_battle().pick_structure_at(8, 8), None);
}

#[test]
fn order_rally_on_barracks() {
    let engine = test_engine();
    let mut session = economy_session();
    let mcv = session.expect_battle().world.entity_id_at(0).expect("entity");
    session.expect_battle_mut().order_deploy(&[mcv]);
    session.tick(&engine.runtime());
    produce_until_ready(&mut session, &engine, "GAPOWR");
    session.expect_battle_mut().order_place_building("GAPOWR", 8, 4);
    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.last_rejects().is_empty());
    produce_until_ready(&mut session, &engine, "GAPILE");
    session.expect_battle_mut().order_place_building("GAPILE", 9, 4);
    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.last_rejects().is_empty());
    let barracks = session.expect_battle().world.find_entity_id_by_type("GAPILE").expect("应有兵营");
    assert!(session.expect_battle().selection_has_structure(&[barracks]));
    session.expect_battle_mut().order_rally(&[barracks], 12, 8);
    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.last_rejects().is_empty());
    assert_eq!(session.expect_battle().world.ecs_rally(barracks).expect("rally").0, Some(12));
    assert_eq!(session.expect_battle().world.ecs_rally(barracks).expect("rally").1, Some(8));
}
