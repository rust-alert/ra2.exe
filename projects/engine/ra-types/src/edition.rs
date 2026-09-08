//! 原版 RA2、尤里的复仇、心灵终结 3：一等公民版本。

use crate::error::{RaError, RaResult};

/// 加载哪一套规则与资源布局。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameEdition {
    /// 原版红色警戒 2（`game.exe` / `rules.ini`）。
    Ra2,
    /// 尤里的复仇（`gamemd.exe` / `rulesmd.ini`）。
    Yr,
    /// 心灵终结 3（Mental Omega 3；基于 YR 布局并带 `expandmo*` 等）。
    Mo3,
}

impl GameEdition {
    /// 稳定短名（配置 / 日志用）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ra2 => "ra2",
            Self::Yr => "yr",
            Self::Mo3 => "mo3",
        }
    }

    /// 解析配置字符串；未知取值返回 `UnknownEdition`。
    pub fn parse(s: &str) -> RaResult<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ra2" | "vanilla" | "original" => Ok(Self::Ra2),
            "yr" | "yuri" | "yuris" | "md" => Ok(Self::Yr),
            "mo3" | "mo" | "mentalomega" | "mental-omega" | "mental_omega" => Ok(Self::Mo3),
            other => Err(RaError::UnknownEdition(other.to_string())),
        }
    }
}
