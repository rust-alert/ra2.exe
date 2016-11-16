//! AI 经 Deploy 展开 MCV。

use crate::common::{test_engine, battle_from_defs, defs_from_rules_ini};
use ra_engine::Session;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition};

#[test]
fn ai_deploys_mcv_via_command() {
    let engine = test_engine();
    let defs = defs_from_rules_ini(b"[VehicleTypes]\n0=SMCV\n\
[BuildingTypes]\n0=NACNST\n1=GACNST\n\
[SMCV]\nDeploysInto=NACNST\nOwner=Soviets\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nFoundation=4x4\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",);
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
        tag: Default::default(),
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
        tag: Default::default(),
    });
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "ai-deploy");
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
    let paint = session.expect_battle_mut().world.take_structure_buildup_dirty();
    assert!(paint.contains(&mcv), "Deploy must dirty structure buildup so yards appear on preview");
}
