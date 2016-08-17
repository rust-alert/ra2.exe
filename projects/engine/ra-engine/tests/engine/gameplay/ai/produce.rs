//! AI 放置兵营并生产步兵。

use crate::common::{test_engine, battle_from_rules};
use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{PRODUCE_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_places_barracks_and_produces_infantry() {
    let engine = test_engine();
    let doc = IniDocument::parse(
        b"[InfantryTypes]\n0=E2\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAHAND\n\
[E2]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Soviets\nStrength=500\nSight=5\nCost=500\nArmor=wood\nTechLevel=1\n",
    )
    .unwrap();
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&doc),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
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
    let mut session = Session::from_state(world, "ai-barracks");
    session.expect_battle_mut().ai_enabled = true;
    for _ in 0..(PRODUCE_TICKS + 4) {
        session.tick(&engine.runtime());
        if session.expect_battle().world.find_entity_id_by_owner_type("Soviets", "NAHAND").is_some() {
            break;
        }
    }
    assert!(session.expect_battle_mut().world.find_entity_id_by_owner_type("Soviets", "NAHAND").is_some(), "AI should place barracks");
    // 下一 tick 兵营空闲后排队生产。
    session.tick(&engine.runtime());
    let hand = session.expect_battle().world.find_entity_id_by_owner_type("Soviets", "NAHAND").expect("barracks");
    assert_eq!(session.expect_battle().world.ecs_produce_item(hand).expect("queue").as_ref().map(|(id, _)| id.as_ref()), Some("E2"));
}

#[test]
fn ai_skips_dog_and_naval_when_picking_produce() {
    let engine = test_engine();
    // 字母序上 ADOG / DEST 会排在 E1 / MTNK 前面；过滤后应造陆地基础单位。
    let doc = IniDocument::parse(
        b"[InfantryTypes]\n0=ADOG\n1=E1\n\
[VehicleTypes]\n0=DEST\n1=MTNK\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=GAPOWR\n3=GAPILE\n4=GAWEAP\n\
[ADOG]\nStrength=100\nSpeed=8\nSight=5\nCost=200\nArmor=none\nTechLevel=1\nCategory=Dog\nOwner=Americans,Alliance\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nTechLevel=1\nCategory=Soldier\nOwner=Americans,Alliance\n\
[DEST]\nStrength=500\nSpeed=6\nSight=6\nCost=1000\nArmor=heavy\nTechLevel=1\nNaval=yes\nOwner=Americans,Alliance\n\
[MTNK]\nStrength=300\nSpeed=6\nSight=6\nCost=700\nArmor=heavy\nTechLevel=1\nOwner=Americans,Alliance\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nArmor=wood\nTechLevel=1\n\
[GAWEAP]\nPower=-30\nPowered=yes\nFactory=UnitType\nOwner=Americans\nStrength=1000\nSight=5\nCost=2000\nArmor=wood\nTechLevel=1\n",
    )
    .unwrap();
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&doc),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-filter");
    map.width = 20;
    map.height = 20;
    // 本地苏联，对手美军吃 AI。
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Soviets".into(),
        type_id: "NACNST".into(),
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
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 11,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPILE".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAWEAP".into(),
        health: 256,
        x: 13,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = battle_from_rules(&rules, map);
    assert!(world.set_house_funds("Americans", 20_000));
    assert!(world.prefer_local_house("Soviets"));
    let mut session = Session::from_state(world, "ai-filter");
    session.expect_battle_mut().ai_enabled = true;
    session.tick(&engine.runtime());
    let pile = session.expect_battle().world.find_entity_id_by_owner_type("Americans", "GAPILE").expect("barracks");
    let weap = session.expect_battle().world.find_entity_id_by_owner_type("Americans", "GAWEAP").expect("war factory");
    assert_eq!(
        session.expect_battle().world.ecs_produce_item(pile).expect("inf queue").as_ref().map(|(id, _)| id.as_ref()),
        Some("E1"),
        "AI must not mass-produce ADOG"
    );
    assert_eq!(
        session.expect_battle().world.ecs_produce_item(weap).expect("veh queue").as_ref().map(|(id, _)| id.as_ref()),
        Some("MTNK"),
        "AI must not mass-produce DEST from land war factory"
    );
}
