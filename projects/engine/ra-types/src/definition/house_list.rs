//! 阵营 / 房屋允许名单（`Owner=` / `RequiredHouses=` / `ForbiddenHouses=`）。

use std::fmt;

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::Deserialize;

use super::HouseName;

/// 装载期一次解码后的房屋名单；空名单语义由字段约定（见各字段文档）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseAllowList {
    /// 大写房屋名；空 = 本名单无约束项。
    houses: Vec<HouseName>,
}

impl HouseAllowList {
    /// 空名单。
    pub fn empty() -> Self {
        Self { houses: Vec::new() }
    }

    /// 由房屋名构造（再排序去重）。
    pub fn from_houses(houses: impl IntoIterator<Item = impl Into<HouseName>>) -> Self {
        let mut houses: Vec<HouseName> = houses
            .into_iter()
            .map(Into::into)
            .filter(|h| !h.is_empty())
            .collect();
        houses.sort_unstable_by(|a, b| a.as_str().cmp(b.as_str()));
        houses.dedup();
        Self { houses }
    }

    /// 解析 `Owner=` 原文：按 `,` / `;` / `|` 拆分；空串 → 空名单（表示不限）。
    pub fn parse_owner(raw: &str) -> Self {
        Self::parse_delimited(raw)
    }

    /// 解析 `RequiredHouses=` / `ForbiddenHouses=` 分隔列表。
    pub fn parse_csv(raw: &str) -> Self {
        Self::parse_delimited(raw)
    }

    fn parse_delimited(raw: &str) -> Self {
        let raw = raw.trim();
        if raw.is_empty() {
            return Self::empty();
        }
        Self::from_houses(
            raw.split(|c| c == ',' || c == ';' || c == '|')
                .map(str::trim)
                .filter(|s| !s.is_empty()),
        )
    }

    /// 是否为空名单。
    pub fn is_empty(&self) -> bool {
        self.houses.is_empty()
    }

    /// 名单长度。
    pub fn len(&self) -> usize {
        self.houses.len()
    }

    /// 迭代房屋名。
    pub fn iter(&self) -> impl Iterator<Item = &HouseName> {
        self.houses.iter()
    }

    /// `Owner=` 语义：空名单 = 不限；否则 house 须命中其一。
    pub fn owner_allows(&self, house: &str) -> bool {
        if self.houses.is_empty() {
            return true;
        }
        self.contains(house)
    }

    /// `RequiredHouses=` 语义：空名单 = 不限制；非空则须命中。
    pub fn required_allows(&self, house: &str) -> bool {
        if self.houses.is_empty() {
            return true;
        }
        self.contains(house)
    }

    /// `ForbiddenHouses=` 语义：命中任一则禁止；空名单 = 不禁止。
    pub fn forbids(&self, house: &str) -> bool {
        !self.houses.is_empty() && self.contains(house)
    }

    fn contains(&self, house: &str) -> bool {
        let want = HouseName::parse(house);
        self.houses.iter().any(|h| h == &want)
    }
}

impl<'de> Deserialize<'de> for HouseAllowList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ListVisitor;

        impl<'de> Visitor<'de> for ListVisitor {
            type Value = HouseAllowList;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("comma-separated house names")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(HouseAllowList::parse_delimited(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(HouseAllowList::parse_delimited(&v))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut houses = Vec::new();
                while let Some(part) = seq.next_element::<HouseName>()? {
                    if !part.is_empty() {
                        houses.push(part);
                    }
                }
                Ok(HouseAllowList::from_houses(houses))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(HouseAllowList::empty())
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(HouseAllowList::empty())
            }
        }

        deserializer.deserialize_any(ListVisitor)
    }
}
