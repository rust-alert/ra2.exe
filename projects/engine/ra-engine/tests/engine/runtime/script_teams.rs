//! Create Team 动作可生成 TaskForce 单位。

use crate::common::{test_engine, battle_from_defs, defs_from_rules_ini};
use ra_engine::{Session, SessionBootKind};
use ra_map::MapInfo;
use ra_types::GameEdition;

fn defs_with_e1() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n",)
}

#[test]
fn create_team_action_spawns_task_force() {
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Triggers]\nTR1=Russians,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=SC1\nTaskForce=TF1\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "team.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "team");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let e1 = snap.units.iter().filter(|u| u.type_id.as_ref() == "E1").count();
    assert!(e1 >= 2, "expected reinforced E1 units, got {e1} in {:?}", snap.units.iter().map(|u| u.type_id.as_ref()).collect::<Vec<_>>());
}

#[test]
fn create_team_spawns_at_team_type_waypoint() {
    // Waypoint 2 = (12,8); TeamType.Waypoint=2 must spawn there (not default WP0).
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n2=8012\n\
[Triggers]\nTR1=Russians,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=1,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=\nTaskForce=TF1\nWaypoint=2\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "team-wp.map", text).unwrap();
    assert_eq!(map.scripting.team_types[0].waypoint, 2);
    assert_eq!(map.waypoints.iter().find(|w| w.index == 2).map(|w| (w.x, w.y)), Some((12, 8)));
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "team-wp");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let unit = snap.units.iter().find(|u| u.type_id.as_ref() == "E1").expect("reinforced E1");
    assert_eq!((unit.x, unit.y), (12, 8), "Create Team should spawn at TeamType Waypoint");
}

#[test]
fn create_team_script_action_3_orders_move_to_waypoint() {
    // Waypoints: index 0 spawn (5,5); index 1 move target (10,10). Script step action=3,argument=1.
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n1=10010\n\
[Triggers]\nTR1=Russians,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=1,E1\nGroup=-1\n\
[ScriptTypes]\n0=SC1\n\
[SC1]\nName=MoveWp\n0=3,1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=SC1\nTaskForce=TF1\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "team-script.map", text).unwrap();
    assert_eq!(map.waypoints.iter().find(|w| w.index == 1).map(|w| (w.x, w.y)), Some((10, 10)));
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "team-script");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let ids: Vec<_> = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "E1").map(|u| u.id).collect();
    assert_eq!(ids.len(), 1, "expected one reinforced E1");
    session.tick(&engine.runtime());
    let dest = session.expect_battle().world.ecs_move_destination(ids[0]);
    assert_eq!(dest, Some((Some(10), Some(10))), "script action 3 should MoveTo waypoint 1");
}

#[test]
fn create_team_script_action_1_orders_attack_near_waypoint() {
    use ra_map::{MapEntity, MapEntityKind};

    // Waypoint 1 at (10,10) has a hostile American; Russian reinforce script action=1,argument=1.
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n1=10010\n\
[Triggers]\nTR1=Russians,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=1,E1\nGroup=-1\n\
[ScriptTypes]\n0=SC1\n\
[SC1]\nName=AtkWp\n0=1,1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=SC1\nTaskForce=TF1\nMax=1\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "team-atk.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Americans".into(),
        type_id: "E1".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: Default::default(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "team-atk");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    session.expect_battle_mut().world.ensure_house("Russians");

    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let russian = snap.units.iter().find(|u| u.owner.eq_ignore_ascii_case("Russians")).map(|u| u.id).expect("reinforced Russian");
    let american = snap.units.iter().find(|u| u.owner.eq_ignore_ascii_case("Americans")).map(|u| u.id).expect("preplaced American");

    session.tick(&engine.runtime());
    let attack = session.expect_battle().world.ecs_attack_state(russian);
    assert_eq!(attack.map(|(t, _)| t), Some(Some(american)), "script action 1 should Attack hostile near waypoint 1");
}

#[test]
fn create_team_script_action_6_deploys_mcv() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n",
    );
    // Script action=6 Deploy after Create Team spawns AMCV at waypoint 0.
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[Triggers]\nTR1=Americans,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Mcv\n0=1,AMCV\nGroup=-1\n\
[ScriptTypes]\n0=SC1\n\
[SC1]\nName=Deploy\n0=6,0\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Americans\nScript=SC1\nTaskForce=TF1\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "team-deploy.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "team-deploy");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;

    session.tick(&engine.runtime());
    let amcv = session.expect_battle().snapshot(&[]).units.iter().filter(|u| u.type_id.as_ref() == "AMCV").count();
    assert_eq!(amcv, 1, "expected reinforced AMCV");

    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let yards = snap.units.iter().filter(|u| u.type_id.as_ref() == "GACNST").count();
    let left = snap.units.iter().filter(|u| u.type_id.as_ref() == "AMCV").count();
    assert!(
        yards >= 1,
        "script action 6 should Deploy AMCV into GACNST, units={:?}",
        snap.units.iter().map(|u| u.type_id.as_ref()).collect::<Vec<_>>()
    );
    assert_eq!(left, 0, "AMCV should be gone after deploy");
}

#[test]
fn create_team_script_action_8_jumps_then_moves() {
    // Steps: 0=jump to 1, 1=move to waypoint 1. After jump tick, next tick should MoveTo.
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n1=10010\n\
[Triggers]\nTR1=Russians,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=1,E1\nGroup=-1\n\
[ScriptTypes]\n0=SC1\n\
[SC1]\nName=JumpMove\n0=8,1\n1=3,1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=SC1\nTaskForce=TF1\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "team-jump.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "team-jump");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;

    session.tick(&engine.runtime()); // spawn + jump to step 1
    let id = session.expect_battle().snapshot(&[]).units.iter().find(|u| u.type_id.as_ref() == "E1").map(|u| u.id).expect("E1");
    assert_eq!(session.expect_battle().world.ecs_move_destination(id), Some((None, None)));

    session.tick(&engine.runtime()); // execute move step
    session.tick(&engine.runtime()); // apply MoveTo
    assert_eq!(session.expect_battle().world.ecs_move_destination(id), Some((Some(10), Some(10))), "script action 8 should jump to move step");
}

#[test]
fn create_team_script_action_7_clears_move_after_waypoint() {
    // Steps: move to wp1, then guard (clear destination).
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n1=10010\n\
[Triggers]\nTR1=Russians,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=1,E1\nGroup=-1\n\
[ScriptTypes]\n0=SC1\n\
[SC1]\nName=MoveGuard\n0=3,1\n1=7,0\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=SC1\nTaskForce=TF1\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "team-guard.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_with_e1(), map), "team-guard");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;

    session.tick(&engine.runtime()); // spawn + enqueue MoveTo
    let id = session.expect_battle().snapshot(&[]).units.iter().find(|u| u.type_id.as_ref() == "E1").map(|u| u.id).expect("E1");
    session.tick(&engine.runtime()); // apply MoveTo + run Guard (clears dest)
    assert_eq!(
        session.expect_battle().world.ecs_move_destination(id),
        Some((None, None)),
        "script action 7 Guard should clear MoveTo destination"
    );
    assert_eq!(session.expect_battle().world.ecs_mission(id).as_deref(), Some("Guard"), "script action 7 Guard should set mission");
}
