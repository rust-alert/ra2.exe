//! 遭遇战积分页 `0x108` DLU 几何。

use ra_layout::{RectPx, SCORE_ROW_SLOTS, rect_px_from_snapshot, solve_skirmish_score};

#[test]
fn score_columns_match_dialog_0x108_at_800x600() {
    let snap = solve_skirmish_score();
    assert_eq!(rect_px_from_snapshot(&snap, "header_name"), RectPx::new(99, 159, 113, 16));
    assert_eq!(rect_px_from_snapshot(&snap, "header_kills"), RectPx::new(221, 159, 68, 16));
    assert_eq!(rect_px_from_snapshot(&snap, "header_losses"), RectPx::new(303, 159, 68, 16));
    assert_eq!(rect_px_from_snapshot(&snap, "header_built"), RectPx::new(386, 159, 68, 16));
    assert_eq!(rect_px_from_snapshot(&snap, "header_score"), RectPx::new(462, 159, 68, 16));
}

#[test]
fn score_row_slots_follow_template_pitch() {
    let snap = solve_skirmish_score();
    let tops: Vec<i32> = (0..SCORE_ROW_SLOTS).map(|i| rect_px_from_snapshot(&snap, &format!("row{i}_name")).y).collect();
    assert_eq!(tops, vec![195, 231, 267, 302, 338, 374, 410, 445]);
}

#[test]
fn score_continue_snaps_to_bottom_panel_row() {
    let continue_btn = rect_px_from_snapshot(&solve_skirmish_score(), "continue");
    assert_eq!(continue_btn, RectPx::new(644, 535, 156, 42));
}

#[test]
fn score_summary_sits_above_headers() {
    let snap = solve_skirmish_score();
    let game = rect_px_from_snapshot(&snap, "game_label");
    let time = rect_px_from_snapshot(&snap, "time_label");
    let header = rect_px_from_snapshot(&snap, "header_name");
    assert_eq!(game, RectPx::new(99, 124, 173, 16));
    assert_eq!(time, RectPx::new(372, 124, 158, 16));
    assert!(game.y < header.y);
}
