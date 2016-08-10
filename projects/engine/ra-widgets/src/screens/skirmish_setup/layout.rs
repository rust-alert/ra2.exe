//! 遭遇战大厅配置：对话框 `0x102` 选项 + 装载请求。
//!
//! 控件几何一律来自 `solve_skirmish_lobby` snapshot；本模块只持状态与命中。

use ra_layout::{
    LayoutSnapshot, RectPx, SKIRMISH_COMBO_ARROW_RESERVE, SKIRMISH_COMBO_FACE_H, SKIRMISH_ROW_COUNT, SKIRMISH_TRACK_ACTIVE_PAD,
    SKIRMISH_TRACK_PLAQUE_W, popup_list_below, popup_list_below_min_w,
};

/// 大厅可选难度标签（写入装载请求；引擎按 Easy/Normal/Hard 调节 AI 节奏）。
use super::model::*;

pub(super) fn country_list_rect_in(snap: &LayoutSnapshot, row: usize, side_count: usize) -> RectPx {
    let row = row.min(SKIRMISH_ROW_COUNT.saturating_sub(1));
    let face = snap_rect_px(snap, &format!("side_face_{row}")).unwrap_or(RectPx::new(0, 0, 0, 0));
    popup_list_below(face, SKIRMISH_COMBO_FACE_H, side_count.max(1))
}

pub(super) fn color_list_rect_in(snap: &LayoutSnapshot, row: usize) -> RectPx {
    let row = row.min(SKIRMISH_ROW_COUNT.saturating_sub(1));
    let face = snap_rect_px(snap, &format!("color_face_{row}")).unwrap_or(RectPx::new(0, 0, 0, 0));
    popup_list_below_min_w(face, SKIRMISH_COMBO_FACE_H, LOBBY_COLORS.len(), 28)
}

pub(super) fn ai_list_rect_in(snap: &LayoutSnapshot) -> RectPx {
    let face = snap_rect_px(snap, "ai_face_0").unwrap_or(RectPx::new(0, 0, 0, 0));
    popup_list_below(face, SKIRMISH_COMBO_FACE_H, LOBBY_DIFFICULTIES.len())
}

pub(super) fn snap_rect_px(snap: &LayoutSnapshot, id: &str) -> Option<RectPx> {
    let r = snap.get(id)?.layout.rect;
    Some(RectPx::new(r.x.round() as i32, r.y.round() as i32, r.width.round() as i32, r.height.round() as i32))
}

pub(super) fn snap_contains(snap: &LayoutSnapshot, id: &str, x: i32, y: i32) -> bool {
    snap_rect_px(snap, id).is_some_and(|r| r.contains(x, y))
}

pub(super) fn track_rect_from_snap(snap: &LayoutSnapshot, id: SkirmishTrackbar) -> Option<RectPx> {
    let key = match id {
        SkirmishTrackbar::GameSpeed => "track_speed",
        SkirmishTrackbar::Credits => "track_credits",
        SkirmishTrackbar::UnitCount => "track_units",
    };
    snap_rect_px(snap, key)
}

pub(super) fn combo_arrow_hit(face: RectPx) -> RectPx {
    let w = SKIRMISH_COMBO_ARROW_RESERVE.min(face.w.max(0));
    RectPx::new(face.x + face.w - w, face.y, w, face.h)
}

pub(super) fn track_pos_from_mouse(rect: RectPx, mouse_x: i32, id: SkirmishTrackbar) -> i32 {
    let max = id.max().max(1);
    // 活跃轨宽 = client_w - 50 - 13；鼠标 x 相对左缘偏 6 后映射。
    let active_w = (rect.w - SKIRMISH_TRACK_PLAQUE_W - SKIRMISH_TRACK_ACTIVE_PAD).max(1);
    let rel = (mouse_x - rect.x - 6).clamp(0, active_w);
    (rel * max + active_w / 2) / active_w
}
