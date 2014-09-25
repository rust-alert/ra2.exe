//! 壳层质感呈现：在 32 位管线上模拟原版 16 位色 DirectDraw 观感。
//!
//! 原版零售客户区多为 RGB565；现代路径解码为 8 位扩展色后直出，会偏亮、偏白、
//! 高光更冲。本配置描述「呈现侧」如何压回原版质感，不是改资源解码。
//!
//! 落盘形态为 `RustAlert.toml` 的 `[present]` 表，由 `toml_edit` + serde 读写。

use serde::{Deserialize, Serialize};

/// 质感呈现模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresentMode {
    /// 关闭模拟，直出 32 位扩展色（便于对照）。
    #[serde(alias = "false", alias = "0", alias = "truecolor", alias = "32bit", alias = "none")]
    Off,
    /// 模拟原版 16 位色呈现（默认产品目标）。
    #[default]
    #[serde(rename = "16bit", alias = "bit16", alias = "16", alias = "on", alias = "true", alias = "1")]
    Bit16,
}

impl PresentMode {
    /// 配置 / 日志用稳定字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Bit16 => "16bit",
        }
    }
}

/// 16 位量化格式（仅 [`PresentMode::Bit16`] 生效）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresentQuantize {
    /// RGB565（原版常见 16 位帧缓冲）。
    #[default]
    #[serde(alias = "565", alias = "r5g6b5")]
    Rgb565,
    /// RGB555。
    #[serde(alias = "555", alias = "r5g5b5")]
    Rgb555,
}

impl PresentQuantize {
    /// 配置 / 日志用稳定字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rgb565 => "rgb565",
            Self::Rgb555 => "rgb555",
        }
    }
}

/// 壳层质感呈现参数集（对应 `RustAlert.toml` 的 `[present]`）。
///
/// 默认对齐同机原版主菜单截图：`16bit` + `rgb565` + `gamma≈1.15` + 有序抖动。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresentFeel {
    /// 呈现模式。
    pub mode: PresentMode,
    /// 量化格式（仅 16 位模式）。
    pub quantize: PresentQuantize,
    /// 显示伽马；`>1` 压暗中高光（同机原版侧栏金属对照约 `1.15`）。
    pub gamma: f32,
    /// 高光收敛 `0..1`：额外压亮部，`0` 表示关闭。
    pub highlight_roll_off: f32,
    /// 量化前有序抖动，找回 16 位颗粒感。
    pub dither: bool,
}

impl Default for PresentFeel {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl PresentFeel {
    /// 产品默认：开启 16 位质感模拟。
    ///
    /// `gamma≈1.15` 对齐侧栏金属；`highlight_roll_off≈0.25` 压 WARNING 屏高光过曝。
    pub const DEFAULT: Self = Self {
        mode: PresentMode::Bit16,
        quantize: PresentQuantize::Rgb565,
        gamma: 1.15,
        highlight_roll_off: 0.25,
        dither: true,
    };

    /// 对照用：关闭模拟。
    pub const OFF: Self = Self {
        mode: PresentMode::Off,
        quantize: PresentQuantize::Rgb565,
        gamma: 1.0,
        highlight_roll_off: 0.0,
        dither: false,
    };

    /// 是否会对像素做呈现变换。
    pub const fn is_active(self) -> bool {
        matches!(self.mode, PresentMode::Bit16)
    }

    /// 夹紧到合法范围（配置加载后调用）。
    pub fn sanitized(self) -> Self {
        Self {
            mode: self.mode,
            quantize: self.quantize,
            gamma: if self.gamma.is_finite() {
                self.gamma.clamp(0.5, 3.0)
            } else {
                Self::DEFAULT.gamma
            },
            highlight_roll_off: if self.highlight_roll_off.is_finite() {
                self.highlight_roll_off.clamp(0.0, 1.0)
            } else {
                0.0
            },
            dither: self.dither,
        }
    }
}
