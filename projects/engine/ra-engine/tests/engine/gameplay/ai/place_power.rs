//! AI 经 PlaceBuilding 放置电厂。

use crate::common::test_engine;
use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, PRODUCE_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_places_power_near_yard() {
    let engine = test_engine();
    let doc = IniDocument::parse(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\nFoundation=4x4\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\nFoundation=2x2\n",
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
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-power");
    map.width = 24;
    map.height = 24;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 2,
        y: 2,
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
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    // 模拟建造场已封满 4x4：旧 AI 只在半径 2 内查 1x1，会选到无法放下 2x2 的邻格。
    world.seal_structure_footprint(8, 8, 4, 4);
    assert!(world.set_house_funds("Soviets", 10_000));
    let mut session = Session::from_state(world, "ai-power");
    session.expect_battle_mut().ai_enabled = true;
    for _ in 0..(PRODUCE_TICKS + 4) {
        session.tick(&engine.runtime());
        if session.expect_battle().world.find_entity_id_by_owner_type("Soviets", "NAPOWR").is_some() {
            break;
        }
    }
    let power = session.expect_battle_mut().world.find_entity_id_by_owner_type("Soviets", "NAPOWR");
    assert!(power.is_some(), "AI should place NAPOWR with 2x2 footprint outside sealed yard");
    let power_id = power.unwrap();
    let (px, py, _) = session.expect_battle().world.ecs_transform(power_id).expect("xf");
    // 2x2 不得与建造场 4x4 [8..12)×[8..12) 重叠。
    assert!(px + 1 < 8 || px >= 12 || py + 1 < 8 || py >= 12);
    assert_eq!(session.expect_battle_mut().world.house_funds("Soviets"), Some(10_000 - 600));
    let paint = session.expect_battle_mut().world.take_structure_buildup_dirty();
    assert!(paint.contains(&power_id), "AI PlaceBuilding must dirty structure buildup");
}
