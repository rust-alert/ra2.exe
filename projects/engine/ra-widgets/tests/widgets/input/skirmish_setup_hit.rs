//! 遭遇战大厅状态与 snapshot 命中集成测试。

use ra_layout::{LayoutSnapshot, RectPx, SKIRMISH_COMBO_ARROW_RESERVE, SKIRMISH_COMBO_FACE_H, solve_skirmish_lobby};
use ra_widgets::skirmish_setup::*;

fn sample_sides() -> Vec<String> {
    ["Americans", "French", "Germans", "British", "Russians"].into_iter().map(str::to_string).collect()
}

fn lobby_with_sides() -> SkirmishBootRequest {
    let mut s = SkirmishBootRequest::default_lobby();
    s.set_lobby_sides(sample_sides());
    s
}

fn snap_rect(snap: &LayoutSnapshot, id: &str) -> RectPx {
    let r = snap.get(id).expect(id).layout.rect;
    RectPx::new(r.x.round() as i32, r.y.round() as i32, r.width.round() as i32, r.height.round() as i32)
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
    let snap = solve_skirmish_lobby();
    let mut s = lobby_with_sides();
    let r = snap_rect(&snap, "checkbox_quick");
    assert_eq!(s.on_press(r.x + 2, r.y + 2, 1), Some(SkirmishLobbyHit::Toggle(SkirmishCheckbox::ShortGame)));
    assert!(!s.short_game);
    let t = snap_rect(&snap, "track_speed");
    assert_eq!(s.on_press(t.x + 4, t.y + 4, 1), Some(SkirmishLobbyHit::Track(SkirmishTrackbar::GameSpeed)));
    assert!(s.on_drag(t.x + t.w - 4, t.y + 4));
    assert_eq!(s.game_speed, 6);
    s.on_release();
    assert!(s.dragging.is_none());
}

#[test]
fn side_face_click_opens_country_combo() {
    let snap = solve_skirmish_lobby();
    let mut s = lobby_with_sides();
    assert_eq!(s.side, "Americans");
    let (ax, ay) = combo_arrow_point(snap_rect(&snap, "side_face_0"));
    assert_eq!(s.on_press(ax, ay, 1), Some(SkirmishLobbyHit::ToggleCountryCombo));
    assert_eq!(s.open_combo, Some(SkirmishComboKind::Country));
    let list = SkirmishBootRequest::country_list_rect(0, s.sides.len());
    // 第三项 Germans，避免与默认行 1（French）撞名。
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(list.x + 2, y, 1), Some(SkirmishLobbyHit::PickCountry(2)));
    assert_eq!(s.side, "Germans");
    assert_eq!(s.row_side(0), "Germans");
    assert_eq!(s.row_side(1), "French");
    assert!(s.open_combo.is_none());
}

#[test]
fn color_face_click_opens_color_combo() {
    let snap = solve_skirmish_lobby();
    let mut s = lobby_with_sides();
    assert_eq!(s.color_index, 0);
    let (ax, ay) = combo_arrow_point(snap_rect(&snap, "color_face_0"));
    assert_eq!(s.on_press(ax, ay, 1), Some(SkirmishLobbyHit::ToggleColorCombo));
    assert_eq!(s.open_combo, Some(SkirmishComboKind::Color));
    let list = SkirmishBootRequest::color_list_rect(0);
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(list.x + 2, y, 1), Some(SkirmishLobbyHit::PickColor(2)));
    assert_eq!(s.color_index, 2);
    assert_eq!(s.row_color_rgb(0), LOBBY_COLORS[2]);
    assert_eq!(s.row_color_rgb(1), LOBBY_COLORS[1]);
    assert!(s.open_combo.is_none());
}

