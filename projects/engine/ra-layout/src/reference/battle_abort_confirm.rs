//! 对局放弃确认（暂停菜单二级页，几何权威为 `solve_battle_abort_confirm`）。
//!
//! 与暂停菜单同族：全屏 dim + 右缘 `SIDEBTTN`。**不是**主菜单壳，也**不是** battle HUD。

use crate::{
    geometry::Rect,
    reference::{DluRect, MS_SANS_SERIF_8PT},
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

use super::battle_pause::{BATTLE_PAUSE_BASE_H, BATTLE_PAUSE_BASE_W, battle_pause_center_offset, battle_sidebttn_rect};

/// 放弃确认钮 id（Leave / Cancel）。
pub const BATTLE_ABORT_CONFIRM_BUTTON_IDS: [&str; 2] = ["leave", "cancel"];

/// Leave 资源 DLU（对齐暂停 Abort 行）。
const LEAVE_DLU: DluRect = DluRect::new(425, 176, 108, 23);
/// Cancel 资源 DLU（对齐暂停 Resume / `0xBBB` Back 行）。
const CANCEL_DLU: DluRect = DluRect::new(425, 346, 108, 23);

/// 提示文案区（相对 800×600 设计像素，再加垂直居中偏移）。
fn prompt_rect(screen_w: f32, screen_h: f32) -> Rect {
    let dy = battle_pause_center_offset(screen_h, BATTLE_PAUSE_BASE_H);
    let dx = battle_pause_center_offset(screen_w, BATTLE_PAUSE_BASE_W);
    // 左区可读提示，避开右缘钮列。
    Rect::from_xywh(dx + 48.0, dy + 200.0, 420.0, 80.0)
}

/// 放弃确认布局树。
pub fn battle_abort_confirm_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let children = vec![
        fixed_rect_leaf("dim", Rect::from_xywh(0.0, 0.0, w, h)),
        fixed_rect_leaf("prompt", prompt_rect(w, h)),
        fixed_rect_leaf(BATTLE_ABORT_CONFIRM_BUTTON_IDS[0], battle_sidebttn_rect(w, h, LEAVE_DLU)),
        fixed_rect_leaf(BATTLE_ABORT_CONFIRM_BUTTON_IDS[1], battle_sidebttn_rect(w, h, CANCEL_DLU)),
    ];
    root_with_fixed_children("battle_abort_confirm", crate::geometry::Size2 { width: w, height: h }, children)
}

/// 在给定视口求解放弃确认 snapshot。
pub fn solve_battle_abort_confirm_at(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    LayoutEngine.solve(
        Viewport { size: crate::geometry::Size2 { width: w, height: h }, ..Viewport::default() },
        &battle_abort_confirm_layout_tree(viewport_w, viewport_h),
    )
}

/// 设计基准 800×600 求解。
pub fn solve_battle_abort_confirm() -> LayoutSnapshot {
    solve_battle_abort_confirm_at(BATTLE_PAUSE_BASE_W as u32, BATTLE_PAUSE_BASE_H as u32)
}

/// 供测试：Leave DLU 转设计像素原点。
#[allow(dead_code)]
pub fn leave_dlu_design_origin() -> Rect {
    LEAVE_DLU.to_design_px(MS_SANS_SERIF_8PT)
}
