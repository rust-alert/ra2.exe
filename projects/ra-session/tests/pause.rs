//! 手动暂停与快照字段。

mod common;

use common::rules_with_mtnk;
use ra_map::MapInfo;
use ra_session::Session;
use ra_types::GameEdition;
use ra_world::World;

#[test]
fn toggle_pause_stops_pump_and_exposes_snapshot() {
    let rules = rules_with_mtnk();
    let map = MapInfo::empty(GameEdition::Ra2, "pause");
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "pause");
    session.tick_hz = 10;
    assert!(!session.snapshot().paused);
    session.toggle_pause();
    assert!(session.paused);
    assert_eq!(session.pause_reason.as_deref(), Some("已暂停"));
    assert!(session.snapshot().paused);
    assert_eq!(session.pump(1.0), 0);
    session.toggle_pause();
    assert!(!session.paused);
    assert!(session.pause_reason.is_none());
    assert_eq!(session.pump(0.1), 1);
}
