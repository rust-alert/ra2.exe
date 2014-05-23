//! 集成测试：原 `src/skirmish_setup.rs` 内联测试迁出。

use ra_desktop::skirmish_setup::*;
#[test]
fn cycle_side_wraps() {
    let mut r = SkirmishBootRequest::default_lobby();
    assert_eq!(r.side, "Americans");
    r.cycle_side();
    assert_eq!(r.side, "Russians");
    r.cycle_side();
    assert_eq!(r.side, "Americans");
}

#[test]
fn cycle_difficulty_advances() {
    let mut r = SkirmishBootRequest::default_lobby();
    assert_eq!(r.difficulty, "Normal");
    r.cycle_difficulty();
    assert_eq!(r.difficulty, "Hard");
}
