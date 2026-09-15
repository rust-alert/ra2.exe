//! AI 基建单票顺序：电 → 矿 → 兵营（有矿类型时不抢先造兵营）；低电补电厂。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{PRODUCE_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

fn count_owner_type(session: &Session, owner: &str, type_id: &str) -> usize {
    let world = &session.expect_battle().world;
    world
        .entity_ids()
        .into_iter()
        .filter(|&id| {
            world.ecs_owner(id).is_some_and(|h| h.eq_ignore_ascii_case(owner))
                && world.ecs_identity(id).is_some_and(|(key, _)| key.eq_ignore_ascii_case(type_id))
        })
        .count()
}

#[test]
fn ai_queues_extra_power_when_low_power_before_refinery() {
    let engine = test_engine();
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n4=NADRAIN\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Soviets\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\n\
[NADRAIN]\nPower=-300\nOwner=Soviets\nStrength=100\nSight=1\nCost=1\nArmor=wood\nTechLevel=1\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-lowpow");
    map.width = 24;
    map.height = 24;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NAPOWR".into(),
        health: 256,
        x: 12,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    // 高耗电建筑制造低电：单座电厂不够。
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NADRAIN".into(),
        health: 256,
        x: 8,
        y: 12,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("SOVIETS", 10_000));
    let soviets = world.players.iter().find(|p| p.house.eq_ignore_ascii_case("SOVIETS")).expect("soviets");
    assert!(soviets.low_power(), "seed should be low-powered (output={} drain={})", soviets.power_output, soviets.power_drain);
    let mut session = Session::from_state(world, "ai-lowpow");
    session.expect_battle_mut().ai_enabled = true;

    // 低电下生产约半速（奇数 tick 停表），需留够两倍 `PRODUCE_TICKS`。
    for _ in 0..(PRODUCE_TICKS * 2 + 16) {
        session.tick(&engine.runtime());
        if count_owner_type(&session, "SOVIETS", "NAPOWR") >= 2 {
            break;
        }
        assert!(
            session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAREFN").is_none(),
            "refinery must wait until low power is addressed"
        );
    }
    assert!(count_owner_type(&session, "SOVIETS", "NAPOWR") >= 2, "AI should place a second power plant while low-powered");
}

#[test]
fn ai_builds_refinery_before_barracks_when_both_missing() {
    let engine = test_engine();
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n4=NAHAND\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Soviets\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Soviets\nStrength=600\nSight=4\nCost=500\nArmor=wood\nTechLevel=1\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-order");
    map.width = 20;
    map.height = 20;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NAPOWR".into(),
        health: 256,
        x: 10,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("SOVIETS", 10_000));
    let mut session = Session::from_state(world, "ai-order");
    session.expect_battle_mut().ai_enabled = true;

    // 第一座应是矿场，而不是更便宜的兵营。
    for _ in 0..(PRODUCE_TICKS + 6) {
        session.tick(&engine.runtime());
        if session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAREFN").is_some() {
            break;
        }
        assert!(
            session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAHAND").is_none(),
            "barracks must not appear before refinery"
        );
    }
    assert!(session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAREFN").is_some());
    assert!(session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAHAND").is_none());
}
