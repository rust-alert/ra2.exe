//! 对局外交子页（暂停层；复用 pause hub + `SIDEBTTN` Back）。
//!
//! 与局内选项 `0xBBB` 同族：一棵树含右 hub 壳槽、全屏 dim、名单行与右缘 Back。

use crate::{
    geometry::Rect,
    reference::{DluRect, MS_SANS_SERIF_8PT, battle_hud::BattleHudChromeMetrics},
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

use super::battle_pause::{
    BATTLE_PAUSE_BASE_H, BATTLE_PAUSE_BASE_W, battle_pause_center_offset, battle_pause_hub_leaves, battle_sidebttn_rect,
};

/// 右栏钮：仅 Back。
pub const BATTLE_DIPLOMACY_BUTTON_IDS: [&str; 1] = ["back"];

/// 名单最大行数（遭遇战席位上限）。
pub const BATTLE_DIPLOMACY_ROW_COUNT: usize = 8;

/// Back（与 `0xBBB` `0x686` 同 DLU）。
const BACK_DLU: DluRect = DluRect::new(425, 346, 108, 23);

fn centered_dlu(screen_w: f32, screen_h: f32, dlu: DluRect) -> Rect {
    let r = dlu.to_design_px(MS_SANS_SERIF_8PT);
    let dx = battle_pause_center_offset(screen_w, BATTLE_PAUSE_BASE_W);
    let dy = battle_pause_center_offset(screen_h, BATTLE_PAUSE_BASE_H);
    Rect::from_xywh(r.x + dx, r.y + dy, r.width, r.height)
}

/// 外交布局树。
pub fn battle_diplomacy_layout_tree(viewport_w: u32, viewport_h: u32) -> LayoutNode {
    let metrics = BattleHudChromeMetrics::sidec01();
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    let mut children = battle_pause_hub_leaves(viewport_w, viewport_h, metrics);
    children.push(fixed_rect_leaf("dim", Rect::from_xywh(0.0, 0.0, w, h)));
    children.push(fixed_rect_leaf("title", centered_dlu(w, h, DluRect::new(40, 40, 280, 14))));
    children.push(fixed_rect_leaf("local_label", centered_dlu(w, h, DluRect::new(40, 64, 280, 12))));
    // 行 y：自 90 起，每行 18 DLU（国家名 + 状态并排）。
    for i in 0..BATTLE_DIPLOMACY_ROW_COUNT {
        let y = 90 + (i as i32) * 18;
        children.push(fixed_rect_leaf(&format!("row_name_{i}"), centered_dlu(w, h, DluRect::new(40, y, 160, 12))));
        children.push(fixed_rect_leaf(&format!("row_status_{i}"), centered_dlu(w, h, DluRect::new(210, y, 100, 12))));
    }
    children.push(fixed_rect_leaf("footer", centered_dlu(w, h, DluRect::new(2, 355, 303, 12))));
    children.push(fixed_rect_leaf(BATTLE_DIPLOMACY_BUTTON_IDS[0], battle_sidebttn_rect(w, h, BACK_DLU)));
    root_with_fixed_children("battle_diplomacy", crate::geometry::Size2 { width: w, height: h }, children)
}

/// 在给定视口求解外交 snapshot。
pub fn solve_battle_diplomacy_at(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    let w = viewport_w.max(1) as f32;
    let h = viewport_h.max(1) as f32;
    LayoutEngine.solve(
        Viewport { size: crate::geometry::Size2 { width: w, height: h }, ..Viewport::default() },
        &battle_diplomacy_layout_tree(viewport_w, viewport_h),
    )
}

/// 设计基准 800×600 求解。
pub fn solve_battle_diplomacy() -> LayoutSnapshot {
    solve_battle_diplomacy_at(BATTLE_PAUSE_BASE_W as u32, BATTLE_PAUSE_BASE_H as u32)
}
