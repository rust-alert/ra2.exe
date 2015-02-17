//! 右栏平铺格吸附。

use crate::geometry::Rect;

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
            shell_w: 800.0,
            shell_h: 600.0,
            panel_w: 168.0,
            panel_top_h: 199.0,
            tile_h: 42.0,
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

    /// 底盖顶边 Y（顶盖以下按 tile 整除后的余数区上沿）。
    pub fn panel_bottom_y(self) -> f32 {
        let remaining = (self.shell_h - self.panel_top_h).max(0.0);
        let tile_count = (remaining / self.tile_h).floor().clamp(0.0, 9.0);
        self.tile_y() + tile_count * self.tile_h
    }
}

/// 将源矩形按 tile 格吸附到右栏按钮列（偏置截断）。
pub fn tile_snap_button(source: Rect, chrome: RightPanelChrome) -> Rect {
    let tile_h = chrome.tile_h.max(1.0);
    let tile_y = chrome.tile_y();
    let tile_index = ((source.y - tile_y + tile_h * 0.5) / tile_h).floor().max(0.0);
    Rect::from_xywh(
        chrome.button_x(),
        tile_y + tile_index * tile_h,
        chrome.button_w,
        chrome.button_h,
    )
}

/// 贴底盖上沿的一行按钮格。
pub fn bottom_cover_button(chrome: RightPanelChrome) -> Rect {
    Rect::from_xywh(
        chrome.button_x(),
        chrome.panel_bottom_y() - chrome.button_h,
        chrome.button_w,
        chrome.button_h,
    )
}
