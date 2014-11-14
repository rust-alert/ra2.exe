//! 集成测试：原 `src/shell_slide.rs` 内联测试迁出。

use ra_desktop::shell_slide::*;
use std::time::{Duration, Instant};

#[test]
fn total_ticks_matches_native_table() {
    for (n, total) in [(3u32, 12), (4, 13), (5, 14), (6, 15)] {
        assert_eq!(total_ticks_for(n), total, "N={n}");
    }
}

#[test]
fn slide_in_ramps_down_then_settles() {
    // 槽 0 进场 tick=1：tick1→10 … tick6→5，之后收束 1。
    assert_eq!(frame_for_tick(0, 1, WaveDirection::SlideIn), 10);
    assert_eq!(frame_for_tick(1, 1, WaveDirection::SlideIn), 10);
    assert_eq!(frame_for_tick(2, 1, WaveDirection::SlideIn), 9);
    assert_eq!(frame_for_tick(6, 1, WaveDirection::SlideIn), 5);
    assert_eq!(frame_for_tick(7, 1, WaveDirection::SlideIn), 1);
}

#[test]
fn slide_out_ramps_up_then_settles() {
    assert_eq!(frame_for_tick(0, 1, WaveDirection::SlideOut), 1);
    assert_eq!(frame_for_tick(1, 1, WaveDirection::SlideOut), 5);
    assert_eq!(frame_for_tick(2, 1, WaveDirection::SlideOut), 6);
    assert_eq!(frame_for_tick(6, 1, WaveDirection::SlideOut), 10);
    assert_eq!(frame_for_tick(7, 1, WaveDirection::SlideOut), 10);
}

#[test]
fn main_menu_exit_shares_options_stagger() {
    assert_eq!(main_menu_entry_tick("options", 4), 5);
    assert_eq!(main_menu_entry_tick("exit", 5), 5);
}

#[test]
fn advance_one_tick_per_interval() {
    let start = Instant::now();
    let mut wave = ShellFrameWave::new(MAIN_MENU_SLIDE, WaveDirection::SlideIn, start);
    assert!(!wave.advance(start));
    assert!(wave.advance(start + Duration::from_millis(30)));
    assert_eq!(wave.tick(), 1);
    assert!(!wave.advance(start + Duration::from_millis(45)));
    assert!(wave.advance(start + Duration::from_millis(60)));
    assert_eq!(wave.tick(), 2);
}
