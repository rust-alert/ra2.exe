//! 生产类别与工厂生产配置。

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

/// 工厂可生产的单位大类（由 INI `Factory=` 等解释而来）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductionCategory {
    /// 步兵。
    Infantry,
    /// 载具。
    Vehicle,
    /// 飞行器。
    Aircraft,
    /// 建筑（建造栏）。
    Building,
}

impl ProductionCategory {
    /// 解析 INI `Factory=`；未知非空串回落 [`Self::Vehicle`]。
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "infantrytype" | "infantry" => Self::Infantry,
            "unittype" | "vehicle" | "unit" => Self::Vehicle,
            "aircrafttype" | "aircraft" => Self::Aircraft,
            "buildingtype" | "building" => Self::Building,
            _ => Self::Vehicle,
        }
    }

    /// 解析可选 `Factory=`：缺省 / 空串 → `None`。
    pub fn parse_optional(raw: &str) -> Option<Self> {
        if raw.trim().is_empty() {
            None
        } else {
            Some(Self::parse(raw))
        }
    }
}

impl<'de> Deserialize<'de> for ProductionCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CatVisitor;

        impl<'de> Visitor<'de> for CatVisitor {
            type Value = ProductionCategory;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Factory= production category")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ProductionCategory::parse(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ProductionCategory::parse(&v))
            }
        }

        deserializer.deserialize_any(CatVisitor)
    }
}

/// 可选 `Factory=`：空 / 缺键 → [`None`]。
pub fn deserialize_optional_factory<'de, D>(deserializer: D) -> Result<Option<ProductionCategory>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptVisitor;

    impl<'de> Visitor<'de> for OptVisitor {
        type Value = Option<ProductionCategory>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("optional Factory= production category")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(ProductionCategory::parse_optional(v))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(ProductionCategory::parse_optional(&v))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(OptVisitor)
}

/// 生产设施配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionProfile {
    /// 该工厂产出的类别。
    pub category: ProductionCategory,
}

/// 生产 / 前置定义集合（骨架扩展位）。
#[derive(Debug, Clone, Default)]
pub struct ProductionDefinitions {
    /// 条目数占位（细表后续接入）。
    pub count: u32,
}
