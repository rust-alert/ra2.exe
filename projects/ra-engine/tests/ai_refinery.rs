//! AI 放置矿场。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{Session, World};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_places_refinery_near_yard() {
    let doc = IniDocument::parse(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NAPOWR]\nStrength=600\nSight=4\nCost=600\nArmor=wood\n\
[NAREFN]\nStrength=900\nSight=4\nCost=2000\nArmor=wood\n",
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
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    assert!(world.set_house_funds("Soviets", 10_000));
    let mut session = Session::new(world, "ai-refn");
    session.ai_enabled = true;
    session.tick();
    let refn = session.world.entities.iter().find(|e| e.owner == "Soviets" && e.type_id == "NAREFN");
    assert!(refn.is_some(), "AI should place NAREFN");
    assert_eq!(session.world.house_funds("Soviets"), Some(10_000 - 2000));
}
