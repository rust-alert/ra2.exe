//! AI 放置战车工厂并生产载具。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use crate::common::test_engine;
use ra_engine::{Session, MatchState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_places_war_factory_and_produces_tank() {
    let engine = test_engine();
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=HTNK\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAWEAP\n\
[HTNK]\nStrength=400\nSpeed=4\nSight=6\nCost=900\nArmor=heavy\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NAPOWR]\nStrength=600\nSight=4\nCost=600\nArmor=wood\n\
[NAWEAP]\nStrength=1000\nSight=5\nCost=2000\nArmor=wood\n",
    )
    .unwrap();
    let rules = RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-weap");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Soviets".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Soviets".into(),
        type_id: "NAPOWR".into(),
        health: 256,
        x: 9,
        y: 8,
        facing: 0,
        sub_cell: 0,
    });
    let mut world = MatchState::new(GameEdition::Ra2, &rules, map);
    assert!(world.set_house_funds("Soviets", 10_000));
    let mut session = Session::from_state(world, "ai-weap");
    session.expect_game_mut().ai_enabled = true;
    session.tick(&engine.runtime());
    assert!(session.expect_game_mut().world.entities.iter().any(|e| e.owner == "Soviets" && e.type_id == "NAWEAP"));
    session.tick(&engine.runtime());
    let weap = session.expect_game_mut().world.entities.iter().find(|e| e.owner == "Soviets" && e.type_id == "NAWEAP").expect("war factory");
    assert_eq!(weap.produce_queue.as_ref().map(|(id, _)| id.as_str()), Some("HTNK"));
}
