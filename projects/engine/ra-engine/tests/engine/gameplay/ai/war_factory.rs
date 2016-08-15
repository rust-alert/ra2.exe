//! AI 放置战车工厂并生产载具。

use crate::common::{test_engine, battle_from_rules};
use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{PRODUCE_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_places_war_factory_and_produces_tank() {
    let engine = test_engine();
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=HTNK\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAWEAP\n\
[HTNK]\nStrength=400\nSpeed=4\nSight=6\nCost=900\nArmor=heavy\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAWEAP]\nPower=-30\nPowered=yes\nFactory=UnitType\nOwner=Soviets\nStrength=1000\nSight=5\nCost=2000\nArmor=wood\nTechLevel=1\n",
    )
    .unwrap();
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
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
        mission: String::new(),
        tag: String::new(),
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
        mission: String::new(),
        tag: String::new(),
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
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = battle_from_rules(&rules, map);
    assert!(world.set_house_funds("Soviets", 10_000));
    let mut session = Session::from_state(world, "ai-weap");
    session.expect_battle_mut().ai_enabled = true;
    for _ in 0..(PRODUCE_TICKS + 4) {
        session.tick(&engine.runtime());
        if session.expect_battle().world.find_entity_id_by_owner_type("Soviets", "NAWEAP").is_some() {
            break;
        }
    }
    assert!(session.expect_battle_mut().world.find_entity_id_by_owner_type("Soviets", "NAWEAP").is_some());
    session.tick(&engine.runtime());
    let weap = session.expect_battle().world.find_entity_id_by_owner_type("Soviets", "NAWEAP").expect("war factory");
    assert_eq!(session.expect_battle().world.ecs_produce_item(weap).expect("queue").as_ref().map(|(id, _)| id.as_ref()), Some("HTNK"));
}
