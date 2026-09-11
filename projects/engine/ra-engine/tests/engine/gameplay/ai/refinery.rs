//! AI 放置矿场。

use crate::common::{test_engine, battle_from_rules};
use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{PRODUCE_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_places_refinery_near_yard() {
    let engine = test_engine();
    let doc = IniDocument::parse(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Soviets\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\n",
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
        super_weapons: SuperWeaponTypeRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-refn");
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
    let mut session = Session::from_state(world, "ai-refn");
    session.expect_battle_mut().ai_enabled = true;
    for _ in 0..(PRODUCE_TICKS + 4) {
        session.tick(&engine.runtime());
        if session.expect_battle().world.find_entity_id_by_owner_type("Soviets", "NAREFN").is_some() {
            break;
        }
    }
    let refn = session.expect_battle_mut().world.find_entity_id_by_owner_type("Soviets", "NAREFN");
    assert!(refn.is_some(), "AI should place NAREFN");
    assert_eq!(session.expect_battle_mut().world.house_funds("Soviets"), Some(10_000 - 2000));
}
