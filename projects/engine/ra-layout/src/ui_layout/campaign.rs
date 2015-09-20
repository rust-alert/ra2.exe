//! 战役页布局。

use super::*;
use crate::{solve_campaign, RightPanelChrome};

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

/// 战役页布局（800×600；chrome 与交互几何来自同一次 `solve_campaign`）。
pub fn campaign_layout(_viewport_w: u32, _viewport_h: u32) -> CampaignLayout {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = solve_campaign();
    let mut shell = layout_from_shell_page_snap(chrome, &snap);
    let back = rect_px_from_snapshot(&snap, CAMPAIGN_BUTTON_IDS[0]);
    shell.buttons = [
        back,
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    CampaignLayout {
        shell,
        title: shell.title,
        allied: rect_px_from_snapshot(&snap, CAMPAIGN_SIDE_IDS[0]),
        tutorial: rect_px_from_snapshot(&snap, CAMPAIGN_SIDE_IDS[1]),
        soviet: rect_px_from_snapshot(&snap, CAMPAIGN_SIDE_IDS[2]),
        difficulty_label: rect_px_from_snapshot(&snap, "difficulty_label"),
        difficulty_value: rect_px_from_snapshot(&snap, "difficulty_value"),
        difficulty_track: rect_px_from_snapshot(&snap, "difficulty"),
        status_help: shell.tooltip,
    }
}
