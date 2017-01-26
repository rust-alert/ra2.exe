//! 右栏平铺格吸附。

use crate::{RIGHT_PANEL_BOTTOM_H, RIGHT_PANEL_TILE_H, RIGHT_PANEL_TOP_H, RIGHT_PANEL_W, SHELL_BASE_H, SHELL_BASE_W, geometry::Rect};

/// 右栏 chrome 度量（设计像素）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RightPanelChrome {
    /// 壳层设计宽。
    pub shell_w: f32,
    /// 壳层设计高。
    pub shell_h: f32,
    /// 右栏宽。
    pub panel_w: f32,
    /// 顶盖高。
    pub panel_top_h: f32,
    /// 底盖高（`sdbtm` 画布，含版本号突出台）。
    pub panel_bottom_h: f32,
    /// 平铺条高。
    pub tile_h: f32,
    /// 按钮格宽。
    pub button_w: f32,
    /// 按钮格高。
    pub button_h: f32,
}

impl RightPanelChrome {
    /// 与壳层常量一致的默认度量。
    pub fn shell_defaults() -> Self {
        Self {
            shell_w: SHELL_BASE_W as f32,
            shell_h: SHELL_BASE_H as f32,
            panel_w: RIGHT_PANEL_W as f32,
            panel_top_h: RIGHT_PANEL_TOP_H as f32,
            panel_bottom_h: RIGHT_PANEL_BOTTOM_H as f32,
            tile_h: RIGHT_PANEL_TILE_H as f32,
            button_w: 156.0,
            button_h: 42.0,
        }
    }

    /// 右栏左缘 X。
    pub fn panel_x(self) -> f32 {
        self.shell_w - self.panel_w
    }

    /// 平铺区顶边 Y。
    pub fn tile_y(self) -> f32 {
        self.panel_top_h
    }

    /// 按钮格左缘（窄列贴右）。
    pub fn button_x(self) -> f32 {
        self.panel_x() + (self.panel_w - self.button_w)
    }

    /// 平铺条数：顶盖与底盖之间按 `tile_h` 整除（默认 8，上限 9）。
    pub fn tile_count(self) -> i32 {
        let remaining = (self.shell_h - self.panel_top_h - self.panel_bottom_h).max(0.0);
        (remaining / self.tile_h).floor().clamp(0.0, 9.0) as i32
    }

    /// 底盖顶边 Y（平铺区结束处，对齐 `sdbtm`）。
    pub fn panel_bottom_y(self) -> f32 {
        self.tile_y() + self.tile_count() as f32 * self.tile_h
    }
}

/// 将源矩形按 tile 格吸附到右栏按钮列（偏置截断）。
pub fn tile_snap_button(source: Rect, chrome: RightPanelChrome) -> Rect {
    let tile_h = chrome.tile_h.max(1.0);
    let tile_y = chrome.tile_y();
    let tile_index = ((source.y - tile_y + tile_h * 0.5) / tile_h).floor().max(0.0);
    Rect::from_xywh(chrome.button_x(), tile_y + tile_index * tile_h, chrome.button_w, chrome.button_h)
}

/// 贴底盖上沿的一行按钮格（叠在 `sdbtm` 顶部，下方留出版本号突出台）。
pub fn bottom_cover_button(chrome: RightPanelChrome) -> Rect {
    Rect::from_xywh(chrome.button_x(), chrome.panel_bottom_y(), chrome.button_w, chrome.button_h)
}
