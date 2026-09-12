//! 对局暂停菜单专属 layout（几何权威为 `solve_battle_pause`）。
//!
//! **不是** [`super::battle_hud`]，也**不是**主菜单 [`super::shell_chrome::solve_shell_page`]。
//! 局内暂停主钮与局内选项 `0xBBB` 同走 owner-draw **type 2**：`SIDEBTTN.SHP` +
//! `SIDEBAR.PAL`，右缘 inset 147、画布 125×25；竖向取资源 DLU + 相对 800×600 居中。
//! 禁止用自制黄框卡片冒充原版 UI。

use crate::{
    BATTLE_PAUSE_MENU_BUTTON_IDS,
    geometry::Rect,
    reference::{DluRect, MS_SANS_SERIF_8PT},
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

/// 设计基准宽（与壳层 / 局内选项同口径）。
pub const BATTLE_PAUSE_BASE_W: f32 = 800.0;
/// 设计基准高。
pub const BATTLE_PAUSE_BASE_H: f32 = 600.0;
/// `sidebttn.shp` 画布宽。
pub const BATTLE_PAUSE_BUTTON_W: f32 = 125.0;
/// `sidebttn.shp` 画布高。
pub const BATTLE_PAUSE_BUTTON_H: f32 = 25.0;
/// 钮左缘：`x = screen_w - inset`（与局内选项 owner-draw 同 inset）。
pub const BATTLE_PAUSE_BUTTON_RIGHT_INSET: f32 = 147.0;
/// 同 [`BATTLE_PAUSE_BUTTON_W`]（`0xBBB` 同族别名）。
pub const BATTLE_PAUSE_SIDEBTTN_W: f32 = BATTLE_PAUSE_BUTTON_W;
/// 同 [`BATTLE_PAUSE_BUTTON_H`]。
pub const BATTLE_PAUSE_SIDEBTTN_H: f32 = BATTLE_PAUSE_BUTTON_H;

/// Options / Game Controls 资源 DLU（`0xBBB` Sound 同列）。
const OPTIONS_DLU: DluRect = DluRect::new(425, 122, 108, 23);
/// Fullscreen 资源 DLU（Sound→Keyboard 同列下一格）。
const FULLSCREEN_DLU: DluRect = DluRect::new(425, 149, 108, 23);
/// Abort 资源 DLU（再下一格，27 DLU 行距）。
const ABORT_DLU: DluRect = DluRect::new(425, 176, 108, 23);
/// Resume 资源 DLU（`0xBBB` Back 同列贴底族）。
const RESUME_DLU: DluRect = DluRect::new(425, 346, 108, 23);

const BUTTON_DLUS: [DluRect; 4] = [OPTIONS_DLU, FULLSCREEN_DLU, ABORT_DLU, RESUME_DLU];

/// 相对 800×600 基准的垂直居中偏移（宽屏 / 高屏）。
pub fn battle_pause_center_offset(screen: f32, base: f32) -> f32 {
    ((screen - base) * 0.5).max(0.0)
}

/// 将资源 DLU 换成 owner-draw 钮：右缘钉死，宽高钉 `SIDEBTTN`，Y 取 DLU + 居中偏移。
pub fn battle_sidebttn_rect(screen_w: f32, screen_h: f32, dlu: DluRect) -> Rect {
    let origin = dlu.to_design_px(MS_SANS_SERIF_8PT);
    let dy = battle_pause_center_offset(screen_h, BATTLE_PAUSE_BASE_H);
    let x = (screen_w - BATTLE_PAUSE_BUTTON_RIGHT_INSET).max(0.0);
    let y = (origin.y + dy).max(0.0);
    Rect::from_xywh(x, y, BATTLE_PAUSE_BUTTON_W, BATTLE_PAUSE_BUTTON_H)
}

/// 暂停菜单布局树（窗口 / 设计像素）：全屏 dim + 右缘 `SIDEBTTN` 四钮。
pub fn battle_pause_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let mut children = vec![fixed_rect_leaf("dim", Rect::from_xywh(0.0, 0.0, w, h))];
    for (id, dlu) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().zip(BUTTON_DLUS.iter()) {
        children.push(fixed_rect_leaf(*id, battle_sidebttn_rect(w, h, *dlu)));
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
