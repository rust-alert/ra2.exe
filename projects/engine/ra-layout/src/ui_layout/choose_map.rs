//! 选图页布局。

use super::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseMapLayout {
    /// 共用壳层 chrome（背景 / 右栏）。选图页不画底条装饰。
    pub shell: MainMenuLayout,
    /// 右栏标题（`0x694` / `GUI:ChooseMap`）。
    pub title: RectPx,
    /// 右栏小地图预览（`0x468`）。
    pub map_preview: RectPx,
    /// 右栏地图名底板（`sdmpbtn`）。
    pub map_name_plate: RectPx,
    /// 「选择交战」说明（`GUI:SelectEngagement`）。
    pub label_engagement: RectPx,
    /// 「游戏类型」列标题（`GUI:GameType`）。
    pub label_game_type: RectPx,
    /// 「游戏地图」列标题（`GUI:GameMap`）。
    pub label_game_map: RectPx,
    /// 游戏类型列表（`0x6EB`）。
    pub game_type_list: RectPx,
    /// 地图列表（`0x553`）。
    pub map_list: RectPx,
    /// 底栏状态提示（`0x695`）。
    pub status_help: RectPx,
}

/// 选图页布局（800×600 内容坐标；DLU→px 用 MS Sans Serif 8pt）。
pub fn choose_map_layout(viewport_w: u32, viewport_h: u32) -> ChooseMapLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    shell.lower_strip = RectPx::new(0, 0, 0, 0);
    let use_map = skirmish_snap_button(dlu_rect(318, 122, 108, 23), shell.panel_tile.y);
    let create_random = skirmish_snap_button(dlu_rect(318, 149, 108, 23), shell.panel_tile.y);
    let cancel = button_cell(shell.panel_top.x, shell.panel_bottom.y - BUTTON_CELL_H);
    shell.buttons = [use_map, create_random, cancel, RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0)];
    ChooseMapLayout {
        shell,
        title: skirmish_right_anchor(dlu_rect(318, 1, 108, 10)),
        map_preview: skirmish_right_anchor(dlu_rect(324, 23, 96, 69)),
        map_name_plate: sdmpbtn_rect(),
        label_engagement: dlu_rect(23, 20, 257, 12),
        label_game_type: dlu_rect(20, 60, 130, 10),
        label_game_map: dlu_rect(168, 60, 130, 10),
        game_type_list: dlu_rect(20, 78, 130, 160),
        map_list: dlu_rect(168, 78, 130, 160),
        status_help: dlu_rect(10, 282, 303, 12),
    }
}
