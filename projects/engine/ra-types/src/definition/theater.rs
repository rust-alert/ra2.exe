//! 地图剧院（地形 MIX / 调色板 / TMP 扩展名的运行契约键）。

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

use crate::{RaError, RaResult};

/// 地图剧院（决定 MIX / 调色板 / TMP 扩展名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Theater {
    /// 温带。
    #[default]
    Temperate,
    /// 雪地。
    Snow,
    /// 城市。
    Urban,
    /// 月球。
    Lunar,
    /// 沙漠。
    Desert,
}

impl Theater {
    /// 小写剧院名（资源路径用）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Temperate => "temperate",
            Self::Snow => "snow",
            Self::Urban => "urban",
            Self::Lunar => "lunar",
            Self::Desert => "desert",
        }
    }

    /// 解析地图 INI 中的 `Theater=` 值。
    pub fn parse(s: &str) -> RaResult<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "TEMPERATE" | "TEM" => Ok(Self::Temperate),
            "SNOW" | "SNO" => Ok(Self::Snow),
            "URBAN" | "URB" => Ok(Self::Urban),
            "LUNAR" | "LUN" => Ok(Self::Lunar),
            "DESERT" | "DES" => Ok(Self::Desert),
            other => Err(RaError::Parse(format!("未知剧院 `{other}`"))),
        }
    }
}

impl<'de> Deserialize<'de> for Theater {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TheaterVisitor;

        impl<'de> Visitor<'de> for TheaterVisitor {
            type Value = Theater;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("地图 Theater= 名称（如 TEMPERATE / SNOW）")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Theater::parse(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(TheaterVisitor)
    }
}
