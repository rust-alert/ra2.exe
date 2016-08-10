//! 对局 HUD → `LayoutNode`（视口像素，非壳层 800×600）。
//!
//! 几何对齐零售对局侧栏：右侧栏贴满屏高；战术区底边留出命令条高度。
//! 命令条横贯侧栏左侧（`lendcap` / `buttonNN` / `rendcap`），不是选项/外交假底栏。
//! `sidec01` / `sidec02` 钮面画布尺寸不同，由 `BattleHudChromeMetrics` 区分（按 mix 索引选包，非阵营语义）。

use crate::{
    geometry::{Rect, Size2},
    policy::RightPanelChrome,
    shell::{RectPx, rect_px_from_snapshot},
    snapshot::LayoutSnapshot,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
};

/// 侧栏共用竖向槽位高度（两套已测 chrome 包一致）。
pub const CREDITS_H: i32 = 16;
pub const TOP_H: i32 = 32;
pub const RADAR_H: i32 = 110;
pub const SIDE1_H: i32 = 69;
pub const SIDE3_H: i32 = 26;
/// `addon.shp` 画布高（勿压成 48，否则底脚鹰标/双蓝板变形）。
pub const ADDON_H: i32 = 63;
/// 战术区底边命令条高度（`lendcap` / `buttonNN` / `rendcap` 画布高）。
pub const COMMAND_BAR_H: i32 = 32;
/// 命令条左端盖宽（`lendcap.shp`）。
pub const COMMAND_LENDCAP_W: i32 = 28;
/// 命令条右端盖宽（`rendcap.shp`）。
pub const COMMAND_RENDCAP_W: i32 = 28;
/// 命令钮画布宽（`button00`… / `bttnbkgd`）。
pub const COMMAND_BUTTON_W: i32 = 52;
/// 遭遇战命令条可视钮槽数（对齐 `ui.ini` `[AdvancedCommandBar]` 长度）。
pub const COMMAND_BAR_BUTTON_COUNT: usize = 6;
/// 命令条可视钮 snapshot id（`cmd0`…）。
pub const COMMAND_BAR_BUTTON_IDS: [&str; COMMAND_BAR_BUTTON_COUNT] = ["cmd0", "cmd1", "cmd2", "cmd3", "cmd4", "cmd5"];

/// 侧栏 chrome 画布尺寸与槽位偏移（像素）。
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
    /// `sidec01` 嵌套包画布度量（已测）。
    pub const fn sidec01() -> Self {
        Self {
            repair_sell_w: 64,
            repair_sell_h: 31,
            repair_x: 20,
            sell_x: 84,
            repair_y: 8,
            // `tab00`…`tab03` 零售画布 32×28；过窄会拉伸挤叠。
            tab_w: 32,
            tab_h: 28,
            tab_x: 20,
            tab_gap: 0,
            top_btn_w: 72,
            top_btn_h: 18,
            top_btn_x: 12,
            top_btn_gap: 0,
            top_btn_y: 7,
            power_w: 12,
        }
    }

    /// `sidec02` 嵌套包画布度量（已测）。
    pub const fn sidec02() -> Self {
        Self {
            repair_sell_w: 52,
            repair_sell_h: 32,
            repair_x: 32,
            sell_x: 84,
            repair_y: 8,
            tab_w: 32,
            tab_h: 28,
            tab_x: 20,
            tab_gap: 0,
            top_btn_w: 72,
            top_btn_h: 22,
            top_btn_x: 12,
            top_btn_gap: 0,
            top_btn_y: 5,
            power_w: 16,
        }
    }

    /// 按 1-based mix 索引选度量；仅 `2` 用 `sidec02` 包，其余（含未测的 3+）用 `sidec01` 包。
    pub const fn for_mix_index(index: u32) -> Self {
        if index == 2 { Self::sidec02() } else { Self::sidec01() }
    }

    /// 从嵌套包名解析 `sidecNN` 索引后选度量；解析失败时用 `sidec01` 包。
    pub fn for_mix(mix: &str) -> Self {
        let lower = mix.to_ascii_lowercase();
        let idx = lower
            .strip_prefix("sidec")
            .and_then(|rest| {
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse::<u32>().ok()
            })
            .unwrap_or(1);
        Self::for_mix_index(idx)
    }
}

/// 世界层可视矩形：左起至侧栏左缘，上起至命令条顶边。
pub fn battle_hud_world_viewport(snap: &LayoutSnapshot) -> RectPx {
    let sidebar = rect_px_from_snapshot(snap, "sidebar");
    let command_bar = rect_px_from_snapshot(snap, "command_bar");
    RectPx::new(0, 0, sidebar.x.max(0), command_bar.y.max(0))
}

