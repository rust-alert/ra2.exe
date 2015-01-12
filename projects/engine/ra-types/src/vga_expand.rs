//! VGA 6-bit 调色板分量扩到 8-bit 的两种常见公式。

use serde::{Deserialize, Serialize};

/// VGA（`0..=63`）→ 8-bit 扩色模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VgaExpandMode {
    /// `(v & 63) * 255 / 63`，满幅到 255（默认；常见 MIX 工具预览）。
    #[default]
    #[serde(alias = "full_range", alias = "scale255")]
    Full,
    /// `v << 2`，最高 252（常见引擎路径）。
    #[serde(alias = "shift", alias = "shift_left2", alias = "engine")]
    Shift2,
}

impl VgaExpandMode {
    /// 配置 / 日志用稳定字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Shift2 => "shift2",
        }
    }

    /// 解析配置字符串；未知值返回 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "full" | "full_range" | "scale255" => Some(Self::Full),
            "shift2" | "shift" | "shift_left2" | "engine" => Some(Self::Shift2),
            _ => None,
        }
    }

    /// 将单个 6-bit 分量扩到 8-bit。
    #[inline]
    pub const fn expand(self, v: u8) -> u8 {
        let v = v & 63;
        match self {
            Self::Full => ((v as u16 * 255) / 63) as u8,
            Self::Shift2 => v << 2,
        }
    }
}
