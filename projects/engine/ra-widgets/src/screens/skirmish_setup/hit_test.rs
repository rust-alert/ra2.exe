//! 遭遇战大厅配置：对话框 `0x102` 选项 + 装载请求。
//!
//! 控件几何一律来自 `solve_skirmish_lobby` snapshot；本模块只持状态与命中。

use ra_layout::{
    LayoutSnapshot, RectPx, SKIRMISH_AI_ROW_COUNT, SKIRMISH_CHECK_H, SKIRMISH_CHECK_W, SKIRMISH_ROW_COUNT, solve_skirmish_lobby_ex,
};

use super::layout::{snap_contains, snap_rect_px};

/// 光标下的悬停入口 id（供底栏 `STT:Skirmish*`；不改状态）。原版无队伍列。
pub fn hover_entry_at(x: i32, y: i32) -> Option<&'static str> {
    hover_entry_ex(x, y, false)
}

/// 光标下的悬停入口。`lobby_teams` 为真时包含队伍列。
pub fn hover_entry_ex(x: i32, y: i32, lobby_teams: bool) -> Option<&'static str> {
    hover_entry_in(&solve_skirmish_lobby_ex(lobby_teams), x, y, lobby_teams)
}

fn hover_entry_in(snap: &LayoutSnapshot, x: i32, y: i32, lobby_teams: bool) -> Option<&'static str> {
    if snap_contains(snap, "player_name", x, y) {
        return Some("player_name");
    }
    for i in 0..SKIRMISH_ROW_COUNT {
        if snap_contains(snap, &format!("flag_{i}"), x, y) {
            return Some("flag");
        }
    }
    for i in 0..SKIRMISH_ROW_COUNT {
        if snap_contains(snap, &format!("side_face_{i}"), x, y) {
            return Some("country");
        }
    }
    for i in 0..SKIRMISH_ROW_COUNT {
        if snap_contains(snap, &format!("color_face_{i}"), x, y) {
            return Some("color");
        }
    }
    if lobby_teams {
        for i in 0..SKIRMISH_ROW_COUNT {
            if snap_contains(snap, &format!("team_face_{i}"), x, y) {
                return Some("team");
            }
        }
    }
    for i in 0..SKIRMISH_AI_ROW_COUNT {
        if snap_contains(snap, &format!("ai_face_{i}"), x, y) {
            return Some("ai");
        }
    }
    const CHECKBOXES: &[(&str, &str)] = &[
        ("checkbox_quick", "short_game"),
        ("checkbox_1", "mcv_repacks"),
        ("checkbox_2", "crates"),
        ("checkbox_3", "superweapons"),
        ("checkbox_4", "build_off_ally"),
    ];
    for (snap_id, entry) in CHECKBOXES {
        if let Some(rect) = snap_rect_px(snap, snap_id) {
            let icon = RectPx::new(rect.x, rect.y, SKIRMISH_CHECK_W, SKIRMISH_CHECK_H.min(rect.h.max(SKIRMISH_CHECK_H)));
            if icon.contains(x, y) || rect.contains(x, y) {
                return Some(*entry);
            }
        }
    }
    if snap_contains(snap, "track_speed", x, y) || snap_contains(snap, "label_speed", x, y) {
        return Some("speed");
    }
    if snap_contains(snap, "track_credits", x, y) || snap_contains(snap, "label_credits", x, y) {
        return Some("credits");
    }
    if snap_contains(snap, "track_units", x, y) || snap_contains(snap, "label_units", x, y) {
        return Some("units");
    }
    if snap_contains(snap, "map_preview", x, y) {
        return Some("map_preview");
    }
    if snap_contains(snap, "game_type", x, y) {
        return Some("game_type");
    }
    if snap_contains(snap, "map_label", x, y) {
        return Some("map_label");
    }
    None
}
