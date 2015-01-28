//! 手动暂停与快照字段。

use crate::common::{rules_with_mtnk, test_engine};
use ra_engine::{BattleState, Session};
use ra_map::MapInfo;
use ra_types::GameEdition;

#[test]
fn toggle_pause_stops_pump_and_exposes_snapshot() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
    let map = MapInfo::empty(GameEdition::Ra2, "pause");
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "pause");
    session.tick_hz = 10;
    assert!(!session.expect_game().snapshot(&[]).paused);
    session.expect_game_mut().toggle_pause();
    assert!(session.expect_game().paused);
    assert_eq!(session.expect_game().pause_reason.as_deref(), Some("已暂停"));
    assert!(session.expect_game().snapshot(&[]).paused);
    assert_eq!(session.pump(&engine.runtime(), 1.0), 0);
    session.expect_game_mut().toggle_pause();
    assert!(!session.expect_game().paused);
    assert!(session.expect_game().pause_reason.is_none());
    assert_eq!(session.pump(&engine.runtime(), 0.1), 1);
}
