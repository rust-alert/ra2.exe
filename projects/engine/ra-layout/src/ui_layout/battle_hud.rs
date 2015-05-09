//! 对局 HUD 布局。

use super::*;
use crate::{battle_hud_layout_tree, LayoutEngine, Size2, Viewport};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleHudLayout {
    /// 右栏整体。
    pub sidebar: RectPx,
    /// 资金条（`credits.shp`）。
    pub credits: RectPx,
    /// 雷达顶盖（`top.shp`）。
    pub top: RectPx,
    /// 雷达窗（`radar.shp`）。
    pub radar: RectPx,
    /// 侧栏上段（`side1.shp`）。
    pub side1: RectPx,
    /// Cameo 平铺带（`side2.shp`）。
    pub cameo_band: RectPx,
    /// 侧栏下段（`side3.shp`）。
    pub side3: RectPx,
    /// 附加条（`addon.shp`）。
    pub addon: RectPx,
    /// 修理钮（`repair.shp`）。
    pub repair: RectPx,
    /// 出售钮（`sell.shp`）。
    pub sell: RectPx,
    /// 底栏条带。
    pub bottom_strip: RectPx,
    /// 选项钮（`optbtn.shp`）。
    pub opt_btn: RectPx,
    /// 外交钮（`diplobtn.shp`）。
    pub diplo_btn: RectPx,
}

impl BattleHudLayout {
    /// 世界层可视矩形：左起至侧栏左缘，上起至底栏上缘（不含右栏与底栏 chrome）。
    pub fn world_viewport(&self) -> RectPx {
        let w = self.sidebar.x.max(0);
        let h = self.bottom_strip.y.max(0);
        RectPx::new(0, 0, w, h)
    }
}

/// 按视口计算对局 HUD 布局（右栏宽对齐壳层 `RIGHT_PANEL_W`）。
///
/// 几何投影自 `battle_hud_layout_tree` → `LayoutSnapshot`。
pub fn battle_hud_layout(viewport_w: u32, viewport_h: u32) -> BattleHudLayout {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let snap = LayoutEngine.solve(
        Viewport {
            size: Size2 {
                width: w,
                height: h,
            },
            ..Viewport::default()
        },
        &battle_hud_layout_tree(viewport_w, viewport_h),
    );
    BattleHudLayout {
        sidebar: rect_px_from_snapshot(&snap, "sidebar"),
        credits: rect_px_from_snapshot(&snap, "credits"),
        top: rect_px_from_snapshot(&snap, "top"),
        radar: rect_px_from_snapshot(&snap, "radar"),
        side1: rect_px_from_snapshot(&snap, "side1"),
        cameo_band: rect_px_from_snapshot(&snap, "cameo_band"),
        side3: rect_px_from_snapshot(&snap, "side3"),
        addon: rect_px_from_snapshot(&snap, "addon"),
        repair: rect_px_from_snapshot(&snap, "repair"),
        sell: rect_px_from_snapshot(&snap, "sell"),
        bottom_strip: rect_px_from_snapshot(&snap, "bottom_strip"),
        opt_btn: rect_px_from_snapshot(&snap, "opt_btn"),
        diplo_btn: rect_px_from_snapshot(&snap, "diplo_btn"),
    }
}
