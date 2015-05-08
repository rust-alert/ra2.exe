//! 对局 HUD → `LayoutNode`（视口像素，非壳层 800×600）。

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
    bottom_strip: Rect,
    opt_btn: Rect,
    diplo_btn: Rect,
}

fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

fn compute_battle_hud_rects(viewport_w: u32, viewport_h: u32) -> BattleHudRects {
    let w = viewport_w.max(1) as i32;
    let h = viewport_h.max(1) as i32;
    let panel_w = (RightPanelChrome::shell_defaults().panel_w as i32).min(w).max(1);
    let panel_x = (w - panel_w).max(0);
    let credits_h = 28.min(h).max(1);
    let top_h = 24.min((h - credits_h).max(1));
    let radar_h = 120.min((h - credits_h - top_h).max(1));
    let radar_y = credits_h + top_h;
    let side1_h = 36.min((h - (radar_y + radar_h)).max(1));
    let side1_y = radar_y + radar_h;
    let bottom_h = 48.min(h / 8).max(24).min(h);
    let side3_h = 28.min((h - bottom_h - (side1_y + side1_h)).max(1));
    let cameo_bottom = (h - bottom_h - side3_h).max(side1_y + side1_h);
    let cameo_y = side1_y + side1_h;
    let cameo_h = (cameo_bottom - cameo_y).max(1);
    let side3_y = cameo_y + cameo_h;
    let btn_w = ((panel_w - 12) / 2).max(1);
    let btn_h = 22.min(side1_h).max(1);
    let bottom_y = h - bottom_h;
    BattleHudRects {
        sidebar: rect_i(panel_x, 0, panel_w, h),
        credits: rect_i(panel_x, 0, panel_w, credits_h),
        top: rect_i(panel_x, credits_h, panel_w, top_h),
        radar: rect_i(panel_x, radar_y, panel_w, radar_h),
        side1: rect_i(panel_x, side1_y, panel_w, side1_h),
        cameo_band: rect_i(panel_x, cameo_y, panel_w, cameo_h),
        side3: rect_i(panel_x, side3_y, panel_w, side3_h),
        addon: rect_i(panel_x, side3_y, panel_w, side3_h.min(20).max(1)),
        repair: rect_i(panel_x + 4, side1_y + 4, btn_w, btn_h),
        sell: rect_i(panel_x + 8 + btn_w, side1_y + 4, btn_w, btn_h),
        bottom_strip: rect_i(0, bottom_y, w, bottom_h),
        opt_btn: rect_i(8, bottom_y + 8, 72, (bottom_h - 16).max(1)),
        diplo_btn: rect_i(88, bottom_y + 8, 72, (bottom_h - 16).max(1)),
    }
}

/// 对局 HUD 布局树：右栏 chrome 槽位 + 底栏按钮。
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
}
