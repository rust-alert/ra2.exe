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

#[test]
fn ai_trigger_weighted_pick_one_team_per_house_tick() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // 两支 Always 触发同房主：权重悬殊时只应抽中一支（同 tick 不双刷）。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF_A\n1=TF_B\n\
[TF_A]\nName=A\n0=2,E1\nGroup=-1\n\
[TF_B]\nName=B\n0=5,E1\nGroup=-1\n\
[TeamTypes]\n0=TM_A\n1=TM_B\n\
[TM_A]\nName=A\nHouse=Russians\nScript=\nTaskForce=TF_A\nMax=1\n\
[TM_B]\nName=B\nHouse=Russians\nScript=\nTaskForce=TF_B\nMax=1\n\
[AITriggerTypes]\n\
AT_A=A,TM_A,Russians,0,-1,,0,9000\n\
AT_B=B,TM_B,Russians,0,-1,,0,1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-weight.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].weight, 9000);
    assert_eq!(map.scripting.ai_triggers[1].weight, 1);
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "ai-weight");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1 == 2 || e1 == 5, "weighted pick should fire exactly one team, got {e1}");
}

#[test]
fn ai_trigger_team2_also_enqueues() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // Team2 在 CSV 第 15 列（0-based 14）。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n1=TF2\n\
[TF1]\nName=One\n0=2,E1\nGroup=-1\n\
[TF2]\nName=Two\n0=3,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n1=TM2\n\
[TM1]\nName=T1\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[TM2]\nName=T2\nHouse=Russians\nScript=\nTaskForce=TF2\nMax=1\n\
[AITriggerTypes]\n\
AT1=Duo,TM1,Russians,0,-1,,0,50,0,0,1,0,0,0,TM2,1,1,1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-team2.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].team2.as_str(), "TM2");
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "ai-team2");
    let team2 = session.expect_battle().world.prepared.ai_triggers[0].team2;
    assert!(team2.is_some(), "bind must resolve Team2");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    // 同航点挤占时个别成员可能落空，但双队入队后应明显多于单队 2 人。
    assert!(e1 >= 4, "team1(2)+team2(3) should mostly spawn, got {e1}");
}

#[test]
fn ai_trigger_dynamic_weight_nudges_after_pick() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // Start=50 Min=1 Max=100；抽中后向 min 收敛，未抽中向 max 回升。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF_A\n1=TF_B\n\
[TF_A]\nName=A\n0=1,E1\nGroup=-1\n\
[TF_B]\nName=B\n0=1,E1\nGroup=-1\n\
[TeamTypes]\n0=TM_A\n1=TM_B\n\
[TM_A]\nName=A\nHouse=Russians\nScript=\nTaskForce=TF_A\nMax=8\n\
[TM_B]\nName=B\nHouse=Russians\nScript=\nTaskForce=TF_B\nMax=8\n\
[AITriggerTypes]\n\
AT_A=A,TM_A,Russians,0,-1,,0,50,1,100\n\
AT_B=B,TM_B,Russians,0,-1,,0,50,1,100\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-dyn-w.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].min_weight, 1);
    assert_eq!(map.scripting.ai_triggers[0].max_weight, 100);
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "ai-dyn-w");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    let ids: Vec<_> = session.expect_battle().world.prepared.ai_triggers.iter().map(|t| (t.id, t.weight)).collect();
    assert_eq!(ids.len(), 2);
    session.tick(&engine.runtime());
    let runtime = &session.expect_battle().world.ai_trigger_runtime;
    let w0 = runtime.current_weight(ids[0].0, ids[0].1);
    let w1 = runtime.current_weight(ids[1].0, ids[1].1);
    assert_ne!(w0, w1, "picked and unpicked weights should diverge after one tick");
    assert!(w0 == 25 || w1 == 25, "picked should average toward min (50+1)/2=25, got {w0}/{w1}");
    assert!(w0 == 75 || w1 == 75, "unpicked should average toward max (50+100)/2=75, got {w0}/{w1}");
}

#[test]
fn ai_trigger_own_credits_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // 条件 5：己方资金 >= 5000。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=Rich,TM1,Russians,0,5,,5000\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-own-cred.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].condition, ra_types::AiTriggerConditionKind::OwnCredits);
    assert_eq!(map.scripting.ai_triggers[0].compare_amount, 5000);
    let engine = test_engine();
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map.clone());
    assert!(world.set_house_funds("RUSSIANS", 1000));
    let mut session = Session::from_state(world, "ai-own-cred-low");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "OwnCredits must not fire when funds below threshold");

    let mut world2 = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world2.set_house_funds("RUSSIANS", 5000));
    let mut session2 = Session::from_state(world2, "ai-own-cred-ok");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "OwnCredits >=5000 should spawn, got {e1b}");
}

