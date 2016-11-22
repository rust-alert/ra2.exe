//! Overlay 类型表（装载期冻结；id 按声明顺序）。

use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use super::ini_string::{deserialize_upper, parse_upper};

/// Overlay 类型名（`[OverlayTypes]` 值；装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct OverlayName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl OverlayName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for OverlayName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for OverlayName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for OverlayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for OverlayName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for OverlayName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for OverlayName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for OverlayName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<OverlayName> for str {
    fn eq(&self, other: &OverlayName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<OverlayName> for &str {
    fn eq(&self, other: &OverlayName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for OverlayName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}

/// Overlay 类型注册表。
///
/// 内部 id 按 `[OverlayTypes]` **声明顺序**（值序列）编号，不按数字键留空洞。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverlayTypeRegistry {
    /// `overlay_id` → 类型名（大写）；下标即 OverlayPack 字节。
    names: Vec<OverlayName>,
    /// 与 `names` 对齐：该 id 是否可采（矿/宝石）。
    harvestable: Vec<bool>,
    /// 与 `names` 对齐：`NoUseTileLandType` 时按 `Land=` 得到的通行覆盖；`None` 表示不改 TMP 封格。
    land_pass_override: Vec<Option<bool>>,
}

impl OverlayTypeRegistry {
    /// 由已解析的名称、可采标记与通行覆盖构造（装载层填充）。
    pub fn from_entries(names: Vec<OverlayName>, harvestable: Vec<bool>, land_pass_override: Vec<Option<bool>>) -> Self {
        debug_assert_eq!(names.len(), harvestable.len());
        debug_assert_eq!(names.len(), land_pass_override.len());
        Self { names, harvestable, land_pass_override }
    }

    /// 已登记的类型数量。
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// 是否没有任何类型。
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// 按 overlay id 取类型名。
    pub fn name(&self, id: u8) -> Option<&str> {
        self.names.get(usize::from(id)).map(|n| n.as_str())
    }

    /// 该 overlay id 是否可采矿/宝石。
    pub fn is_harvestable(&self, id: u8) -> bool {
        self.harvestable.get(usize::from(id)).copied().unwrap_or(false)
    }

    /// `NoUseTileLandType` 覆盖下的目标通行性；无覆盖则 `None`。
    pub fn land_pass_override(&self, id: u8) -> Option<bool> {
        self.land_pass_override.get(usize::from(id)).copied().flatten()
    }
}
