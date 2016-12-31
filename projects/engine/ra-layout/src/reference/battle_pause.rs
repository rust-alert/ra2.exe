//! 对局暂停菜单专属 layout（几何权威为 `solve_battle_pause`）。
//!
//! **不是** [`super::battle_hud`]，也**不是**主菜单 [`super::shell_chrome::solve_shell_page`]。
//! 局内暂停主钮与局内选项 `0xBBB` 同走 owner-draw **type 2**：`SIDEBTTN.SHP` +
//! `SIDEBAR.PAL`，右缘 inset 147、画布 125×25；竖向取资源 DLU + 相对 800×600 居中。
//! 背景板为阵营 `bkgdsm` / `bkgdmd` / `bkgdlg`（按视口选档，调色板 `uibkgd.pal`），铺满左区
//! （屏宽减去右轨 [`BATTLE_PAUSE_RAIL_W`]）与全高，禁止用 HUD 的 `side1`/`addon`/`radar` 冒充右轨。
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
/// 右缘按钮轨宽（`bkgdmd` 632 + 轨 168 = 800；各分辨率档同为 168）。
pub const BATTLE_PAUSE_RAIL_W: f32 = 168.0;
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

/// `bkgdsm.shp` 画布（约 640×480 档）。
pub const BATTLE_PAUSE_BKGD_SM: (f32, f32) = (472.0, 448.0);
/// `bkgdmd.shp` 画布（约 800×600 档）。
pub const BATTLE_PAUSE_BKGD_MD: (f32, f32) = (632.0, 568.0);
/// `bkgdlg.shp` 画布（约 1024×768 档）。
pub const BATTLE_PAUSE_BKGD_LG: (f32, f32) = (856.0, 736.0);

/// 游戏控制（上列首格）。
const GAME_CONTROLS_DLU: DluRect = DluRect::new(425, 122, 108, 23);
/// 载入（+27 DLU）。
const LOAD_DLU: DluRect = DluRect::new(425, 149, 108, 23);
/// 保存。
const SAVE_DLU: DluRect = DluRect::new(425, 176, 108, 23);
/// 删除。
const DELETE_DLU: DluRect = DluRect::new(425, 203, 108, 23);
/// 放弃。
const ABORT_DLU: DluRect = DluRect::new(425, 230, 108, 23);
/// 回到任务（`0xBBB` Back 同列贴底族）。
const RESUME_DLU: DluRect = DluRect::new(425, 346, 108, 23);

const BUTTON_DLUS: [DluRect; 6] = [GAME_CONTROLS_DLU, LOAD_DLU, SAVE_DLU, DELETE_DLU, ABORT_DLU, RESUME_DLU];

/// 相对 800×600 基准的垂直居中偏移（宽屏 / 高屏）。
pub fn battle_pause_center_offset(screen: f32, base: f32) -> f32 {
    ((screen - base) * 0.5).max(0.0)
}

/// 按视口选暂停背景板**素材**画布尺寸（用于选 `bkgdsm`/`md`/`lg` 帧）。
pub fn battle_pause_background_size(screen_w: f32, screen_h: f32) -> (f32, f32) {
    if screen_w <= 640.0 || screen_h <= 480.0 {
        BATTLE_PAUSE_BKGD_SM
    } else if screen_w <= 800.0 || screen_h <= 600.0 {
        BATTLE_PAUSE_BKGD_MD
    } else {
        BATTLE_PAUSE_BKGD_LG
    }
}

/// 暂停背景板矩形：左区铺满（宽 = 屏宽 − 右轨），高铺满视口，消除上下空隙。
pub fn battle_pause_background_rect(screen_w: f32, screen_h: f32) -> Rect {
    let w = screen_w.max(1.0);
    let h = screen_h.max(1.0);
    let bw = (w - BATTLE_PAUSE_RAIL_W).max(1.0);
    Rect::from_xywh(0.0, 0.0, bw, h)
}

/// 右缘按钮轨矩形（仅暗底占位，**不是** HUD chrome）。
pub fn battle_pause_rail_rect(screen_w: f32, screen_h: f32) -> Rect {
    let w = screen_w.max(1.0);
    let h = screen_h.max(1.0);
    let x = (w - BATTLE_PAUSE_RAIL_W).max(0.0);
    Rect::from_xywh(x, 0.0, BATTLE_PAUSE_RAIL_W.min(w), h)
}

/// 将资源 DLU 换成 owner-draw 钮：右缘钉死，宽高钉 `SIDEBTTN`，Y 取 DLU + 居中偏移。
pub fn battle_sidebttn_rect(screen_w: f32, screen_h: f32, dlu: DluRect) -> Rect {
    let origin = dlu.to_design_px(MS_SANS_SERIF_8PT);
    let dy = battle_pause_center_offset(screen_h, BATTLE_PAUSE_BASE_H);
    let x = (screen_w - BATTLE_PAUSE_BUTTON_RIGHT_INSET).max(0.0);
    let y = (origin.y + dy).max(0.0);
    Rect::from_xywh(x, y, BATTLE_PAUSE_BUTTON_W, BATTLE_PAUSE_BUTTON_H)
}

/// 暂停菜单布局树（窗口 / 设计像素）：全屏 dim + 左背景板 + 右轨 + 右缘 `SIDEBTTN` 六钮。
pub fn battle_pause_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let mut children = vec![
        fixed_rect_leaf("dim", Rect::from_xywh(0.0, 0.0, w, h)),
        fixed_rect_leaf("background", battle_pause_background_rect(w, h)),
        fixed_rect_leaf("rail", battle_pause_rail_rect(w, h)),
    ];
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
