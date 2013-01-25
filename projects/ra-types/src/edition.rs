//! 原版 RA2 与尤里的复仇：一等公民版本。

use crate::error::{RaError, RaResult};

/// 加载哪一套零售规则与资源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameEdition {
    /// 原版红色警戒 2（`game.exe` / `rules.ini`）。
    Ra2,
    /// 尤里的复仇（`gamemd.exe` / `rulesmd.ini`）。
    Yr,
}

impl GameEdition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ra2 => "ra2",
            Self::Yr => "yr",
        }
    }

    pub fn parse(s: &str) -> RaResult<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ra2" | "vanilla" | "original" => Ok(Self::Ra2),
            "yr" | "yuri" | "yuris" | "md" => Ok(Self::Yr),
            other => Err(RaError::UnknownEdition(other.to_string())),
        }
    }
}
