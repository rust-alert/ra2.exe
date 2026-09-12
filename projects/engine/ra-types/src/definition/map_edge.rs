//! 地图房屋 `Edge=` 进场边。

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

/// 地图房屋进场边（INI `Edge=`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MapEdge {
    /// 未写或未知。
    #[default]
    Unspecified,
    /// 北。
    North,
    /// 东。
    East,
    /// 南。
    South,
    /// 西。
    West,
}

impl MapEdge {
    /// 解析 `Edge=`；空 / 未知 → [`Self::Unspecified`]。
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "north" => Self::North,
            "east" => Self::East,
            "south" => Self::South,
            "west" => Self::West,
            _ => Self::Unspecified,
        }
    }
}

impl<'de> Deserialize<'de> for MapEdge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EdgeVisitor;

        impl<'de> Visitor<'de> for EdgeVisitor {
            type Value = MapEdge;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Edge= North East South West")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(MapEdge::parse(v))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(MapEdge::parse(&v))
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(MapEdge::Unspecified)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(MapEdge::Unspecified)
            }
        }

        deserializer.deserialize_any(EdgeVisitor)
    }
}
