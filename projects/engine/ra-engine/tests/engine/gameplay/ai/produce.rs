//! AI 放置兵营并生产步兵。

use crate::common::test_engine;
use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{MatchState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_places_barracks_and_produces_infantry() {
    let engine = test_engine();
    let doc = IniDocument::parse(
        b"[InfantryTypes]\n0=E2\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAHAND\n\
[E2]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Soviets\nStrength=500\nSight=5\nCost=500\nArmor=wood\n",
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
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-barracks");
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
    let mut session = Session::from_state(world, "ai-barracks");
    session.expect_game_mut().ai_enabled = true;
    session.tick(&engine.runtime());
    assert!(
        session.expect_game_mut().world.find_entity_id_by_owner_type("Soviets", "NAHAND").is_some(),
        "AI should place barracks"
    );
    // 下一 tick 兵营空闲后排队生产。
    session.tick(&engine.runtime());
    let hand = session.expect_game().world.find_entity_id_by_owner_type("Soviets", "NAHAND").expect("barracks");
    assert_eq!(
        session.expect_game().world.ecs_produce_item(hand).expect("queue").as_ref().map(|(id, _)| id.as_ref()),
        Some("E2")
    );
}
