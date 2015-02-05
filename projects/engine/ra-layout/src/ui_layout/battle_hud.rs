//! 对局 HUD 布局。

use super::*;


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

/// 按视口计算对局 HUD 布局（右栏宽对齐壳层 `RIGHT_PANEL_W`）。
pub fn battle_hud_layout(viewport_w: u32, viewport_h: u32) -> BattleHudLayout {
    let w = viewport_w.max(1) as i32;
    let h = viewport_h.max(1) as i32;
    let panel_w = RIGHT_PANEL_W.min(w).max(1);
    let panel_x = (w - panel_w).max(0);
    let sidebar = RectPx::new(panel_x, 0, panel_w, h);
    let credits_h = 28.min(h).max(1);
    let credits = RectPx::new(panel_x, 0, panel_w, credits_h);
    let top_h = 24.min((h - credits_h).max(1));
    let top = RectPx::new(panel_x, credits.y + credits.h, panel_w, top_h);
    let radar_h = 120.min((h - credits_h - top_h).max(1));
    let radar = RectPx::new(panel_x, top.y + top.h, panel_w, radar_h);
    let side1_h = 36.min((h - (radar.y + radar.h)).max(1));
    let side1 = RectPx::new(panel_x, radar.y + radar.h, panel_w, side1_h);
    let bottom_h = 48.min(h / 8).max(24).min(h);
    let side3_h = 28.min((h - bottom_h - (side1.y + side1.h)).max(1));
    let cameo_bottom = (h - bottom_h - side3_h).max(side1.y + side1.h);
    let cameo_band = RectPx::new(panel_x, side1.y + side1.h, panel_w, (cameo_bottom - (side1.y + side1.h)).max(1));
    let side3 = RectPx::new(panel_x, cameo_band.y + cameo_band.h, panel_w, side3_h);
    let addon = RectPx::new(panel_x, side3.y, panel_w, side3_h.min(20).max(1));
    let btn_w = ((panel_w - 12) / 2).max(1);
    let btn_h = 22.min(side1_h).max(1);
    let repair = RectPx::new(panel_x + 4, side1.y + 4, btn_w, btn_h);
    let sell = RectPx::new(panel_x + 8 + btn_w, side1.y + 4, btn_w, btn_h);
    let bottom_strip = RectPx::new(0, h - bottom_h, w, bottom_h);
    let opt_btn = RectPx::new(8, bottom_strip.y + 8, 72, (bottom_h - 16).max(1));
    let diplo_btn = RectPx::new(88, bottom_strip.y + 8, 72, (bottom_h - 16).max(1));
    BattleHudLayout {
        sidebar,
        credits,
        top,
        radar,
        side1,
        cameo_band,
        side3,
        addon,
        repair,
        sell,
        bottom_strip,
        opt_btn,
        diplo_btn,
    }
}
