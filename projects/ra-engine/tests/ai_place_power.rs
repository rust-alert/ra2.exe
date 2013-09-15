//! AI 经 PlaceBuilding 放置电厂。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_engine::Session;
use ra_types::GameEdition;
use ra_engine::World;

#[test]
fn ai_places_power_near_yard() {
    let doc = IniDocument::parse(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NAPOWR]\nStrength=600\nSight=4\nCost=600\nArmor=wood\n",
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
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-power");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 2,
        y: 2,
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
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    assert!(world.set_house_funds("Soviets", 10_000));
    let mut session = Session::new(world, "ai-power");
    session.ai_enabled = true;
    session.tick();
    let power = session
        .world
        .entities
        .iter()
        .find(|e| e.owner == "Soviets" && e.type_id == "NAPOWR");
    assert!(power.is_some(), "AI should place NAPOWR");
    let p = power.unwrap();
    let dist = (i32::from(p.x) - 8).unsigned_abs() + (i32::from(p.y) - 8).unsigned_abs();
    assert!(dist >= 1 && dist <= 3);
    assert_eq!(session.world.house_funds("Soviets"), Some(10_000 - 600));
}
