//! 自 `engine/ra-engine/src/gameplay/eva_advice.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-engine/src/gameplay/eva_advice.rs :: tests
use ra_engine::gameplay::eva_advice::speak_delay_ticks_from_minutes;

#[test]
fn speak_delay_two_minutes_is_1800_ticks() {
    assert_eq!(speak_delay_ticks_from_minutes(2.0), 1800);
    assert_eq!(speak_delay_ticks_from_minutes(0.0), 0);
    assert_eq!(speak_delay_ticks_from_minutes(-1.0), 0);
}
