//! AI 经 Deploy 展开 MCV。

use crate::common::test_engine;
use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_deploys_mcv_via_command() {
    let engine = test_engine();
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=SMCV\n\
[BuildingTypes]\n0=NACNST\n1=GACNST\n\
[SMCV]\nDeploysInto=NACNST\nOwner=Soviets\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nFoundation=4x4\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
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
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-deploy");
    map.width = 16;
    map.height = 16;
    // 本地玩家先占 PlayerId(0)；AI 控制 Soviets。
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
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "SMCV".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "ai-deploy");
    session.expect_battle_mut().ai_enabled = true;
    session.tick(&engine.runtime());
    let mcv = session.expect_battle().world.entity_id_at(1).expect("entity");
    let identity = session.expect_battle().world.ecs_identity(mcv).expect("id");
    assert_eq!(identity.1, MapEntityKind::Structure);
    assert_eq!(identity.0.as_ref(), "NACNST");
    let world = &session.expect_battle().world;
    assert!(!world.pass_grid.is_passable(8, 8));
    assert!(!world.pass_grid.is_passable(11, 11), "Deploy must seal full Foundation");
    assert!(world.pass_grid.is_passable(12, 12));
    let paint = session.expect_battle_mut().world.take_structure_paint_dirty();
    assert!(paint.contains(&mcv), "Deploy must dirty structure paint so yards appear on preview");
}