/// 对局 HUD 各槽位设计矩形（内部算树用）。
#[derive(Debug, Clone, Copy)]
pub struct BattleHudRects {
    pub sidebar: Rect,
    pub credits: Rect,
    pub top: Rect,
    pub radar: Rect,
    pub side1: Rect,
    pub cameo_band: Rect,
    pub side3: Rect,
    pub addon: Rect,
    pub repair: Rect,
    pub sell: Rect,
    pub tabs: [Rect; 4],
    /// 右栏底脚条带（仅侧栏内，不是命令条）。
    pub bottom_strip: Rect,
    /// 战术区底边命令条（侧栏左缘以左，高 `COMMAND_BAR_H`）。
    pub command_bar: Rect,
    pub lendcap: Rect,
    pub rendcap: Rect,
    pub cmd_buttons: [Rect; COMMAND_BAR_BUTTON_COUNT],
    pub opt_btn: Rect,
    pub diplo_btn: Rect,
}

pub fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

pub fn compute_battle_hud_rects(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> BattleHudRects {
    let w = viewport_w.max(1) as i32;
    let h = viewport_h.max(1) as i32;
    let panel_w = (RightPanelChrome::shell_defaults().panel_w as i32).min(w).max(1);
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
    let repair_h = metrics.repair_sell_h.min(side1_h.saturating_sub(4)).max(1);
    let sell_x = (panel_x + metrics.sell_x).min(panel_x + panel_w - repair_w);
    let repair_sell_y = side1_y + metrics.repair_y.min(side1_h.saturating_sub(repair_h));

    let tab_w = {
        let row = (panel_w - metrics.tab_x).max(1);
        // 四页签必须排进侧栏行宽，避免槽位互叠或画出栏外。
        metrics.tab_w.min(row / SIDEBAR_TAB_COUNT as i32).max(1)
    };
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
    let top_btn_w = metrics.top_btn_w.min((panel_w - metrics.top_btn_x).max(1)).max(1);
    let top_btn_y = credits_h + metrics.top_btn_y.min(top_h.saturating_sub(top_btn_h));
    let diplo_x = panel_x + metrics.top_btn_x.min(panel_w.saturating_sub(top_btn_w));
    let opt_x = (diplo_x + top_btn_w + metrics.top_btn_gap).min(panel_x + panel_w - top_btn_w);

    let command_bar = rect_i(0, command_bar_y, command_bar_w, command_bar_h);
    let lend_w = COMMAND_LENDCAP_W.min(command_bar_w).max(1);
    let rend_w = COMMAND_RENDCAP_W.min(command_bar_w.saturating_sub(lend_w)).max(1);
    let buttons_left = lend_w;
    let buttons_right = (command_bar_w - rend_w).max(buttons_left);
    let btn_w = COMMAND_BUTTON_W.max(1);
    let mut cmd_buttons = [rect_i(0, 0, 1, 1); COMMAND_BAR_BUTTON_COUNT];
    for (i, slot) in cmd_buttons.iter_mut().enumerate() {
        let x = buttons_left + (i as i32) * btn_w;
        if x + btn_w > buttons_right {
            // 槽位仍占位（零宽），保持稳定 id。
            *slot = rect_i(x.min(buttons_right), command_bar_y, 0, command_bar_h);
        }
        else {
            *slot = rect_i(x, command_bar_y, btn_w, command_bar_h);
        }
    }

    BattleHudRects {
        sidebar: rect_i(panel_x, 0, panel_w, h),
        credits: rect_i(panel_x, 0, panel_w, credits_h),
        top: rect_i(panel_x, credits_h, panel_w, top_h),
        radar: rect_i(panel_x, radar_y, panel_w, radar_h),
        side1: rect_i(panel_x, side1_y, panel_w, side1_h),
        cameo_band: rect_i(panel_x, cameo_y, panel_w, cameo_h),
        side3: rect_i(panel_x, side3_y, panel_w, side3_h),
        addon: rect_i(panel_x, addon_y, panel_w, addon_h),
        repair: rect_i(panel_x + metrics.repair_x.min(panel_w.saturating_sub(repair_w)), repair_sell_y, repair_w, repair_h),
        sell: rect_i(sell_x, repair_sell_y, repair_w, repair_h),
        tabs,
        // 仅右栏底脚，供 chrome / 文案锚点。
        bottom_strip: rect_i(panel_x, side3_y, panel_w, (h - side3_y).max(1)),
        // 战术区底边命令条：左端至侧栏左缘。
        command_bar,
        lendcap: rect_i(0, command_bar_y, lend_w, command_bar_h),
        rendcap: rect_i((command_bar_w - rend_w).max(0), command_bar_y, rend_w, command_bar_h),
        cmd_buttons,
        diplo_btn: rect_i(diplo_x, top_btn_y, top_btn_w, top_btn_h),
        opt_btn: rect_i(opt_x, top_btn_y, top_btn_w, top_btn_h),
    }
}

pub fn battle_hud_tree_from_rects(viewport_w: u32, viewport_h: u32, r: BattleHudRects) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let mut children = vec![
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
        fixed_rect_leaf("lendcap", r.lendcap),
        fixed_rect_leaf("rendcap", r.rendcap),
        fixed_rect_leaf("opt_btn", r.opt_btn),
        fixed_rect_leaf("diplo_btn", r.diplo_btn),
    ];
    for (id, cell) in COMMAND_BAR_BUTTON_IDS.iter().zip(r.cmd_buttons.iter()) {
        children.push(fixed_rect_leaf(*id, *cell));
    }
    root_with_fixed_children("battle_hud", Size2 { width: w, height: h }, children)
}

