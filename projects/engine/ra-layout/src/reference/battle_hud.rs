//! 对局 HUD → `LayoutNode`（视口像素，非壳层 800×600）。
//!
//! 几何对齐零售对局侧栏：右侧栏贴满屏高；战术区底边留出命令条高度。
//! 命令条横贯侧栏左侧（`lendcap` / `buttonNN` / `rendcap`），不是选项/外交假底栏。
//! 盟军 `sidec01` 与苏军 `sidec02` 钮面画布尺寸不同，由 `BattleHudChromeMetrics` 区分。

use crate::{
    geometry::{Rect, Size2},
    policy::RightPanelChrome,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

/// 侧栏共用竖向槽位高度（两阵营一致）。
const CREDITS_H: i32 = 16;
const TOP_H: i32 = 32;
const RADAR_H: i32 = 110;
const SIDE1_H: i32 = 69;
const SIDE3_H: i32 = 26;
/// `addon.shp` 画布高（勿压成 48，否则底脚鹰标/双蓝板变形）。
const ADDON_H: i32 = 63;
/// 战术区底边命令条高度（`lendcap` / `buttonNN` / `rendcap` 画布高）。
pub const COMMAND_BAR_H: i32 = 32;
/// 命令条左端盖宽（`lendcap.shp`）。
pub const COMMAND_LENDCAP_W: i32 = 28;
/// 命令条右端盖宽（`rendcap.shp`）。
pub const COMMAND_RENDCAP_W: i32 = 28;
/// 命令钮画布宽（`button00`… / `bttnbkgd`）。
pub const COMMAND_BUTTON_W: i32 = 52;

/// 阵营侧栏 chrome 画布尺寸与槽位偏移（像素）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleHudChromeMetrics {
    /// `repair`/`sell` 画布宽。
    pub repair_sell_w: i32,
    /// `repair`/`sell` 画布高。
    pub repair_sell_h: i32,
    /// 修理钮相对侧栏左缘的 x 偏移。
    pub repair_x: i32,
    /// 出售钮相对侧栏左缘的 x 偏移。
    pub sell_x: i32,
    /// 修理/出售钮相对 `side1` 顶边的 y 偏移。
    pub repair_y: i32,
    /// 分类页签画布宽。
    pub tab_w: i32,
    /// 分类页签画布高。
    pub tab_h: i32,
    /// 首个页签相对侧栏左缘的 x 偏移。
    pub tab_x: i32,
    /// 相邻页签间距。
    pub tab_gap: i32,
    /// `optbtn`/`diplobtn` 画布宽（贴在 `top.shp` 双钮槽，不是底脚）。
    pub top_btn_w: i32,
    /// `optbtn`/`diplobtn` 画布高。
    pub top_btn_h: i32,
    /// 顶栏左钮相对侧栏左缘的 x 偏移。
    pub top_btn_x: i32,
    /// 顶栏双钮间距。
    pub top_btn_gap: i32,
    /// 顶栏钮相对 `top` 顶边的 y 偏移。
    pub top_btn_y: i32,
    /// `powerp` 电表条带宽度。
    pub power_w: i32,
}

impl BattleHudChromeMetrics {
    /// 盟军 `sidec01` 画布度量。
    pub const fn allied() -> Self {
        Self {
            repair_sell_w: 64,
            repair_sell_h: 31,
            repair_x: 20,
            sell_x: 84,
            repair_y: 8,
            tab_w: 28,
            tab_h: 27,
            tab_x: 27,
            tab_gap: 2,
            top_btn_w: 72,
            top_btn_h: 18,
            top_btn_x: 12,
            top_btn_gap: 0,
            top_btn_y: 7,
            power_w: 12,
        }
    }

    /// 苏军 `sidec02` 画布度量。
    pub const fn soviet() -> Self {
        Self {
            repair_sell_w: 52,
            repair_sell_h: 32,
            repair_x: 32,
            sell_x: 84,
            repair_y: 8,
            tab_w: 32,
            tab_h: 28,
            tab_x: 20,
            tab_gap: 2,
            top_btn_w: 72,
            top_btn_h: 22,
            top_btn_x: 12,
            top_btn_gap: 0,
            top_btn_y: 5,
            power_w: 16,
        }
    }

