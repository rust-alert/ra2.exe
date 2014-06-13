//! 离散显示分辨率（游戏客户区模式，非自由拉伸窗口）。

use crate::error::{RaError, RaResult};

/// 产品支持的客户区分辨率档位。
///
/// 原版与现代重写都按**有限档位**选型：壳层布局、菜单影片与 GPU 离屏缓冲
/// 都挂在这些尺寸上，而不是任意宽高。操作系统窗口可以 letterbox 这一档，
/// 但游戏逻辑分辨率仍是枚举值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DisplayMode {
    /// 640×480（窄壳，小尺寸菜单资源）。
    W640H480,
    /// 800×600（设计基准壳）。
    W800H600,
    /// 1024×768（大窗居中壳）。
    #[default]
    W1024H768,
}

impl DisplayMode {
    /// 产品默认档（与当前桌面启动默认一致）。
    pub const DEFAULT: Self = Self::W1024H768;

    /// 全部已支持档位（选项菜单枚举用）。
    pub const ALL: &'static [Self] = &[Self::W640H480, Self::W800H600, Self::W1024H768];

    /// 客户区宽（逻辑像素）。
    pub const fn width(self) -> u32 {
        match self {
            Self::W640H480 => 640,
            Self::W800H600 => 800,
            Self::W1024H768 => 1024,
        }
    }

    /// 客户区高（逻辑像素）。
    pub const fn height(self) -> u32 {
        match self {
            Self::W640H480 => 480,
            Self::W800H600 => 600,
            Self::W1024H768 => 768,
        }
    }

    /// `(宽, 高)`。
    pub const fn size(self) -> (u32, u32) {
        (self.width(), self.height())
    }

    /// 配置 / 日志用稳定字符串（`640x480` 形式）。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::W640H480 => "640x480",
            Self::W800H600 => "800x600",
            Self::W1024H768 => "1024x768",
        }
    }

    /// 解析配置字符串；未知取值返回 [`RaError::UnknownDisplayMode`]。
    pub fn parse(s: &str) -> RaResult<Self> {
        let raw = s.trim().to_ascii_lowercase().replace('×', "x").replace('*', "x").replace(' ', "");
        match raw.as_str() {
            "640x480" | "640" => Ok(Self::W640H480),
            "800x600" | "800" => Ok(Self::W800H600),
            "1024x768" | "1024" => Ok(Self::W1024H768),
            other => Err(RaError::UnknownDisplayMode(other.to_string())),
        }
    }
}
