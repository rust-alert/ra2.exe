//! 动作码 46/47：Lock / Unlock input。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{Session, SessionBootKind};
use ra_map::{MapActionKind, MapInfo, campaign_blocking_capability_message, map_scripting_capability_gaps};
use ra_types::GameEdition;

#[test]
fn lock_input_fires_on_campaign_timer() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Tags]\nT1=0,Start,TR1\n\
[Triggers]\nTR1=Americans,<none>,LockIn,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,46,0,0,0,0,0,0,A\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "lock.map", text).unwrap();
    assert_eq!(map.scripting.actions[0].commands[0].kind, MapActionKind::LockInput);
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_from_rules_ini(b"[General]\n"), map), "lock");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("AMERICANS");
    let _ = session.expect_battle_mut().world.prefer_local_house("AMERICANS");
    assert!(!session.expect_battle().world.trigger_runtime.script_input_locked);

    for _ in 0..5 {
        session.tick(&engine.runtime());
        if session.expect_battle().world.trigger_runtime.script_input_locked {
            break;
        }
    }
    assert!(session.expect_battle().world.trigger_runtime.script_input_locked, "action 46 should lock input");
}

#[test]
fn unlock_input_clears_lock_after_timer() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Tags]\nT1=0,Start,TR1\nT2=0,Unlock,TR2\n\
[Triggers]\nTR1=Americans,<none>,LockIn,0,1,1,1,0\nTR2=Americans,<none>,Unlock,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\nTR2=1,13,3,0\n\
[Actions]\nTR1=1,46,0,0,0,0,0,0,A\nTR2=1,47,0,0,0,0,0,0,A\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "unlock.map", text).unwrap();
    assert_eq!(map.scripting.actions[1].commands[0].kind, MapActionKind::UnlockInput);
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs_from_rules_ini(b"[General]\n"), map), "unlock");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("AMERICANS");
    let _ = session.expect_battle_mut().world.prefer_local_house("AMERICANS");

    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.trigger_runtime.script_input_locked, "tick0: lock");
    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.trigger_runtime.script_input_locked, "tick1: still locked");
    session.tick(&engine.runtime());
    assert!(!session.expect_battle().world.trigger_runtime.script_input_locked, "tick2: unlock");
}

#[test]
fn lock_unlock_are_not_campaign_blocking_gaps() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Triggers]\nTR1=Americans,<none>,X,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=2,46,0,0,0,0,0,0,A,47,0,0,0,0,0,0,A\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "gap.map", text).unwrap();
    let gaps = map_scripting_capability_gaps(&map);
    assert!(gaps.iter().all(|g| !g.code.contains("map.action.46 unsupported")), "{gaps:?}");
    assert!(gaps.iter().all(|g| !g.code.contains("map.action.47 unsupported")), "{gaps:?}");
    assert!(campaign_blocking_capability_message(&map).is_none());
}
