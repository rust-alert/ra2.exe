//! AI 基建顺序由 rules `[AI]` 候选表驱动：低电补电厂；矿场先于兵营；空表 / 低 IQ 不造工厂类。

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

fn soviet_yard_map(name: &str) -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, name);
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
    map
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
    let mut map = soviet_yard_map("ai-lowpow");
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
    let mut map = soviet_yard_map("ai-order");
    map.width = 20;
    map.height = 20;
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

    // 第一座应是矿场，而不是更便宜的兵营（表序：Refinery 先于 Barracks）。
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

#[test]
fn ai_skips_barracks_when_build_barracks_table_empty() {
    let engine = test_engine();
    // 显式 `[AI]`：有矿场表、无兵营表；类型仍入库，但候选为空则不得下单。
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n4=NAHAND\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Soviets\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Soviets\nStrength=600\nSight=4\nCost=500\nArmor=wood\nTechLevel=1\n\
[AI]\nAIBaseSpacing=1\nPowerSurplus=50\nBuildPower=NAPOWR\nBuildRefinery=NAREFN\nRefineryLimit=1\n\
[IQ]\nMaxIQLevels=5\nProduction=0\n",
    );
    let mut map = soviet_yard_map("ai-no-barracks-table");
    map.width = 20;
    map.height = 20;
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
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NAREFN".into(),
        health: 256,
        x: 8,
        y: 11,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("SOVIETS", 10_000));
    let mut session = Session::from_state(world, "ai-no-barracks-table");
    session.expect_battle_mut().ai_enabled = true;
    for _ in 0..(PRODUCE_TICKS + 8) {
        session.tick(&engine.runtime());
    }
    assert!(
        session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAHAND").is_none(),
        "empty BuildBarracks must not queue NAHAND"
    );
}

#[test]
fn ai_skips_factory_categories_when_iq_below_production() {
    let engine = test_engine();
    // `Production=5`：无地图 House IQ 时回落到 MaxIQLevels=0 → 不得造兵营。
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n4=NAHAND\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Soviets\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Soviets\nStrength=600\nSight=4\nCost=500\nArmor=wood\nTechLevel=1\n\
[AI]\nAIBaseSpacing=1\nPowerSurplus=50\nBuildPower=NAPOWR\nBuildRefinery=NAREFN\nRefineryLimit=1\n\
BuildBarracks=NAHAND\nBarracksLimit=1\n\
[IQ]\nMaxIQLevels=0\nProduction=5\n",
    );
    let mut map = soviet_yard_map("ai-low-iq");
    map.width = 20;
    map.height = 20;
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
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NAREFN".into(),
        health: 256,
        x: 8,
        y: 11,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("SOVIETS", 10_000));
    assert_eq!(world.definitions.ai_controls.iq_production, 5);
    assert_eq!(world.definitions.ai_controls.max_iq_levels, 0);
    let mut session = Session::from_state(world, "ai-low-iq");
    session.expect_battle_mut().ai_enabled = true;
    for _ in 0..(PRODUCE_TICKS + 8) {
        session.tick(&engine.runtime());
    }
    assert!(
        session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAHAND").is_none(),
        "IQ below Production must not expand barracks"
    );
}
