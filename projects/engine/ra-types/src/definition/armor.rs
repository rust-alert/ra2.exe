//! 护甲种类（与弹头 `Verses` 11 项顺序对齐）。

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

/// 护甲名在 `Verses` 列表中的固定顺序（11 项）。
pub const ARMOR_ORDER: [&str; 11] =
    ["none", "flak", "plate", "light", "medium", "heavy", "wood", "steel", "concrete", "special_1", "special_2"];

/// 护甲种类；装载期由 `Armor=` **一次**解码，执行侧只认此枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ArmorKind {
    /// `none` / 未知。
    #[default]
    None = 0,
    /// `flak`
    Flak = 1,
    /// `plate`
    Plate = 2,
    /// `light`
    Light = 3,
    /// `medium`
    Medium = 4,
    /// `heavy`
    Heavy = 5,
    /// `wood`
    Wood = 6,
    /// `steel`
    Steel = 7,
    /// `concrete`
    Concrete = 8,
    /// `special_1`
    Special1 = 9,
    /// `special_2`
    Special2 = 10,
}

impl ArmorKind {
    /// 解析 INI `Armor=` 文本；未知回落 [`Self::None`]。
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "flak" => Self::Flak,
            "plate" => Self::Plate,
            "light" => Self::Light,
            "medium" => Self::Medium,
            "heavy" => Self::Heavy,
            "wood" => Self::Wood,
            "steel" => Self::Steel,
            "concrete" => Self::Concrete,
            "special_1" => Self::Special1,
            "special_2" => Self::Special2,
            _ => Self::None,
        }
    }

    /// 对应 [`ARMOR_ORDER`] / `Verses` 下标。
    pub const fn index(self) -> usize {
        self as u8 as usize
    }

    /// 稳定字面量（诊断 / 快照）。
    pub const fn as_str(self) -> &'static str {
        ARMOR_ORDER[self as u8 as usize]
    }
}

impl<'de> Deserialize<'de> for ArmorKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArmorVisitor;

        impl<'de> Visitor<'de> for ArmorVisitor {
            type Value = ArmorKind;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Armor= kind name")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ArmorKind::parse(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ArmorKind::parse(&v))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ArmorKind::None)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ArmorKind::None)
            }
        }

        deserializer.deserialize_any(ArmorVisitor)
    }
}

/// 护甲名 → `Verses` 下标；未知护甲按 `none`（0）。
pub fn armor_index(armor: &str) -> usize {
    ArmorKind::parse(armor).index()
}
