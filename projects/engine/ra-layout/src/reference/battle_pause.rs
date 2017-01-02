//! 对局暂停菜单专属 layout（几何权威为 `solve_battle_pause`）。
//!
//! **不是**主菜单 [`super::shell_chrome::solve_shell_page`]。左区贴阵营 `bkgd*`（`uibkgd.pal`），
//! 右缘 `SIDEBTTN` 六钮；**右侧金属壳与战术底边命令条空轨**由暂停态 HUD chrome 垫底
//!（雷达关图、`side*`/`addon`、无 cameo），本 layout 不画也不用纯色轨冒充。
//! 禁止自制黄框卡片。

use crate::{
    BATTLE_PAUSE_MENU_BUTTON_IDS,
    geometry::Rect,
    reference::{
        DluRect, MS_SANS_SERIF_8PT,
        battle_hud::COMMAND_BAR_H,
    },
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

/// 暂停背景板矩形：战术区（侧栏以左、命令条以上），勿盖住底边命令条空轨。
pub fn battle_pause_background_rect(screen_w: f32, screen_h: f32) -> Rect {
    let w = screen_w.max(1.0);
    let h = screen_h.max(1.0);
    let bw = (w - BATTLE_PAUSE_RAIL_W).max(1.0);
    let bh = (h - COMMAND_BAR_H as f32).max(1.0);
    Rect::from_xywh(0.0, 0.0, bw, bh)
}

/// 右缘按钮轨矩形（透明占位，供命中/诊断；合成时不填色，露出 HUD 侧栏壳）。
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

/// 暂停菜单布局树：战术区 dim/背景板 + 右轨占位 + 右缘 `SIDEBTTN` 六钮。
pub fn battle_pause_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let world = battle_pause_background_rect(w, h);
    let mut children = vec![
        fixed_rect_leaf("dim", world),
        fixed_rect_leaf("background", world),
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
