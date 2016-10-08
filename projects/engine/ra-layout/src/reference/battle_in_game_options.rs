//! 局内选项 dialog `0xBBB`（暂停菜单二级页）。
//!
//! 与暂停菜单同族：全屏 dim + 右缘 `SIDEBTTN`；普通控件取相对 800×600 的居中偏移。
//! **不是**主菜单选项壳（`mnscrnl` / `0xF5`）。

use crate::{
    geometry::Rect,
    reference::{DluRect, MS_SANS_SERIF_8PT},
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

use super::battle_pause::{BATTLE_PAUSE_BASE_H, BATTLE_PAUSE_BASE_W, battle_pause_center_offset, battle_sidebttn_rect};

/// 右栏钮：Sound / Keyboard / Back。
pub const BATTLE_IN_GAME_OPTIONS_BUTTON_IDS: [&str; 3] = ["sound", "keyboard", "back"];

/// Sound（`0x52D`）资源 DLU。
const SOUND_DLU: DluRect = DluRect::new(425, 122, 108, 23);
/// Keyboard（`0x52C`）资源 DLU。
const KEYBOARD_DLU: DluRect = DluRect::new(425, 149, 108, 23);
/// Back（`0x686`）资源 DLU。
const BACK_DLU: DluRect = DluRect::new(425, 346, 108, 23);

fn centered_dlu(screen_w: f32, screen_h: f32, dlu: DluRect) -> Rect {
    let r = dlu.to_design_px(MS_SANS_SERIF_8PT);
    let dx = battle_pause_center_offset(screen_w, BATTLE_PAUSE_BASE_W);
    let dy = battle_pause_center_offset(screen_h, BATTLE_PAUSE_BASE_H);
    Rect::from_xywh(r.x + dx, r.y + dy, r.width, r.height)
}

/// 局内选项布局树。
pub fn battle_in_game_options_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let children = vec![
        fixed_rect_leaf("dim", Rect::from_xywh(0.0, 0.0, w, h)),
        fixed_rect_leaf("title", centered_dlu(w, h, DluRect::new(425, 1, 108, 10))),
        fixed_rect_leaf("caption_game_speed", centered_dlu(w, h, DluRect::new(40, 100, 100, 10))),
        fixed_rect_leaf("track_game_speed", centered_dlu(w, h, DluRect::new(144, 100, 128, 13))),
        fixed_rect_leaf("value_game_speed", centered_dlu(w, h, DluRect::new(280, 100, 80, 10))),
        fixed_rect_leaf("caption_scroll_rate", centered_dlu(w, h, DluRect::new(40, 131, 100, 10))),
        fixed_rect_leaf("track_scroll_rate", centered_dlu(w, h, DluRect::new(144, 131, 128, 13))),
        fixed_rect_leaf("value_scroll_rate", centered_dlu(w, h, DluRect::new(280, 131, 80, 10))),
        fixed_rect_leaf("check_target_lines", centered_dlu(w, h, DluRect::new(89, 206, 119, 10))),
        fixed_rect_leaf("check_show_hidden", centered_dlu(w, h, DluRect::new(89, 224, 119, 10))),
        fixed_rect_leaf("check_tooltips", centered_dlu(w, h, DluRect::new(214, 206, 127, 10))),
        fixed_rect_leaf("footer", centered_dlu(w, h, DluRect::new(2, 355, 303, 12))),
        fixed_rect_leaf(BATTLE_IN_GAME_OPTIONS_BUTTON_IDS[0], battle_sidebttn_rect(w, h, SOUND_DLU)),
        fixed_rect_leaf(BATTLE_IN_GAME_OPTIONS_BUTTON_IDS[1], battle_sidebttn_rect(w, h, KEYBOARD_DLU)),
        fixed_rect_leaf(BATTLE_IN_GAME_OPTIONS_BUTTON_IDS[2], battle_sidebttn_rect(w, h, BACK_DLU)),
    ];
    root_with_fixed_children("battle_in_game_options", crate::geometry::Size2 { width: w, height: h }, children)
}

/// 在给定视口求解局内选项 snapshot。
pub fn solve_battle_in_game_options_at(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    LayoutEngine.solve(
        Viewport { size: crate::geometry::Size2 { width: w, height: h }, ..Viewport::default() },
        &battle_in_game_options_layout_tree(viewport_w, viewport_h),
    )
}

/// 设计基准 800×600 求解。
pub fn solve_battle_in_game_options() -> LayoutSnapshot {
    solve_battle_in_game_options_at(BATTLE_PAUSE_BASE_W as u32, BATTLE_PAUSE_BASE_H as u32)
}
