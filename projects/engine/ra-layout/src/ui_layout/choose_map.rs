//! 选图页布局。

use super::*;
use crate::solve_choose_map;

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

/// 选图页布局（800×600 内容坐标；几何权威来自 `0x6B` snapshot）。
pub fn choose_map_layout(_viewport_w: u32, _viewport_h: u32) -> ChooseMapLayout {
    let mut shell = shell_chrome_base_layout();
    shell.lower_strip = RectPx::new(0, 0, 0, 0);
    let snap = solve_choose_map();
    shell.buttons = [
        rect_px_from_snapshot(&snap, "use_map"),
        rect_px_from_snapshot(&snap, "create_random"),
        rect_px_from_snapshot(&snap, "cancel"),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    ChooseMapLayout {
        shell,
        title: rect_px_from_snapshot(&snap, "title"),
        map_preview: rect_px_from_snapshot(&snap, "map_preview"),
        map_name_plate: rect_px_from_snapshot(&snap, "map_name_plate"),
        label_engagement: rect_px_from_snapshot(&snap, "label_engagement"),
        label_game_type: rect_px_from_snapshot(&snap, "label_game_type"),
        label_game_map: rect_px_from_snapshot(&snap, "label_game_map"),
        game_type_list: rect_px_from_snapshot(&snap, "game_type_list"),
        map_list: rect_px_from_snapshot(&snap, "map_list"),
        status_help: rect_px_from_snapshot(&snap, "status_help"),
    }
}