#[test]
fn ai_trigger_own_super_weapon_charge_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[SuperWeaponTypes]\n0=MultiSpecial\n\
[MultiSpecial]\nType=MultiMissile\nAction=MultiMissile\nRechargeTime=10\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // 条件 6：指定超武充能百分比 >= 80。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=NukeReady,TM1,Russians,0,6,MultiSpecial,80\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-sw-pct.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].condition, ra_types::AiTriggerConditionKind::OwnSuperWeaponCharge);
    assert_eq!(map.scripting.ai_triggers[0].condition_object.as_str(), "MULTISPECIAL");
    let engine = test_engine();
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map.clone());
    world.super_weapon_runtime.set_charge_for_test("RUSSIANS", ra_types::SuperWeaponName::parse("MultiSpecial"), 40, 100);
    let mut session = Session::from_state(world, "ai-sw-low");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "OwnSuperWeaponCharge must not fire below 80%");

    let mut world2 = battle_from_defs(GameEdition::Ra2, defs, map);
    world2.super_weapon_runtime.set_charge_for_test("RUSSIANS", ra_types::SuperWeaponName::parse("MultiSpecial"), 80, 100);
    let mut session2 = Session::from_state(world2, "ai-sw-ok");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "OwnSuperWeaponCharge >=80% should spawn, got {e1b}");
}

#[test]
fn ai_trigger_enemy_credits_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // 条件 4：敌方资金 >= 3000。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=EnemyCash,TM1,Russians,0,4,,3000\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-enemy-cred.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].condition, ra_types::AiTriggerConditionKind::EnemyCredits);
    let engine = test_engine();
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map.clone());
    world.ensure_house("AMERICANS");
    assert!(world.set_house_funds("AMERICANS", 1000));
    let mut session = Session::from_state(world, "ai-enemy-cred-low");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "EnemyCredits must not fire when enemy funds below threshold");

    let mut world2 = battle_from_defs(GameEdition::Ra2, defs, map);
    world2.ensure_house("AMERICANS");
    assert!(world2.set_house_funds("AMERICANS", 3000));
    let mut session2 = Session::from_state(world2, "ai-enemy-cred-ok");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "EnemyCredits >=3000 should spawn, got {e1b}");
}

#[test]
fn ai_trigger_enemy_yellow_power_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // 条件 2：敌方黄电（drain > output）。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=EnemyYellow,TM1,Russians,0,2\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-enemy-yellow.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].condition, ra_types::AiTriggerConditionKind::EnemyYellowPower);
    let engine = test_engine();
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map.clone());
    world.ensure_house("AMERICANS");
    if let Some(p) = world.players.iter_mut().find(|p| p.house.as_ref().eq_ignore_ascii_case("AMERICANS")) {
        p.power_output = 100;
        p.power_drain = 50;
    }
    let mut session = Session::from_state(world, "ai-enemy-yellow-ok-power");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "EnemyYellowPower must not fire when enemy has surplus power");

    let mut world2 = battle_from_defs(GameEdition::Ra2, defs, map);
    world2.ensure_house("AMERICANS");
    if let Some(p) = world2.players.iter_mut().find(|p| p.house.as_ref().eq_ignore_ascii_case("AMERICANS")) {
        p.power_output = 50;
        p.power_drain = 100;
    }
    let mut session2 = Session::from_state(world2, "ai-enemy-yellow-low");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "EnemyYellowPower should spawn when enemy is low power, got {e1b}");
}

#[test]
fn ai_trigger_enemy_red_power_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    );
    // 条件 3：敌方红电（有效供电为 0 且仍耗电）。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=EnemyRed,TM1,Russians,0,3\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-enemy-red.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].condition, ra_types::AiTriggerConditionKind::EnemyRedPower);
    let engine = test_engine();
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map.clone());
    world.ensure_house("AMERICANS");
    if let Some(p) = world.players.iter_mut().find(|p| p.house.as_ref().eq_ignore_ascii_case("AMERICANS")) {
        // 黄电：有供电但仍不足，不应满足红电。
        p.power_output = 50;
        p.power_drain = 100;
    }
    let mut session = Session::from_state(world, "ai-enemy-red-yellow-only");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "EnemyRedPower must not fire on yellow-only low power");

    let mut world2 = battle_from_defs(GameEdition::Ra2, defs, map);
    world2.ensure_house("AMERICANS");
    if let Some(p) = world2.players.iter_mut().find(|p| p.house.as_ref().eq_ignore_ascii_case("AMERICANS")) {
        p.power_output = 0;
        p.power_drain = 100;
    }
    let mut session2 = Session::from_state(world2, "ai-enemy-red-ok");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "EnemyRedPower should spawn when enemy effective power is zero, got {e1b}");
}

