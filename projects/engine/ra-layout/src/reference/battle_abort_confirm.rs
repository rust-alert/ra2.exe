//! 对局放弃确认（暂停菜单二级页，几何权威为 `solve_battle_abort_confirm`）。
//!
//! 与暂停菜单同族：全屏 dim + **居中卡片**（Leave / Cancel）。**不是**主菜单壳，
//! **不是** battle HUD，**也不是** `0xBBB` 右缘 `SIDEBTTN` 列。

use crate::{
    geometry::Rect,
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

use super::battle_pause::{
    BATTLE_PAUSE_BASE_H, BATTLE_PAUSE_BASE_W, BATTLE_PAUSE_BUTTON_GAP, BATTLE_PAUSE_BUTTON_H, BATTLE_PAUSE_BUTTON_W,
    BATTLE_PAUSE_CARD_PAD_BOTTOM, BATTLE_PAUSE_CARD_PAD_TOP, BATTLE_PAUSE_CARD_W, BATTLE_PAUSE_TITLE_H,
};

/// 放弃确认钮 id（Leave / Cancel）。
pub const BATTLE_ABORT_CONFIRM_BUTTON_IDS: [&str; 2] = ["leave", "cancel"];

fn abort_card_height() -> f32 {
    let prompt_h = 56.0;
    BATTLE_PAUSE_CARD_PAD_TOP
        + BATTLE_PAUSE_TITLE_H
        + prompt_h
        + 2.0 * BATTLE_PAUSE_BUTTON_H
        + BATTLE_PAUSE_BUTTON_GAP
        + BATTLE_PAUSE_CARD_PAD_BOTTOM
}

fn abort_card_rect(screen_w: f32, screen_h: f32) -> Rect {
    let card_h = abort_card_height();
    let x = ((screen_w - BATTLE_PAUSE_CARD_W) * 0.5).max(0.0);
    let y = ((screen_h - card_h) * 0.5).max(0.0);
    Rect::from_xywh(x, y, BATTLE_PAUSE_CARD_W, card_h)
}

fn abort_prompt_rect(screen_w: f32, screen_h: f32) -> Rect {
    let card = abort_card_rect(screen_w, screen_h);
    Rect::from_xywh(card.x + 24.0, card.y + BATTLE_PAUSE_CARD_PAD_TOP + BATTLE_PAUSE_TITLE_H, BATTLE_PAUSE_CARD_W - 48.0, 56.0)
}

fn abort_button_rect(screen_w: f32, screen_h: f32, index: usize) -> Rect {
    let card = abort_card_rect(screen_w, screen_h);
    let x = card.x + (BATTLE_PAUSE_CARD_W - BATTLE_PAUSE_BUTTON_W) * 0.5;
    let y = card.y
        + BATTLE_PAUSE_CARD_PAD_TOP
        + BATTLE_PAUSE_TITLE_H
        + 56.0
        + (index as f32) * (BATTLE_PAUSE_BUTTON_H + BATTLE_PAUSE_BUTTON_GAP);
    Rect::from_xywh(x, y, BATTLE_PAUSE_BUTTON_W, BATTLE_PAUSE_BUTTON_H)
}

/// 放弃确认布局树。
pub fn battle_abort_confirm_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let children = vec![
        fixed_rect_leaf("dim", Rect::from_xywh(0.0, 0.0, w, h)),
        fixed_rect_leaf("card", abort_card_rect(w, h)),
        fixed_rect_leaf("title", {
            let card = abort_card_rect(w, h);
            Rect::from_xywh(card.x + 16.0, card.y + 10.0, BATTLE_PAUSE_CARD_W - 32.0, BATTLE_PAUSE_TITLE_H)
        }),
        fixed_rect_leaf("prompt", abort_prompt_rect(w, h)),
        fixed_rect_leaf(BATTLE_ABORT_CONFIRM_BUTTON_IDS[0], abort_button_rect(w, h, 0)),
        fixed_rect_leaf(BATTLE_ABORT_CONFIRM_BUTTON_IDS[1], abort_button_rect(w, h, 1)),
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
