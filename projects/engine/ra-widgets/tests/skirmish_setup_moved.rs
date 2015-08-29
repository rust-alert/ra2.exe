//! 集成测试：原 `src/skirmish_setup.rs` 内联测试迁出。

use ra_widgets::skirmish_setup::*;
use ra_layout::ui_layout::{RectPx, SKIRMISH_COMBO_ARROW_RESERVE, SKIRMISH_COMBO_FACE_H, skirmish_lobby_layout};

fn sample_sides() -> Vec<String> {
    ["Americans", "French", "Germans", "British", "Russians"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn lobby_with_sides() -> SkirmishBootRequest {
    let mut s = SkirmishBootRequest::default_lobby();
    s.set_lobby_sides(sample_sides());
    s
}

/// 下拉仅右侧箭头可切换（与壳层 `dnarrow` 命中一致）。
fn combo_arrow_point(face: RectPx) -> (i32, i32) {
    let w = SKIRMISH_COMBO_ARROW_RESERVE.min(face.w.max(0));
    (face.x + face.w - w / 2, face.y + face.h / 2)
}

#[test]
fn default_options_match_retail_defaults() {
    let s = SkirmishBootRequest::default_lobby();
    assert!(s.short_game && s.mcv_repacks && s.crates && s.superweapons);
    assert!(!s.build_off_ally);
    assert_eq!(s.game_speed, 6);
    assert_eq!(s.credits, 10_000);
    assert_eq!(s.unit_count, 10);
    assert_eq!(s.player_name, "Player");
    assert!(s.sides.is_empty());
}

#[test]
fn checkbox_toggle_and_track_drag() {
    let layout = skirmish_lobby_layout(800, 600);
    let mut s = lobby_with_sides();
    let r = layout.checkboxes[0];
    assert_eq!(s.on_press(&layout, r.x + 2, r.y + 2, 1), Some(SkirmishLobbyHit::Toggle(SkirmishCheckbox::ShortGame)));
    assert!(!s.short_game);
    let t = layout.track_speed;
    assert_eq!(s.on_press(&layout, t.x + 4, t.y + 4, 1), Some(SkirmishLobbyHit::Track(SkirmishTrackbar::GameSpeed)));
    assert!(s.on_drag(&layout, t.x + t.w - 4, t.y + 4));
    assert_eq!(s.game_speed, 6);
    s.on_release();
    assert!(s.dragging.is_none());
}

#[test]
fn side_face_click_opens_country_combo() {
    let layout = skirmish_lobby_layout(800, 600);
    let mut s = lobby_with_sides();
    assert_eq!(s.side, "Americans");
    let (ax, ay) = combo_arrow_point(layout.side_faces[0]);
    assert_eq!(s.on_press(&layout, ax, ay, 1), Some(SkirmishLobbyHit::ToggleCountryCombo));
    assert_eq!(s.open_combo, Some(SkirmishComboKind::Country));
    let list = SkirmishBootRequest::country_list_rect(&layout, 0, s.sides.len());
    // 第三项 Germans，避免与默认行 1（French）撞名。
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(&layout, list.x + 2, y, 1), Some(SkirmishLobbyHit::PickCountry(2)));
    assert_eq!(s.side, "Germans");
    assert_eq!(s.row_side(0), "Germans");
    assert_eq!(s.row_side(1), "French");
    assert!(s.open_combo.is_none());
}

#[test]
fn color_face_click_opens_color_combo() {
    let layout = skirmish_lobby_layout(800, 600);
    let mut s = lobby_with_sides();
    assert_eq!(s.color_index, 0);
    let (ax, ay) = combo_arrow_point(layout.color_faces[0]);
    assert_eq!(s.on_press(&layout, ax, ay, 1), Some(SkirmishLobbyHit::ToggleColorCombo));
    assert_eq!(s.open_combo, Some(SkirmishComboKind::Color));
    let list = SkirmishBootRequest::color_list_rect(&layout, 0);
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(&layout, list.x + 2, y, 1), Some(SkirmishLobbyHit::PickColor(2)));
    assert_eq!(s.color_index, 2);
    assert_eq!(s.row_color_rgb(0), LOBBY_COLORS[2]);
    assert_eq!(s.row_color_rgb(1), LOBBY_COLORS[1]);
    assert!(s.open_combo.is_none());
}