    /// 按嵌套包名选择度量（`sidec02` → 苏军，其余默认盟军）。
    pub fn for_mix(mix: &str) -> Self {
        let lower = mix.to_ascii_lowercase();
        if lower.contains("sidec02") {
            Self::soviet()
        } else {
            Self::allied()
        }
    }
}

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
    tabs: [Rect; 4],
    /// 右栏底脚条带（仅侧栏内，不是命令条）。
    bottom_strip: Rect,
    /// 战术区底边命令条（侧栏左缘以左，高 `COMMAND_BAR_H`）。
    command_bar: Rect,
    opt_btn: Rect,
    diplo_btn: Rect,
}

fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

fn compute_battle_hud_rects(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> BattleHudRects {
    let w = viewport_w.max(1) as i32;
    let h = viewport_h.max(1) as i32;
    let panel_w = (RightPanelChrome::shell_defaults().panel_w as i32)
        .min(w)
        .max(1);
    let panel_x = (w - panel_w).max(0);
    let command_bar_h = COMMAND_BAR_H.min(h).max(1);
    let command_bar_y = (h - command_bar_h).max(0);
    let command_bar_w = panel_x.max(1);

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

    let repair_w = metrics.repair_sell_w.min(panel_w / 2).max(1);
    let repair_h = metrics
        .repair_sell_h
        .min(side1_h.saturating_sub(4))
        .max(1);
    let sell_x = (panel_x + metrics.sell_x).min(panel_x + panel_w - repair_w);
    let repair_sell_y =
        side1_y + metrics.repair_y.min(side1_h.saturating_sub(repair_h));

    let tab_w = metrics.tab_w.min(panel_w).max(1);
    let tab_h = metrics.tab_h.min(side1_h).max(1);
    let tab_y = (side1_y + side1_h - tab_h).max(side1_y);
    let mut tab_x = panel_x + metrics.tab_x.min(panel_w.saturating_sub(tab_w));
    let mut tabs = [rect_i(0, 0, 1, 1); 4];
    for slot in &mut tabs {
        *slot = rect_i(tab_x, tab_y, tab_w, tab_h);
        tab_x += tab_w + metrics.tab_gap;
    }

    // 选项/外交贴在资金条下的 `top.shp` 双槽（原版顶栏），不是底脚。
    // 左槽为外交（折线图标），右槽为选项（圆点条图标），与零售顶栏一致。
    let top_btn_h = metrics.top_btn_h.min(top_h).max(1);
    let top_btn_w = metrics
        .top_btn_w
        .min((panel_w - metrics.top_btn_x).max(1))
        .max(1);
    let top_btn_y = credits_h + metrics.top_btn_y.min(top_h.saturating_sub(top_btn_h));
    let diplo_x = panel_x + metrics.top_btn_x.min(panel_w.saturating_sub(top_btn_w));
    let opt_x =
        (diplo_x + top_btn_w + metrics.top_btn_gap).min(panel_x + panel_w - top_btn_w);

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
            panel_x + metrics.repair_x.min(panel_w.saturating_sub(repair_w)),
            repair_sell_y,
            repair_w,
            repair_h,
        ),
        sell: rect_i(sell_x, repair_sell_y, repair_w, repair_h),
        tabs,
        // 仅右栏底脚，供 chrome / 文案锚点。
        bottom_strip: rect_i(panel_x, side3_y, panel_w, (h - side3_y).max(1)),
        // 战术区底边命令条：左端至侧栏左缘。
        command_bar: rect_i(0, command_bar_y, command_bar_w, command_bar_h),
        diplo_btn: rect_i(diplo_x, top_btn_y, top_btn_w, top_btn_h),
        opt_btn: rect_i(opt_x, top_btn_y, top_btn_w, top_btn_h),
    }
}

