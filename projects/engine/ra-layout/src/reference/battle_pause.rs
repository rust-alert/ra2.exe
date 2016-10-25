//! 对局暂停菜单专属 layout（几何权威为 `solve_battle_pause`）。
//!
//! **不是** [`super::battle_hud`]，也**不是**主菜单 [`super::shell_chrome::solve_shell_page`]。
//! **不是**局内选项 `0xBBB` 的右缘 `SIDEBTTN` 列。
//!
//! 对齐 vera 局内菜单呈现：全屏压暗 + **居中卡片** + 竖排主钮（原生 dialog 模板
//! 尚未落盘前的可玩口径；禁止把暂停钮钉在 HUD 侧栏上来冒充）。

use crate::{
    BATTLE_PAUSE_MENU_BUTTON_IDS,
    geometry::Rect,
    reference::DluRect,
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

/// 设计基准宽（与壳层 / 局内选项同口径）。
pub const BATTLE_PAUSE_BASE_W: f32 = 800.0;
/// 设计基准高。
pub const BATTLE_PAUSE_BASE_H: f32 = 600.0;

/// 居中卡片宽（对齐 vera `CARD_WIDTH`）。
pub const BATTLE_PAUSE_CARD_W: f32 = 340.0;
/// 卡片内主钮宽。
pub const BATTLE_PAUSE_BUTTON_W: f32 = 240.0;
/// 卡片内主钮高。
pub const BATTLE_PAUSE_BUTTON_H: f32 = 40.0;
/// 主钮间距。
pub const BATTLE_PAUSE_BUTTON_GAP: f32 = 10.0;
/// 卡片顶区内边距（标题区）。
pub const BATTLE_PAUSE_CARD_PAD_TOP: f32 = 28.0;
/// 卡片底区内边距。
pub const BATTLE_PAUSE_CARD_PAD_BOTTOM: f32 = 20.0;
/// 标题区高度。
pub const BATTLE_PAUSE_TITLE_H: f32 = 36.0;

/// `sidebttn.shp` 画布宽（仅局内选项 `0xBBB` / 同族 owner-draw 使用）。
pub const BATTLE_PAUSE_SIDEBTTN_W: f32 = 125.0;
/// `sidebttn.shp` 画布高。
pub const BATTLE_PAUSE_SIDEBTTN_H: f32 = 25.0;
/// 钮左缘：`x = screen_w - inset`（仅 `0xBBB` owner-draw）。
pub const BATTLE_PAUSE_BUTTON_RIGHT_INSET: f32 = 147.0;

/// 相对 800×600 基准的居中偏移（宽屏 / 高屏）。
pub fn battle_pause_center_offset(screen: f32, base: f32) -> f32 {
    ((screen - base) * 0.5).max(0.0)
}

/// 将资源 DLU 换成 `0xBBB` owner-draw 钮：右缘钉死，宽高钉 `SIDEBTTN`，Y 取 DLU + 居中偏移。
///
/// 仅供局内选项等 `0xBBB` 同族使用；**暂停菜单 / 放弃确认不要走这里**。
pub fn battle_sidebttn_rect(screen_w: f32, screen_h: f32, dlu: DluRect) -> Rect {
    use crate::reference::MS_SANS_SERIF_8PT;
    let origin = dlu.to_design_px(MS_SANS_SERIF_8PT);
    let dy = battle_pause_center_offset(screen_h, BATTLE_PAUSE_BASE_H);
    let x = (screen_w - BATTLE_PAUSE_BUTTON_RIGHT_INSET).max(0.0);
    let y = (origin.y + dy).max(0.0);
    Rect::from_xywh(x, y, BATTLE_PAUSE_SIDEBTTN_W, BATTLE_PAUSE_SIDEBTTN_H)
}

fn pause_card_height(button_count: usize) -> f32 {
    BATTLE_PAUSE_CARD_PAD_TOP
        + BATTLE_PAUSE_TITLE_H
        + (button_count as f32) * BATTLE_PAUSE_BUTTON_H
        + ((button_count.saturating_sub(1)) as f32) * BATTLE_PAUSE_BUTTON_GAP
        + BATTLE_PAUSE_CARD_PAD_BOTTOM
}

/// 居中暂停卡片外框。
pub fn battle_pause_card_rect(screen_w: f32, screen_h: f32, button_count: usize) -> Rect {
    let card_h = pause_card_height(button_count);
    let x = ((screen_w - BATTLE_PAUSE_CARD_W) * 0.5).max(0.0);
    let y = ((screen_h - card_h) * 0.5).max(0.0);
    Rect::from_xywh(x, y, BATTLE_PAUSE_CARD_W, card_h)
}

/// 卡片内第 `index` 个主钮（自上而下）。
pub fn battle_pause_card_button_rect(screen_w: f32, screen_h: f32, button_count: usize, index: usize) -> Rect {
    let card = battle_pause_card_rect(screen_w, screen_h, button_count);
    let x = card.x + (BATTLE_PAUSE_CARD_W - BATTLE_PAUSE_BUTTON_W) * 0.5;
    let y = card.y + BATTLE_PAUSE_CARD_PAD_TOP + BATTLE_PAUSE_TITLE_H + (index as f32) * (BATTLE_PAUSE_BUTTON_H + BATTLE_PAUSE_BUTTON_GAP);
    Rect::from_xywh(x, y, BATTLE_PAUSE_BUTTON_W, BATTLE_PAUSE_BUTTON_H)
}

/// 卡片标题区。
pub fn battle_pause_card_title_rect(screen_w: f32, screen_h: f32, button_count: usize) -> Rect {
    let card = battle_pause_card_rect(screen_w, screen_h, button_count);
    Rect::from_xywh(card.x + 16.0, card.y + 10.0, BATTLE_PAUSE_CARD_W - 32.0, BATTLE_PAUSE_TITLE_H)
}

/// 暂停菜单布局树（窗口 / 设计像素）：全屏 dim + 居中卡片 + 四钮。
pub fn battle_pause_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let n = BATTLE_PAUSE_MENU_BUTTON_IDS.len();
    let mut children = vec![
        fixed_rect_leaf("dim", Rect::from_xywh(0.0, 0.0, w, h)),
        fixed_rect_leaf("card", battle_pause_card_rect(w, h, n)),
        fixed_rect_leaf("title", battle_pause_card_title_rect(w, h, n)),
    ];
    for (i, id) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().enumerate() {
        children.push(fixed_rect_leaf(*id, battle_pause_card_button_rect(w, h, n, i)));
    }
    root_with_fixed_children("battle_pause", crate::geometry::Size2 { width: w, height: h }, children)
}

/// 在给定视口求解暂停菜单 snapshot。
pub fn solve_battle_pause_at(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    LayoutEngine.solve(Viewport { size: crate::geometry::Size2 { width: w, height: h }, ..Viewport::default() }, &battle_pause_layout_tree(viewport_w, viewport_h))
}

/// 设计基准 800×600 求解（占位 `RenderPlan` / 无窗口测试）。
pub fn solve_battle_pause() -> LayoutSnapshot {
    solve_battle_pause_at(BATTLE_PAUSE_BASE_W as u32, BATTLE_PAUSE_BASE_H as u32)
}
