//! Techno `Category=`（装载期一次解码）。

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

/// INI `Category=` 玩法分类；装载期一次解码，执行侧只认此枚举。
///
/// 未知非空值归入 [`Self::Other`]（模组扩展位）；缺键 / 空串 → [`Self::Unspecified`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TechnoCategory {
    /// 未写或空串。
    #[default]
    Unspecified,
    /// `Soldier`
    Soldier,
    /// `Dog`（警犬等；AI 常规量产排除）。
    Dog,
    /// `Civilian`
    Civilian,
    /// `Vehicle`
    Vehicle,
    /// `Armored`
    Armored,
    /// `AirPower`
    AirPower,
    /// `Ship`
    Ship,
    /// 其它非空 `Category=` 文本。
    Other,
}

impl TechnoCategory {
    /// 解析 INI `Category=`；空 → [`Self::Unspecified`]，已知名映射，其余 → [`Self::Other`]。
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "" => Self::Unspecified,
            s if s.eq_ignore_ascii_case("Soldier") => Self::Soldier,
            s if s.eq_ignore_ascii_case("Dog") => Self::Dog,
            s if s.eq_ignore_ascii_case("Civilian") => Self::Civilian,
            s if s.eq_ignore_ascii_case("Vehicle") => Self::Vehicle,
            s if s.eq_ignore_ascii_case("Armored") => Self::Armored,
            s if s.eq_ignore_ascii_case("AirPower") => Self::AirPower,
            s if s.eq_ignore_ascii_case("Ship") => Self::Ship,
            _ => Self::Other,
        }
    }

    /// 稳定诊断字面量（空表示未写）。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unspecified => "",
            Self::Soldier => "Soldier",
            Self::Dog => "Dog",
            Self::Civilian => "Civilian",
            Self::Vehicle => "Vehicle",
            Self::Armored => "Armored",
            Self::AirPower => "AirPower",
            Self::Ship => "Ship",
            Self::Other => "Other",
        }
    }
}

impl<'de> Deserialize<'de> for TechnoCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CategoryVisitor;

        impl<'de> Visitor<'de> for CategoryVisitor {
            type Value = TechnoCategory;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Category= techno category name")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TechnoCategory::parse(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TechnoCategory::parse(&v))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TechnoCategory::Unspecified)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TechnoCategory::Unspecified)
            }
        }

        deserializer.deserialize_any(CategoryVisitor)
    }
}
