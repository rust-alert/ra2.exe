//! 对局暂停菜单专属 layout（几何权威为 `solve_battle_pause`）。
//!
//! 一棵树同时持有：战术区 `dim`/`background`、右侧 pause hub 槽位（几何来自
//! [`compute_battle_hud_rects`]，id 独立：`list_band` 而非对局 `cameo_band`）、
//! 以及右缘 `SIDEBTTN` 六钮紧列表。合成层只认本 snapshot，禁止再叠一套 HUD 垫底。
//! 禁止自制黄框卡片。

use crate::{
    BATTLE_PAUSE_MENU_BUTTON_IDS,
    geometry::Rect,
    reference::battle_hud::{BattleHudChromeMetrics, compute_battle_hud_rects},
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

/// 设计基准宽。
pub const BATTLE_PAUSE_BASE_W: f32 = 800.0;
/// 设计基准高。
pub const BATTLE_PAUSE_BASE_H: f32 = 600.0;
/// 右缘 hub 宽（与壳层右栏默认同值；以 hub 几何为准）。
pub const BATTLE_PAUSE_RAIL_W: f32 = 168.0;
/// `sidebttn.shp` 画布宽。
pub const BATTLE_PAUSE_BUTTON_W: f32 = 125.0;
/// `sidebttn.shp` 画布高。
pub const BATTLE_PAUSE_BUTTON_H: f32 = 25.0;
/// 钮左缘相对侧栏左缘 inset。
pub const BATTLE_PAUSE_BUTTON_SIDEBAR_INSET: f32 = 21.0;
/// 兼容旧名：`x = screen_w - inset`。
pub const BATTLE_PAUSE_BUTTON_RIGHT_INSET: f32 = 147.0;
/// 同 [`BATTLE_PAUSE_BUTTON_W`]。
pub const BATTLE_PAUSE_SIDEBTTN_W: f32 = BATTLE_PAUSE_BUTTON_W;
/// 同 [`BATTLE_PAUSE_BUTTON_H`]。
pub const BATTLE_PAUSE_SIDEBTTN_H: f32 = BATTLE_PAUSE_BUTTON_H;

/// `bkgdsm.shp` 画布（约 640×480 档）。
pub const BATTLE_PAUSE_BKGD_SM: (f32, f32) = (472.0, 448.0);
/// `bkgdmd.shp` 画布（约 800×600 档）。
pub const BATTLE_PAUSE_BKGD_MD: (f32, f32) = (632.0, 568.0);
/// `bkgdlg.shp` 画布（约 1024×768 档）。
pub const BATTLE_PAUSE_BKGD_LG: (f32, f32) = (856.0, 736.0);

/// 上列可点/灰显钮数（不含 resume）。
const UPPER_BUTTON_COUNT: usize = 5;

/// 相对 800×600 基准的垂直居中偏移（局内选项等仍用）。
pub fn battle_pause_center_offset(screen: f32, base: f32) -> f32 {
    ((screen - base) * 0.5).max(0.0)
}

/// 按视口选暂停背景板素材画布尺寸。
pub fn battle_pause_background_size(screen_w: f32, screen_h: f32) -> (f32, f32) {
    if screen_w <= 640.0 || screen_h <= 480.0 {
        BATTLE_PAUSE_BKGD_SM
    } else if screen_w <= 800.0 || screen_h <= 600.0 {
        BATTLE_PAUSE_BKGD_MD
    } else {
        BATTLE_PAUSE_BKGD_LG
    }
}

/// 战术区矩形（侧栏以左、命令条以上）。
pub fn battle_pause_world_rect(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> Rect {
    let hud = compute_battle_hud_rects(viewport_w, viewport_h, metrics);
    Rect::from_xywh(0.0, 0.0, hud.sidebar.x.max(0.0), hud.command_bar.y.max(0.0))
}

/// 兼容旧 API：默认 `sidec01` 度量下的战术区。
pub fn battle_pause_background_rect(screen_w: f32, screen_h: f32) -> Rect {
    battle_pause_world_rect(screen_w.max(1.0) as u32, screen_h.max(1.0) as u32, BattleHudChromeMetrics::sidec01())
}

/// 兼容旧 API：右栏整列。
pub fn battle_pause_rail_rect(screen_w: f32, screen_h: f32) -> Rect {
    let hud = compute_battle_hud_rects(screen_w.max(1.0) as u32, screen_h.max(1.0) as u32, BattleHudChromeMetrics::sidec01());
    hud.sidebar
}

/// 兼容旧 API：owner-draw 钮（局内选项 `0xBBB` 仍用 DLU 列）。
pub fn battle_sidebttn_rect(screen_w: f32, screen_h: f32, dlu: crate::reference::DluRect) -> Rect {
    use crate::reference::MS_SANS_SERIF_8PT;
    let origin = dlu.to_design_px(MS_SANS_SERIF_8PT);
    let dy = battle_pause_center_offset(screen_h, BATTLE_PAUSE_BASE_H);
    let x = (screen_w - BATTLE_PAUSE_BUTTON_RIGHT_INSET).max(0.0);
    let y = (origin.y + dy).max(0.0);
    Rect::from_xywh(x, y, BATTLE_PAUSE_BUTTON_W, BATTLE_PAUSE_BUTTON_H)
}

/// 暂停六钮：钉在 `list_band` 内，上列五钮贴紧，`resume` 贴带底。
pub fn battle_pause_menu_button_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
    index: usize,
) -> Rect {
    let hud = compute_battle_hud_rects(viewport_w, viewport_h, metrics);
    let band = hud.cameo_band;
    let x = (hud.sidebar.x + BATTLE_PAUSE_BUTTON_SIDEBAR_INSET).max(hud.sidebar.x);
    let btn_h = BATTLE_PAUSE_BUTTON_H;
    let y = if index + 1 == BATTLE_PAUSE_MENU_BUTTON_IDS.len() {
        (band.y + band.height - btn_h).max(band.y)
    } else {
        let i = index.min(UPPER_BUTTON_COUNT.saturating_sub(1)) as f32;
        band.y + i * btn_h
    };
    Rect::from_xywh(x, y, BATTLE_PAUSE_BUTTON_W, btn_h)
}

/// 暂停布局树（默认 `sidec01` 度量）。
pub fn battle_pause_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    battle_pause_layout_tree_with_metrics(viewport_w, viewport_h, BattleHudChromeMetrics::sidec01())
}

