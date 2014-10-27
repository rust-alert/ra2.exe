//! 壳层质感呈现：在 32 位管线上模拟原版 16 位色 DirectDraw 观感。
//!
//! 原版零售客户区多为 RGB565。呈现侧只做 **16 位截断量化 + 满量程线性展开**
//!（可选有序抖动）。亮度观感由资源解码（如 BIK 色域）决定，不用显示伽马拧 UI。
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
/// 默认：`16bit` + `rgb565` 截断往返 + 有序抖动。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresentFeel {
    /// 呈现模式。
    pub mode: PresentMode,
    /// 量化格式（仅 16 位模式）。
    pub quantize: PresentQuantize,
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
    pub const DEFAULT: Self = Self {
        mode: PresentMode::Bit16,
        quantize: PresentQuantize::Rgb565,
        dither: true,
    };

    /// 对照用：关闭模拟。
    pub const OFF: Self = Self {
        mode: PresentMode::Off,
        quantize: PresentQuantize::Rgb565,
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
            dither: self.dither,
        }
    }
}
