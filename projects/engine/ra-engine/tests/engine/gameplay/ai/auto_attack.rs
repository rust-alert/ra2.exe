//! AI 经 GameCommand 自动攻击。

use crate::common::{test_engine, battle_from_defs, defs_from_rules_ini};
use ra_engine::Session;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition};

fn duel_session() -> Session {
    let defs = defs_from_rules_ini(b"[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[Gun]\nDamage=40\nROF=2\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",);
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-duel");
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
        x: 14,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 6,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "ai");
    session.expect_battle_mut().ai_enabled = true;
    session
}

#[test]
fn ai_issues_attack_via_commands() {
    let engine = test_engine();
    let mut session = duel_session();
    let ally = session.expect_battle().world.find_entity_id_by_owner_type("Americans", "MTNK").expect("ally tank");
    let enemy = session.expect_battle().world.find_entity_id_by_owner_type("Soviets", "MTNK").expect("enemy tank");
    let before = session.expect_battle().world.ecs_health(ally).expect("health").0;
    for _ in 0..30 {
        session.tick(&engine.runtime());
    }
    let game = session.expect_battle();
    assert!(game.world.ecs_attack_state(enemy).expect("atk").0 == Some(ally) || game.world.ecs_health(ally).expect("health").0 < before);
    assert!(game.world.ecs_health(ally).expect("health").0 < before);
}

#[test]
fn ambient_house_does_not_auto_attack() {
    let defs = defs_from_rules_ini(b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\n\
[Gun]\nDamage=40\nROF=2\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",);
    let mut map = MapInfo::empty(GameEdition::Ra2, "ambient-ai");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Neutral".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 6,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "ambient");
    session.expect_battle_mut().ai_enabled = true;
    let engine = test_engine();
    let ally = session.expect_battle().world.entity_id_at(0).expect("entity");
    let ambient = session.expect_battle().world.entity_id_at(1).expect("entity");
    let before = session.expect_battle().world.ecs_health(ally).expect("health").0;
    for _ in 0..30 {
        session.tick(&engine.runtime());
    }
    let game = session.expect_battle();
    assert_eq!(game.world.ecs_attack_state(ambient).expect("atk").0, None);
    assert_eq!(game.world.ecs_health(ally).expect("health").0, before);
}

#[test]
fn guard_mission_skips_ai_auto_attack() {
    let defs = defs_from_rules_ini(b"[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[Gun]\nDamage=40\nROF=2\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",);
    let mut map = MapInfo::empty(GameEdition::Ra2, "guard-ai");
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
        x: 14,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: "Guard".into(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 6,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "guard");
    session.expect_battle_mut().ai_enabled = true;
    let engine = test_engine();
    let guard = session.expect_battle().world.find_entity_id_by_owner_type("Americans", "MTNK").expect("guard tank");
    assert_eq!(session.expect_battle().world.ecs_mission(guard).as_deref(), Some("Guard"));
    for _ in 0..30 {
        session.tick(&engine.runtime());
    }
    assert_eq!(session.expect_battle().world.ecs_attack_state(guard).expect("atk").0, None, "Guard mission must not AI auto-attack");
}