/// 暂停布局树：world + pause hub 槽 + 六钮。
pub fn battle_pause_layout_tree_with_metrics(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let hud = compute_battle_hud_rects(viewport_w, viewport_h, metrics);
    let world = battle_pause_world_rect(viewport_w, viewport_h, metrics);
    let mut children = vec![
        fixed_rect_leaf("dim", world),
        fixed_rect_leaf("background", world),
        fixed_rect_leaf("sidebar", hud.sidebar),
        fixed_rect_leaf("credits", hud.credits),
        fixed_rect_leaf("top", hud.top),
        fixed_rect_leaf("radar", hud.radar),
        fixed_rect_leaf("side1", hud.side1),
        fixed_rect_leaf("list_band", hud.cameo_band),
        fixed_rect_leaf("side3", hud.side3),
        fixed_rect_leaf("addon", hud.addon),
        fixed_rect_leaf("command_bar", hud.command_bar),
        fixed_rect_leaf("lendcap", hud.lendcap),
        fixed_rect_leaf("rendcap", hud.rendcap),
        fixed_rect_leaf("rail", hud.sidebar),
    ];
    for (i, id) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().enumerate() {
        children.push(fixed_rect_leaf(*id, battle_pause_menu_button_rect(viewport_w, viewport_h, metrics, i)));
    }
    root_with_fixed_children("battle_pause", crate::geometry::Size2 { width: w, height: h }, children)
}

/// 在给定视口求解（默认 `sidec01`）。
pub fn solve_battle_pause_at(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    solve_battle_pause_with_metrics(viewport_w, viewport_h, BattleHudChromeMetrics::sidec01())
}

/// 按侧栏 chrome 度量求解。
pub fn solve_battle_pause_with_metrics(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> LayoutSnapshot {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    LayoutEngine.solve(
        Viewport { size: crate::geometry::Size2 { width: w, height: h }, ..Viewport::default() },
        &battle_pause_layout_tree_with_metrics(viewport_w, viewport_h, metrics),
    )
}

/// 设计基准 800×600。
pub fn solve_battle_pause() -> LayoutSnapshot {
    solve_battle_pause_at(BATTLE_PAUSE_BASE_W as u32, BATTLE_PAUSE_BASE_H as u32)
}