/// 对局 HUD 布局树：右栏 chrome 槽位 + 战术区底边命令条（默认 `sidec01` 度量）。
pub fn battle_hud_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    battle_hud_layout_tree_with_metrics(viewport_w, viewport_h, BattleHudChromeMetrics::sidec01())
}

/// 对局 HUD 布局树：按 chrome 度量计算槽位。
pub fn battle_hud_layout_tree_with_metrics(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> LayoutNode {
    let r = compute_battle_hud_rects(viewport_w, viewport_h, metrics);
    battle_hud_tree_from_rects(viewport_w, viewport_h, r)
}

/// 求解对局 HUD snapshot（默认 `sidec01` 度量）。
pub fn solve_battle_hud(viewport_w: u32, viewport_h: u32) -> crate::LayoutSnapshot {
    solve_battle_hud_with_metrics(viewport_w, viewport_h, BattleHudChromeMetrics::sidec01())
}

/// 求解对局 HUD snapshot（指定阵营度量）。
pub fn solve_battle_hud_with_metrics(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> crate::LayoutSnapshot {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    crate::LayoutEngine.solve(
        crate::Viewport { size: Size2 { width: w, height: h }, ..crate::Viewport::default() },
        &battle_hud_layout_tree_with_metrics(viewport_w, viewport_h, metrics),
    )
}

/// 建造栏 cameo 画布宽（像素）。
pub const CAMEO_CELL_W: i32 = 60;
/// 建造栏 cameo 画布高（像素）。
pub const CAMEO_CELL_H: i32 = 48;
/// 建造栏行距（含缝，像素）。
pub const CAMEO_ROW_STRIDE: i32 = 50;
/// 建造栏列数。
pub const CAMEO_COLS: i32 = 2;
/// 分类页签数量（建筑 / 步兵 / 载具 / 飞行器）。
pub const SIDEBAR_TAB_COUNT: usize = 4;

/// 电表右侧可摆 cameo 的内容区（去掉左缘电表条）。
pub fn cameo_content_rect(cameo_band: RectPx, power_meter_w: i32) -> RectPx {
    let left = power_meter_w.max(0).min(cameo_band.w.saturating_sub(1));
    RectPx::new(cameo_band.x + left, cameo_band.y, (cameo_band.w - left).max(1), cameo_band.h.max(1))
}

/// 当前可视 cameo 槽位数（行数 × 2）。
pub fn cameo_visible_slot_count(cameo_band_h: i32) -> usize {
    let rows = ((cameo_band_h - 1).max(0) / CAMEO_ROW_STRIDE) as usize;
    rows.saturating_mul(CAMEO_COLS as usize)
}

/// 可视槽 `slot`（先行后列）的屏幕矩形；越界返回 `None`。
pub fn cameo_slot_rect(cameo_band: RectPx, power_meter_w: i32, slot: usize) -> Option<RectPx> {
    let visible = cameo_visible_slot_count(cameo_band.h);
    if slot >= visible {
        return None;
    }
    let content = cameo_content_rect(cameo_band, power_meter_w);
    let col = (slot as i32) % CAMEO_COLS;
    let row = (slot as i32) / CAMEO_COLS;
    let x = content.x + col * CAMEO_CELL_W;
    let y = content.y + 1 + row * CAMEO_ROW_STRIDE;
    if x + CAMEO_CELL_W > content.x + content.w {
        return None;
    }
    if y + CAMEO_CELL_H > content.y + content.h {
        return None;
    }
    Some(RectPx::new(x, y, CAMEO_CELL_W, CAMEO_CELL_H))
}

/// 命中 cameo 可视槽下标（相对当前滚动起点为 0）。
pub fn hit_cameo_slot(cameo_band: RectPx, power_meter_w: i32, x: i32, y: i32) -> Option<usize> {
    let visible = cameo_visible_slot_count(cameo_band.h);
    for slot in 0..visible {
        if cameo_slot_rect(cameo_band, power_meter_w, slot).is_some_and(|r| r.contains(x, y)) {
            return Some(slot);
        }
    }
    None
}
