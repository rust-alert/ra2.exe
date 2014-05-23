//! 集成测试：原 `src/game/game.rs` 内联测试迁出。

use ra_engine::{difficulty_extra_produce, difficulty_skips_offensive};

#[test]
fn easy_skips_odd_ticks_hard_never_skips() {
    assert!(difficulty_skips_offensive("Easy", 1));
    assert!(!difficulty_skips_offensive("Easy", 2));
    assert!(!difficulty_skips_offensive("Hard", 3));
    assert!(difficulty_skips_offensive("Normal", 3));
    assert!(!difficulty_skips_offensive("Normal", 0));
}

#[test]
fn only_hard_gets_extra_produce() {
    assert!(difficulty_extra_produce("Hard"));
    assert!(!difficulty_extra_produce("Easy"));
    assert!(!difficulty_extra_produce("Normal"));
}
