//! 选中血条门控与 pip 计数（不依赖 MIX）。

use ra_renderer::{
    BUILDING_PIP_FRAMES, HealthPipTone, UNIT_PIP_FRAMES, building_pip_count, filled_pip_count, health_pip_tone, ndc_visible, status_visibility,
};

#[test]
fn ndc_margin_rejects_far_points() {
    assert!(ndc_visible([0.0, 0.0], 1.15));
    assert!(ndc_visible([1.1, -1.1], 1.15));
    assert!(!ndc_visible([2.0, 0.0], 1.15));
    assert!(!ndc_visible([0.0, -3.0], 1.15));
}

#[test]
fn health_tone_uses_condition_thresholds() {
    assert_eq!(health_pip_tone(1.0, 0.5, 0.25), HealthPipTone::Green);
    assert_eq!(health_pip_tone(0.51, 0.5, 0.25), HealthPipTone::Green);
    assert_eq!(health_pip_tone(0.5, 0.5, 0.25), HealthPipTone::Yellow);
    assert_eq!(health_pip_tone(0.26, 0.5, 0.25), HealthPipTone::Yellow);
    assert_eq!(health_pip_tone(0.25, 0.5, 0.25), HealthPipTone::Red);
    assert_eq!(health_pip_tone(0.01, 0.5, 0.25), HealthPipTone::Red);
}

#[test]
fn filled_pips_floor_and_alive_minimum() {
    assert_eq!(filled_pip_count(100, 100, 17), 17);
    assert_eq!(filled_pip_count(50, 100, 17), 8);
    assert_eq!(filled_pip_count(1, 100, 17), 1);
    assert_eq!(filled_pip_count(0, 100, 17), 0);
    assert_eq!(filled_pip_count(100, 0, 17), 0);
    assert_eq!(filled_pip_count(8, 100, 8), 1);
}

#[test]
fn building_pip_count_from_foundation_height() {
    assert_eq!(building_pip_count(1), 7);
    assert_eq!(building_pip_count(2), 15);
    assert_eq!(building_pip_count(3), 22);
    assert_eq!(building_pip_count(4), 30);
}

#[test]
fn selected_draws_bracket_and_pips_hover_only_pips() {
    assert_eq!(status_visibility(true, false), (true, true));
    assert_eq!(status_visibility(true, true), (true, true));
    assert_eq!(status_visibility(false, true), (false, true));
    assert_eq!(status_visibility(false, false), (false, false));
}

#[test]
fn stock_pip_frame_indices() {
    assert_eq!(UNIT_PIP_FRAMES, [15, 16, 17]);
    assert_eq!(BUILDING_PIP_FRAMES, [0, 1, 2, 4]);
}

// 自顶层 `markers_unit.rs` 并入。

// 自 engine/ra-renderer/src/markers.rs :: tests

#[test]
fn tone_thresholds_match_audio_visual_defaults() {
    assert_eq!(health_pip_tone(1.0, 0.5, 0.25), HealthPipTone::Green);
    assert_eq!(health_pip_tone(0.5, 0.5, 0.25), HealthPipTone::Yellow);
    assert_eq!(health_pip_tone(0.25, 0.5, 0.25), HealthPipTone::Red);
    assert_eq!(health_pip_tone(0.1, 0.5, 0.25), HealthPipTone::Red);
}

#[test]
fn filled_pips_floor_and_min_one_while_alive() {
    assert_eq!(filled_pip_count(100, 100, 17), 17);
    assert_eq!(filled_pip_count(1, 100, 17), 1);
    assert_eq!(filled_pip_count(0, 100, 17), 0);
    assert_eq!(filled_pip_count(50, 100, 8), 4);
}

#[test]
fn building_pip_count_by_foundation_height() {
    assert_eq!(building_pip_count(1), 7);
    assert_eq!(building_pip_count(2), 15);
    assert_eq!(building_pip_count(4), 30);
}

#[test]
fn hover_skips_bracket_selected_keeps_both() {
    assert_eq!(status_visibility(true, false), (true, true));
    assert_eq!(status_visibility(false, true), (false, true));
    assert_eq!(status_visibility(false, false), (false, false));
}
