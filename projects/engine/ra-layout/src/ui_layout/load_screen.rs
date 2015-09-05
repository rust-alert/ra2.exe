//! 装载页布局。

use super::*;
use crate::{solve_load_screen, LOAD_SCREEN_BUTTON_IDS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadScreenLayout {
    /// 合成画布。
    pub canvas: RectPx,
    /// 特殊单位标题槽。
    pub special: RectPx,
    /// 简报正文槽。
    pub brief: RectPx,
    /// 右下国家名槽。
    pub name: RectPx,
    /// 装载状态文案槽。
    pub status: RectPx,
    /// 进度条放置区（实际宽度由 SHP 裁剪）。
    pub progress: RectPx,
    /// 阵营小旗。
    pub player_flag: RectPx,
    /// 玩家名。
    pub player_name: RectPx,
    /// 失败时：重试 / 取消。
    pub buttons: [RectPx; 2],
}

/// 装载页布局（投影自 `load_screen_layout_tree`）。
pub fn load_screen_layout(_viewport_w: u32, _viewport_h: u32) -> LoadScreenLayout {
    let snap = solve_load_screen();
    LoadScreenLayout {
        canvas: RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H),
        special: rect_px_from_snapshot(&snap, "special"),
        brief: rect_px_from_snapshot(&snap, "brief"),
        name: rect_px_from_snapshot(&snap, "name"),
        status: rect_px_from_snapshot(&snap, "status"),
        progress: rect_px_from_snapshot(&snap, "progress"),
        player_flag: rect_px_from_snapshot(&snap, "player_flag"),
        player_name: rect_px_from_snapshot(&snap, "player_name"),
        buttons: [
            rect_px_from_snapshot(&snap, LOAD_SCREEN_BUTTON_IDS[0]),
            rect_px_from_snapshot(&snap, LOAD_SCREEN_BUTTON_IDS[1]),
        ],
    }
}
