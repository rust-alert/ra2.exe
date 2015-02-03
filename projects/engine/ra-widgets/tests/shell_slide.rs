//! 集成测试：原 `src/shell_slide.rs` 内联测试迁出。

use ra_widgets::shell_slide::*;
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
fn panel_wave_uses_physical_tile_slots() {
    // 格 4（Options）→ tick 5；格 8（Exit）→ tick 9；总长覆盖整列。
    assert_eq!(entry_tick_for_slot(4), 5);
    assert_eq!(entry_tick_for_slot(8), 9);
    assert_eq!(MAIN_MENU_SLIDE.slot_count, SHELL_PANEL_WAVE_SLOTS);
    assert_eq!(total_ticks_for(SHELL_PANEL_WAVE_SLOTS), 18);
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
