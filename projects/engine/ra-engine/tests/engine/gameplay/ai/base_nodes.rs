//! 地图 `[Base]` 节点驱动 AI 选型与落点。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{PRODUCE_TICKS, Session};
use ra_map::{MapBaseNode, MapBasePlan, MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, HouseName, TechnoName};

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

fn structure_xy(session: &Session, owner: &str, type_id: &str) -> Option<(u16, u16)> {
    let world = &session.expect_battle().world;
    world.entity_ids().into_iter().find_map(|id| {
        if !world.ecs_owner(id).is_some_and(|h| h.eq_ignore_ascii_case(owner)) {
            return None;
        }
        if !world.ecs_identity(id).is_some_and(|(key, _)| key.eq_ignore_ascii_case(type_id)) {
            return None;
        }
        world.ecs_transform(id).map(|(x, y, _)| (x, y))
    })
}

#[test]
fn ai_follows_base_node_type_and_cell() {
    let engine = test_engine();
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n4=NAHAND\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Soviets\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Soviets\nStrength=600\nSight=4\nCost=500\nArmor=wood\nTechLevel=1\n\
[AI]\nAIBaseSpacing=0\nPowerSurplus=50\nBaseSizeAdd=0\n\
BuildPower=NAPOWR\nBuildRefinery=NAREFN\nRefineryLimit=1\nBuildBarracks=NAHAND\nBarracksLimit=1\n\
[IQ]\nMaxIQLevels=5\nProduction=0\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-base-nodes");
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
        x: 11,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    // AiControls 会先造矿场；Base 节点要求先造兵营并落在指定格。
    map.scripting.base = Some(MapBasePlan {
        player: HouseName::parse("Soviets"),
        count: Some(1),
        nodes: vec![MapBaseNode { type_name: TechnoName::parse("NAHAND"), x: 12, y: 10 }],
    });

    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.prepared.base_plan.is_some(), "Base plan must bind");
    assert!(world.set_house_funds("SOVIETS", 10_000));
    let mut session = Session::from_state(world, "ai-base-nodes");
    session.expect_battle_mut().ai_enabled = true;

    for _ in 0..(PRODUCE_TICKS + 12) {
        session.tick(&engine.runtime());
        if count_owner_type(&session, "SOVIETS", "NAHAND") >= 1 {
            break;
        }
        assert_eq!(count_owner_type(&session, "SOVIETS", "NAREFN"), 0, "refinery must not preempt Base node");
    }
    assert_eq!(count_owner_type(&session, "SOVIETS", "NAHAND"), 1);
    assert_eq!(structure_xy(&session, "SOVIETS", "NAHAND"), Some((12, 10)));
}
