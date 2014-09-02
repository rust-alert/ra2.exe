//! 集成测试：遭遇战大厅配置循环。

use ra_desktop::skirmish_setup::{LOBBY_DIFFICULTIES, LOBBY_SIDES, SkirmishBootRequest};

#[test]
fn cycle_side_wraps() {
    let mut req = SkirmishBootRequest::default_lobby();
    assert_eq!(req.side, LOBBY_SIDES[0]);
    for expected in LOBBY_SIDES.iter().skip(1) {
        req.cycle_side();
        assert_eq!(req.side, *expected);
    }
    req.cycle_side();
    assert_eq!(req.side, LOBBY_SIDES[0]);
}

#[test]
fn cycle_difficulty_advances() {
    let mut req = SkirmishBootRequest::default_lobby();
    assert_eq!(req.difficulty, LOBBY_DIFFICULTIES[1]);
    req.cycle_difficulty();
    assert_eq!(req.difficulty, LOBBY_DIFFICULTIES[2]);
    req.cycle_difficulty();
    assert_eq!(req.difficulty, LOBBY_DIFFICULTIES[0]);
}
