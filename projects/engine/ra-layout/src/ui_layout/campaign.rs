//! 战役页布局。

use super::*;


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

/// 战役页布局（800×600 内容坐标）。
pub fn campaign_layout(viewport_w: u32, viewport_h: u32) -> CampaignLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    // 仅「上一页」贴底盖；右栏其余为部队 cameo 格（后续接线，勿再放载入钮）。
    let back = button_cell(shell.panel_top.x, shell.panel_bottom.y - BUTTON_CELL_H);
    shell.buttons =
        [back, RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0)];
    let allied = RectPx::new(CAMPAIGN_ALLIED_ORIGIN.0, CAMPAIGN_ALLIED_ORIGIN.1, CAMPAIGN_ALLIED_SIZE.0, CAMPAIGN_ALLIED_SIZE.1);
    let tutorial = RectPx::new(CAMPAIGN_TUTORIAL_ORIGIN.0, CAMPAIGN_TUTORIAL_ORIGIN.1, CAMPAIGN_TUTORIAL_SIZE.0, CAMPAIGN_TUTORIAL_SIZE.1);
    let soviet = RectPx::new(CAMPAIGN_SOVIET_ORIGIN.0, CAMPAIGN_SOVIET_ORIGIN.1, CAMPAIGN_SOVIET_SIZE.0, CAMPAIGN_SOVIET_SIZE.1);
    // 难度：原版截图映到 800×600 — 标签 y≈454、轨 y≈483、x≈191、轨宽≈247。
    let diff_label_y = 454;
    let diff_x = 191;
    let diff_track_w = 247;
    let diff_label_w = 100;
    CampaignLayout {
        shell,
        title: shell.title,
        allied,
        tutorial,
        soviet,
        difficulty_label: RectPx::new(diff_x, diff_label_y, diff_label_w, 20),
        difficulty_value: RectPx::new(diff_x + diff_track_w - diff_label_w, diff_label_y, diff_label_w, 20),
        difficulty_track: RectPx::new(diff_x, 483, diff_track_w, 13),
        status_help: shell.tooltip,
    }
}
