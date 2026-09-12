//! 地图键值：`y * 1000 + x` 十进制打包格坐标。

use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

/// 解包后的格子坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedCellCoords {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
}

impl PackedCellCoords {
    /// 由打包整数构造。
    pub fn from_packed(pos: u32) -> Self {
        let (x, y) = unpack_packed_cell(pos);
        Self { x, y }
    }

    /// 解析十进制打包格文本；非法数字返回 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        let pos: u32 = raw.trim().parse().ok()?;
        Some(Self::from_packed(pos))
    }
}

impl<'de> Deserialize<'de> for PackedCellCoords {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PackedCellVisitor;

        impl<'de> Visitor<'de> for PackedCellVisitor {
            type Value = PackedCellCoords;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("packed cell y*1000+x")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                if v > u64::from(u32::MAX) {
                    return Err(E::custom("packed cell out of u32 range"));
                }
                Ok(PackedCellCoords::from_packed(v as u32))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                if v < 0 {
                    return Err(E::custom("packed cell must be non-negative"));
                }
                self.visit_u64(v as u64)
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                PackedCellCoords::parse(v).ok_or_else(|| E::custom("invalid packed cell"))
            }
        }

        deserializer.deserialize_any(PackedCellVisitor)
    }
}

/// 将打包整数拆成 `(x, y)`。
pub fn unpack_packed_cell(pos: u32) -> (u16, u16) {
    let y = (pos / 1000) as u16;
    let x = (pos % 1000) as u16;
    (x, y)
}

/// 解析十进制打包格文本；非法数字返回 `None`。
pub fn parse_packed_cell(raw: &str) -> Option<(u16, u16)> {
    PackedCellCoords::parse(raw).map(|c| (c.x, c.y))
}