#[test]
fn ai_row_country_pick_does_not_change_local_side() {
    let snap = solve_skirmish_lobby();
    let mut s = lobby_with_sides();
    assert_eq!(s.row_side(0), "Americans");
    assert_eq!(s.row_side(1), "French");
    let (ax, ay) = combo_arrow_point(snap_rect(&snap, "side_face_1"));
    assert_eq!(s.on_press(ax, ay, 1), Some(SkirmishLobbyHit::ToggleCountryCombo));
    assert_eq!(s.combo_row, 1);
    let list = SkirmishBootRequest::country_list_rect(1, s.sides.len());
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(list.x + 2, y, 1), Some(SkirmishLobbyHit::PickCountry(2)));
    assert_eq!(s.row_side(1), "Germans");
    assert_eq!(s.row_side(0), "Americans");
    assert_eq!(s.side, "Americans");
}

#[test]
fn ai_face_click_opens_difficulty_combo() {
    let snap = solve_skirmish_lobby();
    let mut s = lobby_with_sides();
    assert_eq!(s.difficulty, "Normal");
    let (ax, ay) = combo_arrow_point(snap_rect(&snap, "ai_face_0"));
    assert_eq!(s.on_press(ax, ay, 1), Some(SkirmishLobbyHit::ToggleAiCombo));
    assert_eq!(s.open_combo, Some(SkirmishComboKind::Ai));
    let list = SkirmishBootRequest::ai_list_rect();
    // 第三项 Hard。
    let y = list.y + SKIRMISH_COMBO_FACE_H * 2 + 2;
    assert_eq!(s.on_press(list.x + 2, y, 1), Some(SkirmishLobbyHit::PickAi(2)));
    assert_eq!(s.difficulty, "Hard");
    assert_eq!(SkirmishBootRequest::ai_difficulty_csf_key(&s.difficulty), "GUI:AIHard");
    assert!(s.open_combo.is_none());
}

#[test]
fn ai_face_ignored_when_map_has_no_ai_rows() {
    let snap = solve_skirmish_lobby();
    let mut s = lobby_with_sides();
    let (ax, ay) = combo_arrow_point(snap_rect(&snap, "ai_face_0"));
    assert_eq!(s.on_press(ax, ay, 0), None);
    assert!(s.open_combo.is_none());
}

#[test]
fn player_name_edit_accepts_ascii_and_backspace() {
    let snap = solve_skirmish_lobby();
    let mut s = lobby_with_sides();
    let r = snap_rect(&snap, "player_name");
    assert_eq!(s.on_press(r.x + 2, r.y + 2, 1), Some(SkirmishLobbyHit::FocusName));
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
fn pick_side_flag_uses_explicit_candidates_only() {
    assert_eq!(pick_side_flag_pcx(&["usai.pcx"], |_| Some(1)), Some("usai.pcx"));
    assert_eq!(pick_side_flag_pcx(&[], |_| Some(1)), None);
    // 基包 djbi；expand 覆盖 lati → 选更高 priority。
    let picked = pick_side_flag_pcx(&["djbi.pcx", "lati.pcx"], |name| match name {
        "djbi.pcx" => Some(0),
        "lati.pcx" => Some(101),
        _ => None,
    });
    assert_eq!(picked, Some("lati.pcx"));
    // 同优先级保留候选表更靠前项。
    let vanilla = pick_side_flag_pcx(&["djbi.pcx", "lati.pcx"], |name| match name {
        "djbi.pcx" | "lati.pcx" => Some(0),
        _ => None,
    });
    assert_eq!(vanilla, Some("djbi.pcx"));
}

#[test]
fn hover_entry_reports_checkbox_and_country() {
    let snap = solve_skirmish_lobby();
    let r = snap_rect(&snap, "checkbox_quick");
    assert_eq!(hover_entry_at(r.x + 2, r.y + 2), Some("short_game"));
    let face = snap_rect(&snap, "side_face_0");
    assert_eq!(hover_entry_at(face.x + 2, face.y + 2), Some("country"));
    let preview = snap_rect(&snap, "map_preview");
    assert_eq!(hover_entry_at(preview.x + 2, preview.y + 2), Some("map_preview"));
}