#[test]
fn ai_trigger_own_owns_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n1=NAPOWR\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NAPOWR]\nPower=200\nOwner=Russians\nStrength=600\nSight=4\nCost=600\nArmor=wood\n",
    );
    // ConditionType=1 OwnOwns NAPOWR>=1：仅有建造场时不应出队。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=NeedPower,TM1,Russians,0,1,NAPOWR,1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-own-owns.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].condition, ra_types::AiTriggerConditionKind::OwnOwns);
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "ai-own-owns");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "OwnOwns NAPOWR must not fire without own power plant");

    let text2 = b"\
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
AT1=NeedPower,TM1,Russians,0,1,NAPOWR,1\n\
";
    let map2 = MapInfo::parse_ini(GameEdition::Ra2, "ai-own-owns2.map", text2).unwrap();
    let mut session2 = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map2), "ai-own-owns2");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "OwnOwns NAPOWR>=1 should spawn team, got {e1b}");
}

#[test]
fn ai_trigger_neutral_owns_condition_gates_spawn() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=NACNST\n1=CAGATE\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[CAGATE]\nStrength=400\nSight=1\nCost=100\nArmor=concrete\nOwner=Neutral\n",
    );
    // ConditionType=7 NeutralOwns CAGATE>=1。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n0=Russians,NACNST,256,8,8,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=TechGate,TM1,Russians,0,7,CAGATE,1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-neutral.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers[0].condition, ra_types::AiTriggerConditionKind::NeutralOwns);
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "ai-neutral");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 0, "NeutralOwns CAGATE must not fire without ambient gate");

    let text2 = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Structures]\n\
0=Russians,NACNST,256,8,8,0\n\
1=Neutral,CAGATE,256,4,4,0\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=TechGate,TM1,Russians,0,7,CAGATE,1\n\
";
    let map2 = MapInfo::parse_ini(GameEdition::Ra2, "ai-neutral2.map", text2).unwrap();
    let mut session2 = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map2), "ai-neutral2");
    session2.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session2.tick(&engine.runtime());
    let e1b = session2.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert!(e1b >= 2, "NeutralOwns CAGATE>=1 should spawn team, got {e1b}");
}

#[test]
fn ai_trigger_all_house_expands_to_non_local_players() {
    let defs = defs_from_rules_ini(
        b"[Countries]\n0=Americans\n1=Russians\n\
[Americans]\nSide=GDI\n\
[Russians]\nSide=Nod\n\
[InfantryTypes]\n0=E1\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Americans,Russians\n",
    );
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=<all>\nScript=\nTaskForce=TF1\nMax=1\n\
[AITriggerTypes]\n\
AT1=Strike,TM1,<all>,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-all.map", text).unwrap();
    assert!(map.scripting.team_types[0].house.is_all_sentinel() || map.scripting.team_types[0].house.as_str().eq_ignore_ascii_case("<ALL>"));
    let engine = test_engine();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    world.ensure_house("AMERICANS");
    world.ensure_house("RUSSIANS");
    let _ = world.prefer_local_house("AMERICANS");
    let mut session = Session::from_state(world, "ai-all");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let russian_e1 = snap.units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead && u.owner.eq_ignore_ascii_case("RUSSIANS")).count();
    let american_e1 = snap.units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead && u.owner.eq_ignore_ascii_case("AMERICANS")).count();
    assert!(russian_e1 >= 2, "ALL AITrigger should spawn for non-local Russians, got {russian_e1}");
    assert_eq!(american_e1, 0, "ALL AITrigger must not spawn for local Americans");
}

#[test]
fn ai_trigger_prefers_higher_team_type_priority() {
    // 同房主两触发权重相同：只抽 Priority 更高的 TM_HI（3 个 E1），不抽 TM_LO（1 个）。
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[TaskForces]\n0=TF_LO\n1=TF_HI\n\
[TF_LO]\nName=Lo\n0=1,E1\nGroup=-1\n\
[TF_HI]\nName=Hi\n0=3,E1\nGroup=-1\n\
[TeamTypes]\n0=TM_LO\n1=TM_HI\n\
[TM_LO]\nName=Low\nHouse=Russians\nScript=\nTaskForce=TF_LO\nMax=1\nPriority=1\n\
[TM_HI]\nName=High\nHouse=Russians\nScript=\nTaskForce=TF_HI\nMax=1\nPriority=50\n\
[AITriggerTypes]\n\
AT_LO=LowPrio,TM_LO,Russians,0\n\
AT_HI=HighPrio,TM_HI,Russians,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai-prio.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "ai-prio");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let e1 = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1" && !u.dead).count();
    assert_eq!(e1, 3, "higher TeamType.Priority must win the house pick, got {e1}");
}
