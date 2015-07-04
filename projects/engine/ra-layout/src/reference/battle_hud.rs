//! 对局 HUD → `LayoutNode`（视口像素，非壳层 800×600）。
//!
//! 几何对齐零售对局侧栏：仅右侧栏贴满屏高，战术区铺到屏底，无全宽底栏。

use crate::{
    geometry::{Rect, Size2},
    policy::RightPanelChrome,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

/// 对局 HUD 各槽位设计矩形（与过渡期 `battle_hud_layout` 同构）。
#[derive(Debug, Clone, Copy)]
struct BattleHudRects {
    sidebar: Rect,
    credits: Rect,
    top: Rect,
    radar: Rect,
    side1: Rect,
    cameo_band: Rect,
    side3: Rect,
    addon: Rect,
    repair: Rect,
    sell: Rect,
    /// 右栏底脚条带（仅侧栏内，不是全宽底栏）。
    bottom_strip: Rect,
    opt_btn: Rect,
    diplo_btn: Rect,
}

fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

/// 侧栏顶段固定高度（credits + top + radar + side1）。
const CREDITS_H: i32 = 16;
const TOP_H: i32 = 32;
const RADAR_H: i32 = 110;
const SIDE1_H: i32 = 69;
const SIDE3_H: i32 = 26;
const ADDON_H: i32 = 48;
const REPAIR_SELL_W: i32 = 52;
const REPAIR_SELL_H: i32 = 32;
const REPAIR_X_OFF: i32 = 32;
const SELL_X_OFF: i32 = 84;
const REPAIR_SELL_Y_OFF: i32 = 8;
const FOOTER_BTN_W: i32 = 60;
const FOOTER_BTN_H: i32 = 24;

fn compute_battle_hud_rects(viewport_w: u32, viewport_h: u32) -> BattleHudRects {
    let w = viewport_w.max(1) as i32;
    let h = viewport_h.max(1) as i32;
    let panel_w = (RightPanelChrome::shell_defaults().panel_w as i32).min(w).max(1);
    let panel_x = (w - panel_w).max(0);

    let credits_h = CREDITS_H.min(h).max(1);
    let top_h = TOP_H.min((h - credits_h).max(1));
    let radar_h = RADAR_H.min((h - credits_h - top_h).max(1));
    let radar_y = credits_h + top_h;
    let side1_h = SIDE1_H.min((h - (radar_y + radar_h)).max(1));
    let side1_y = radar_y + radar_h;

    // 底脚：side3 + addon 叠在右栏底，战术区不裁切。
    let addon_h = ADDON_H.min(h / 4).max(1).min(h);
    let side3_h = SIDE3_H.min((h - addon_h).max(1));
    let addon_y = (h - addon_h).max(0);
    let side3_y = (addon_y - side3_h).max(0);

    let cameo_y = side1_y + side1_h;
    let cameo_bottom = side3_y.max(cameo_y);
    let cameo_h = (cameo_bottom - cameo_y).max(1);

    let repair_w = REPAIR_SELL_W.min(panel_w / 2).max(1);
    let repair_h = REPAIR_SELL_H.min(side1_h.saturating_sub(4)).max(1);
    let sell_x = (panel_x + SELL_X_OFF).min(panel_x + panel_w - repair_w);

    let footer_btn_h = FOOTER_BTN_H.min(addon_h.saturating_sub(8)).max(1);
    let footer_btn_w = FOOTER_BTN_W.min((panel_w - 20) / 2).max(1);
    let footer_btn_y = addon_y + ((addon_h - footer_btn_h) / 2).max(0);

    BattleHudRects {
        sidebar: rect_i(panel_x, 0, panel_w, h),
        credits: rect_i(panel_x, 0, panel_w, credits_h),
        top: rect_i(panel_x, credits_h, panel_w, top_h),
        radar: rect_i(panel_x, radar_y, panel_w, radar_h),
        side1: rect_i(panel_x, side1_y, panel_w, side1_h),
        cameo_band: rect_i(panel_x, cameo_y, panel_w, cameo_h),
        side3: rect_i(panel_x, side3_y, panel_w, side3_h),
        addon: rect_i(panel_x, addon_y, panel_w, addon_h),
        repair: rect_i(
            panel_x + REPAIR_X_OFF.min(panel_w.saturating_sub(repair_w)),
            side1_y + REPAIR_SELL_Y_OFF.min(side1_h.saturating_sub(repair_h)),
            repair_w,
            repair_h,
        ),
        sell: rect_i(
            sell_x,
            side1_y + REPAIR_SELL_Y_OFF.min(side1_h.saturating_sub(repair_h)),
            repair_w,
            repair_h,
        ),
        // 仅右栏底脚，供 chrome / 文案锚点；不再横贯战术区。
        bottom_strip: rect_i(panel_x, side3_y, panel_w, (h - side3_y).max(1)),
        opt_btn: rect_i(panel_x + 8, footer_btn_y, footer_btn_w, footer_btn_h),
        diplo_btn: rect_i(panel_x + 12 + footer_btn_w, footer_btn_y, footer_btn_w, footer_btn_h),
    }
}

/// 对局 HUD 布局树：右栏 chrome 槽位（无全宽底栏）。
pub fn battle_hud_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let r = compute_battle_hud_rects(viewport_w, viewport_h);
    let children = vec![
        fixed_rect_leaf("sidebar", r.sidebar),
        fixed_rect_leaf("credits", r.credits),
        fixed_rect_leaf("top", r.top),
        fixed_rect_leaf("radar", r.radar),
        fixed_rect_leaf("side1", r.side1),
        fixed_rect_leaf("cameo_band", r.cameo_band),
        fixed_rect_leaf("side3", r.side3),
        fixed_rect_leaf("addon", r.addon),
        fixed_rect_leaf("repair", r.repair),
        fixed_rect_leaf("sell", r.sell),
        fixed_rect_leaf("bottom_strip", r.bottom_strip),
        fixed_rect_leaf("opt_btn", r.opt_btn),
        fixed_rect_leaf("diplo_btn", r.diplo_btn),
    ];
    root_with_fixed_children(
        "battle_hud",
        Size2 {
            width: w,
            height: h,
        },
        children,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutEngine, Viewport};

    #[test]
    fn battle_hud_tree_matches_ui_layout() {
        for (vw, vh) in [(640u32, 480u32), (800, 600), (1280, 720), (2560, 1440)] {
            let snap = LayoutEngine.solve(
                Viewport {
                    size: Size2 {
                        width: vw.max(1) as f32,
                        height: vh.max(1) as f32,
                    },
                    ..Viewport::default()
                },
                &battle_hud_layout_tree(vw, vh),
            );
            let legacy = crate::ui_layout::battle_hud_layout(vw, vh);
            for (id, cell) in [
                ("sidebar", legacy.sidebar),
                ("credits", legacy.credits),
                ("top", legacy.top),
                ("radar", legacy.radar),
                ("side1", legacy.side1),
                ("cameo_band", legacy.cameo_band),
                ("side3", legacy.side3),
                ("addon", legacy.addon),
                ("repair", legacy.repair),
                ("sell", legacy.sell),
                ("bottom_strip", legacy.bottom_strip),
                ("opt_btn", legacy.opt_btn),
                ("diplo_btn", legacy.diplo_btn),
            ] {
                let got = snap.get(id).expect(id).layout.rect;
                assert_eq!(got.x as i32, cell.x, "{vw}x{vh} {id} x");
                assert_eq!(got.y as i32, cell.y, "{vw}x{vh} {id} y");
                assert_eq!(got.width as i32, cell.w, "{vw}x{vh} {id} w");
                assert_eq!(got.height as i32, cell.h, "{vw}x{vh} {id} h");
            }
        }
    }

    #[test]
    fn battle_hud_has_no_full_width_bottom_strip() {
        let r = compute_battle_hud_rects(800, 600);
        assert_eq!(r.sidebar.x as i32, 800 - 168);
        assert_eq!(r.sidebar.width as i32, 168);
        assert_eq!(r.sidebar.height as i32, 600);
        // 底脚只在右栏内。
        assert_eq!(r.bottom_strip.x as i32, r.sidebar.x as i32);
        assert_eq!(r.bottom_strip.width as i32, 168);
        assert!(r.bottom_strip.x as i32 > 0);
        // 选项 / 外交在右栏底脚，不在战术区左下。
        assert!(r.opt_btn.x as i32 >= r.sidebar.x as i32);
        assert!(r.diplo_btn.x as i32 >= r.sidebar.x as i32);
        // 修理 / 出售在 side1 带内。
        assert!(r.repair.y as i32 >= r.side1.y as i32);
        assert!(r.sell.y as i32 >= r.side1.y as i32);
        assert_eq!(r.credits.height as i32, CREDITS_H);
        assert_eq!(r.top.height as i32, TOP_H);
        assert_eq!(r.radar.height as i32, RADAR_H);
        assert_eq!(r.side1.height as i32, SIDE1_H);
    }
}
