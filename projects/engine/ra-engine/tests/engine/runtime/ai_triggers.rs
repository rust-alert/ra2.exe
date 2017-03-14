//! AITriggerTypes 最小执行：按冷却排队产队。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{Session, SessionBootKind};
use ra_map::MapInfo;
use ra_types::GameEdition;

fn defs_with_e1() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n",
    )
}

#[test]
fn ai_trigger_spawns_team_on_campaign_tick() {
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=Strike,TM1,Russians,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers.len(), 1);
    assert!(ra_map::campaign_blocking_capability_message(&map).is_none());

    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "ai");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    assert!(session.expect_battle().world.ai_trigger_runtime.enabled);

    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1 >= 2, "AITrigger should enqueue Create Team on first tick, got {e1}");
}

#[test]
fn ai_trigger_house_skips_heuristic_factory_produce() {
    // 有 AITrigger 覆盖 Russians 时，启发式不得再造兵营量产（部队只走产队）。
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n1=NAPOWR\n2=NAHAND\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nTechLevel=1\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Russians\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Russians\nStrength=500\nSight=5\nCost=500\nArmor=wood\nTechLevel=1\n",
    );
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n\
0=Russians,NACNST,256,8,8,0\n\
1=Russians,NAPOWR,256,9,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=Strike,TM1,Russians,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-no-heuristic.map", text).unwrap();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("RUSSIANS", 10_000));
    // 本地玩家挂 Americans，使 Russians 走对手 AI 路径。
    world.ensure_house("AMERICANS");
    if let Some(p) = world.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case("AMERICANS")) {
        world.local_player = p.id;
    }
    let mut session = Session::from_state(world, "ai-no-heuristic");
    session.expect_battle_mut().ai_enabled = true;
    let engine = test_engine();
    for _ in 0..20 {
        session.tick(&engine.runtime());
    }
    assert!(
        session.expect_battle().world.find_entity_id_by_owner_type("RUSSIANS", "NAHAND").is_none(),
        "AITrigger house must not heuristic-place barracks"
    );
}

#[test]
fn ai_trigger_enemy_owns_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n1=GACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n\
0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=Strike,TM1,Russians,0,0,GACNST,1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-cond.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "ai-cond");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "EnemyOwns GACNST must not fire without enemy yard");

    let text2 = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n\
0=Russians,NACNST,256,8,8,0\n\
1=Americans,GACNST,256,1,1,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=Strike,TM1,Russians,0,0,GACNST,1\n\
";
    let map2 = MapInfo::parse_ini(GameEdition::Ra2, "ai-cond2.map", text2).unwrap();
    let mut session2 = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map2), "ai-cond2");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "EnemyOwns GACNST>=1 should spawn team, got {e1b}");
}
