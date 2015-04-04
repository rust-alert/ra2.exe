//! 战役页布局。

use super::*;
use crate::{
    campaign_content_layout_tree, LayoutEngine, LayoutSnapshot, Rect, RightPanelChrome, Viewport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignLayout {
    /// 共用壳层 chrome（背景 / 右栏 / 底条）。
    pub shell: MainMenuLayout,
    /// 右栏标题（与主菜单壳层 `title` 同格；CSF `GUI:CampaignMenu`）。
    pub title: RectPx,
    /// 盟军侧图（`fsalg.shp` 570×135 @ (30,26)）。
    pub allied: RectPx,
    /// 新兵训练营侧图（`fsbclg.shp` 468×108 @ (82,186)）。
    pub tutorial: RectPx,
    /// 苏军侧图（`fsslg.shp` 444×149 @ (98,298)）。
    pub soviet: RectPx,
    /// 难度标签（`GUI:Difficulty`）。
    pub difficulty_label: RectPx,
    /// 难度当前值。
    pub difficulty_value: RectPx,
    /// 难度滑条（`0x50F`）。
    pub difficulty_track: RectPx,
    /// 底栏状态提示（与主菜单壳层 `tooltip` 同格）。
    pub status_help: RectPx,
}

fn rect_px(snap: &LayoutSnapshot, id: &str) -> RectPx {
    let Rect {
        x,
        y,
        width,
        height,
    } = snap.get(id).map(|e| e.layout.rect).unwrap_or_default();
    RectPx::new(x as i32, y as i32, width as i32, height as i32)
}

/// 战役页布局（800×600；交互几何来自 `campaign_content_layout_tree`）。
pub fn campaign_layout(viewport_w: u32, viewport_h: u32) -> CampaignLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: crate::shell_design_size(chrome),
            ..Viewport::default()
        },
        &campaign_content_layout_tree(chrome),
    );
    let back = rect_px(&snap, CAMPAIGN_BUTTON_IDS[0]);
    shell.buttons = [
        back,
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    // 难度标签/数值仍为过渡期固定设计坐标（尚未进 content tree）。
    let diff_label_y = 454;
    let diff_x = 191;
    let diff_track_w = 247;
    let diff_label_w = 100;
    CampaignLayout {
        shell,
        title: shell.title,
        allied: rect_px(&snap, CAMPAIGN_SIDE_IDS[0]),
        tutorial: rect_px(&snap, CAMPAIGN_SIDE_IDS[1]),
        soviet: rect_px(&snap, CAMPAIGN_SIDE_IDS[2]),
        difficulty_label: RectPx::new(diff_x, diff_label_y, diff_label_w, 20),
        difficulty_value: RectPx::new(diff_x + diff_track_w - diff_label_w, diff_label_y, diff_label_w, 20),
        difficulty_track: rect_px(&snap, "difficulty"),
        status_help: shell.tooltip,
    }
}