#[test]
fn ai_row_country_pick_does_not_change_local_side() {
    let layout = skirmish_lobby_layout(800, 600);
    let mut s = lobby_with_sides();
    assert_eq!(s.row_side(0), "Americans");
    assert_eq!(s.row_side(1), "French");
    let (ax, ay) = combo_arrow_point(layout.side_faces[1]);
    assert_eq!(s.on_press(&layout, ax, ay, 1), Some(SkirmishLobbyHit::ToggleCountryCombo));
    assert_eq!(s.combo_row, 1);
    let list = SkirmishBootRequest::country_list_rect(&layout, 1, s.sides.len());
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(&layout, list.x + 2, y, 1), Some(SkirmishLobbyHit::PickCountry(2)));
    assert_eq!(s.row_side(1), "Germans");
    assert_eq!(s.row_side(0), "Americans");
    assert_eq!(s.side, "Americans");
}

#[test]
fn ai_face_click_opens_difficulty_combo() {
    let layout = skirmish_lobby_layout(800, 600);
    let mut s = lobby_with_sides();
    assert_eq!(s.difficulty, "Normal");
    let (ax, ay) = combo_arrow_point(layout.ai_faces[0]);
    assert_eq!(s.on_press(&layout, ax, ay, 1), Some(SkirmishLobbyHit::ToggleAiCombo));
    assert_eq!(s.open_combo, Some(SkirmishComboKind::Ai));
    let list = SkirmishBootRequest::ai_list_rect(&layout);
    // 第三项 Hard。
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(&layout, list.x + 2, y, 1), Some(SkirmishLobbyHit::PickAi(2)));
    assert_eq!(s.difficulty, "Hard");
    assert_eq!(SkirmishBootRequest::ai_difficulty_csf_key(&s.difficulty), "GUI:AIHard");
    assert!(s.open_combo.is_none());
}

#[test]
fn ai_face_ignored_when_map_has_no_ai_rows() {
    let layout = skirmish_lobby_layout(800, 600);
    let mut s = lobby_with_sides();
    let (ax, ay) = combo_arrow_point(layout.ai_faces[0]);
    assert_eq!(s.on_press(&layout, ax, ay, 0), None);
    assert!(s.open_combo.is_none());
}

#[test]
fn player_name_edit_accepts_ascii_and_backspace() {
    let layout = skirmish_lobby_layout(800, 600);
    let mut s = lobby_with_sides();
    let r = layout.player_name;
    assert_eq!(s.on_press(&layout, r.x + 2, r.y + 2, 1), Some(SkirmishLobbyHit::FocusName));
    assert!(s.player_name_editing);
    s.player_name.clear();
    assert!(s.append_name_text("Ab"));
    assert_eq!(s.player_name, "Ab");
    assert!(s.backspace_name());
    assert_eq!(s.player_name, "A");
    // 超长截断。
    s.player_name.clear();
    assert!(s.append_name_text(&"x".repeat(PLAYER_NAME_MAX_CHARS + 4)));
    assert_eq!(s.player_name.len(), PLAYER_NAME_MAX_CHARS);
    s.end_name_edit();
    assert!(!s.player_name_editing);
    // 空名回退 `Player`。
    s.player_name_editing = true;
    s.player_name.clear();
    s.end_name_edit();
    assert_eq!(s.player_name, "Player");
}

#[test]
fn americans_flag_pcx() {
    assert_eq!(side_flag_pcx("Americans"), "usai.pcx");
    assert_eq!(side_flag_pcx("Russians"), "rusi.pcx");
    assert_eq!(side_flag_pcx("Alliance"), "japi.pcx");
    assert_eq!(side_flag_pcx("Africans"), "lati.pcx");
    assert_eq!(side_flag_pcx("Arabs"), "arbi.pcx");
    assert_eq!(side_flag_pcx("Confederation"), "djbi.pcx");
    assert_eq!(side_flag_pcx("YuriCountry"), "yrii.pcx");
    assert_eq!(side_flag_pcx_candidates("Africans"), &["lati.pcx", "lybi.pcx"]);
}

#[test]
fn hover_entry_reports_checkbox_and_country() {
    let layout = skirmish_lobby_layout(800, 600);
    let r = layout.checkboxes[0];
    assert_eq!(hover_entry_at(&layout, r.x + 2, r.y + 2), Some("short_game"));
    let face = layout.side_faces[0];
    assert_eq!(hover_entry_at(&layout, face.x + 2, face.y + 2), Some("country"));
    assert_eq!(hover_entry_at(&layout, layout.map_preview.x + 2, layout.map_preview.y + 2), Some("map_preview"));
}