fn battle_hud_tree_from_rects(viewport_w: u32, viewport_h: u32, r: BattleHudRects) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
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
        fixed_rect_leaf("tab00", r.tabs[0]),
        fixed_rect_leaf("tab01", r.tabs[1]),
        fixed_rect_leaf("tab02", r.tabs[2]),
        fixed_rect_leaf("tab03", r.tabs[3]),
        fixed_rect_leaf("bottom_strip", r.bottom_strip),
        fixed_rect_leaf("command_bar", r.command_bar),
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

/// 对局 HUD 布局树：右栏 chrome 槽位 + 战术区底边命令条（默认盟军度量）。
pub fn battle_hud_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    battle_hud_layout_tree_with_metrics(
        viewport_w,
        viewport_h,
        BattleHudChromeMetrics::allied(),
    )
}

/// 对局 HUD 布局树：按阵营 chrome 度量计算槽位。
pub fn battle_hud_layout_tree_with_metrics(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> LayoutNode {
    let r = compute_battle_hud_rects(viewport_w, viewport_h, metrics);
    battle_hud_tree_from_rects(viewport_w, viewport_h, r)
}

/// 求解对局 HUD snapshot（默认盟军度量）。
pub fn solve_battle_hud(viewport_w: u32, viewport_h: u32) -> crate::LayoutSnapshot {
    solve_battle_hud_with_metrics(viewport_w, viewport_h, BattleHudChromeMetrics::allied())
}

/// 求解对局 HUD snapshot（指定阵营度量）。
pub fn solve_battle_hud_with_metrics(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> crate::LayoutSnapshot {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    crate::LayoutEngine.solve(
        crate::Viewport {
            size: Size2 {
                width: w,
                height: h,
            },
            ..crate::Viewport::default()
        },
        &battle_hud_layout_tree_with_metrics(viewport_w, viewport_h, metrics),
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
                ("tab00", legacy.tabs[0]),
                ("tab01", legacy.tabs[1]),
                ("tab02", legacy.tabs[2]),
                ("tab03", legacy.tabs[3]),
                ("bottom_strip", legacy.bottom_strip),
                ("command_bar", legacy.command_bar),
                ("opt_btn", legacy.opt_btn),
                ("diplo_btn", legacy.diplo_btn),
            ] {
                let got = snap.get(id).expect(id).layout.rect;
                assert_eq!(got.x as i32, cell.x, "{vw}x{vh} {id} x");
                assert_eq!(got.y as i32, cell.y, "{vw}x{vh} {id} y");
                assert_eq!(got.width as i32, cell.w, "{vw}x{vh} {id} w");
                assert_eq!(got.height as i32, cell.h, "{vw}x{vh} {id} h");
            }
            assert_eq!(legacy.power_meter_w, BattleHudChromeMetrics::allied().power_w);
        }
    }

    #[test]
    fn battle_hud_command_bar_spans_tactical_bottom() {
        let allied = BattleHudChromeMetrics::allied();
        let r = compute_battle_hud_rects(800, 600, allied);
        assert_eq!(r.sidebar.x as i32, 800 - 168);
        assert_eq!(r.sidebar.width as i32, 168);
        assert_eq!(r.sidebar.height as i32, 600);
        // 右栏底脚仍只在侧栏内。
        assert_eq!(r.bottom_strip.x as i32, r.sidebar.x as i32);
        assert_eq!(r.bottom_strip.width as i32, 168);
        // 命令条横贯战术区底边，右缘贴侧栏左缘。
        assert_eq!(r.command_bar.x as i32, 0);
        assert_eq!(r.command_bar.y as i32, 600 - COMMAND_BAR_H);
        assert_eq!(r.command_bar.width as i32, r.sidebar.x as i32);
        assert_eq!(r.command_bar.height as i32, COMMAND_BAR_H);
        // 选项 / 外交在资金条下的顶栏双槽，不在战术区左下、也不在底脚。
        assert!(r.opt_btn.x as i32 >= r.sidebar.x as i32);
        assert!(r.diplo_btn.x as i32 >= r.sidebar.x as i32);
        assert!(r.opt_btn.y as i32 >= r.top.y as i32);
        assert!(r.diplo_btn.y as i32 >= r.top.y as i32);
        assert!(r.opt_btn.y as i32 + r.opt_btn.height as i32 <= r.top.y as i32 + r.top.height as i32);
        assert!(r.diplo_btn.y as i32 + r.diplo_btn.height as i32 <= r.top.y as i32 + r.top.height as i32);
        // 修理 / 出售在 side1 带内。
        assert!(r.repair.y as i32 >= r.side1.y as i32);
        assert!(r.sell.y as i32 >= r.side1.y as i32);
        assert_eq!(r.credits.height as i32, CREDITS_H);
        assert_eq!(r.top.height as i32, TOP_H);
        assert_eq!(r.radar.height as i32, RADAR_H);
        assert_eq!(r.side1.height as i32, SIDE1_H);
        assert_eq!(r.side3.height as i32, SIDE3_H);
        assert_eq!(r.addon.height as i32, ADDON_H);
        assert_eq!(r.opt_btn.width as i32, allied.top_btn_w);
        assert_eq!(r.opt_btn.height as i32, allied.top_btn_h);
        assert_eq!(r.diplo_btn.width as i32, allied.top_btn_w);
        assert_eq!(r.repair.width as i32, allied.repair_sell_w);
        assert_eq!(r.repair.height as i32, allied.repair_sell_h);
        assert_eq!(r.tabs[0].width as i32, allied.tab_w);
        assert_eq!(r.tabs[0].height as i32, allied.tab_h);
    }

    #[test]
    fn allied_and_soviet_chrome_metrics_differ() {
        let allied = BattleHudChromeMetrics::allied();
        let soviet = BattleHudChromeMetrics::soviet();
        assert_ne!(allied.repair_sell_w, soviet.repair_sell_w);
        assert_ne!(allied.repair_sell_h, soviet.repair_sell_h);
        assert_ne!(allied.repair_x, soviet.repair_x);
        assert_ne!(allied.tab_w, soviet.tab_w);
        assert_ne!(allied.tab_h, soviet.tab_h);
        assert_ne!(allied.tab_x, soviet.tab_x);
        assert_ne!(allied.top_btn_h, soviet.top_btn_h);
        assert_ne!(allied.power_w, soviet.power_w);

        let a = compute_battle_hud_rects(800, 600, allied);
        let s = compute_battle_hud_rects(800, 600, soviet);
        assert_eq!(a.repair.width as i32, 64);
        assert_eq!(a.repair.height as i32, 31);
        assert_eq!(s.repair.width as i32, 52);
        assert_eq!(s.repair.height as i32, 32);
        assert_eq!(
            a.repair.x as i32 - a.sidebar.x as i32,
            allied.repair_x
        );
        assert_eq!(
            s.repair.x as i32 - s.sidebar.x as i32,
            soviet.repair_x
        );
        assert_eq!(a.tabs[0].width as i32, 28);
        assert_eq!(a.tabs[0].height as i32, 27);
        assert_eq!(s.tabs[0].width as i32, 32);
        assert_eq!(s.tabs[0].height as i32, 28);
        assert_eq!(a.opt_btn.height as i32, 18);
        assert_eq!(s.opt_btn.height as i32, 22);
        assert_eq!(
            a.tabs[0].x as i32 - a.sidebar.x as i32,
            allied.tab_x
        );
        assert_eq!(
            s.tabs[0].x as i32 - s.sidebar.x as i32,
            soviet.tab_x
        );

        assert_eq!(
            BattleHudChromeMetrics::for_mix("sidec01.mix"),
            allied
        );
        assert_eq!(
            BattleHudChromeMetrics::for_mix("sidec02.mix"),
            soviet
        );
        assert_eq!(
            BattleHudChromeMetrics::for_mix("SIDEC02.MIX"),
            soviet
        );
    }
}
