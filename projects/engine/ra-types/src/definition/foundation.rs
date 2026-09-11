//! 建筑 `Foundation=` 占地。

use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

/// 建筑占地（逻辑格矩形）。
///
/// 原版还有命名特例（如闸门），本结构先覆盖常见 `WxH` / `WxHName` 前缀；
/// 无法解析时回退 `1x1`，并保留原始字符串供诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct Foundation {
    /// 横向格数（≥1）。
    pub width: u16,
    /// 纵向格数（≥1）。
    pub height: u16,
    /// INI 原始值（大写）；缺省为空。
    pub raw: String,
}

impl Default for Foundation {
    fn default() -> Self {
        Self { width: 1, height: 1, raw: String::new() }
    }
}

impl Foundation {
    /// 从 INI `Foundation=` 解析。支持 `2x2`、`3x5Refinery` 等以 `WxH` 开头的写法。
    pub fn parse(raw: &str) -> Self {
        let raw = raw.trim().to_ascii_uppercase();
        if raw.is_empty() {
            return Self::default();
        }
        let (width, height) = parse_wh_prefix(&raw).unwrap_or((1, 1));
        Self { width: width.max(1), height: height.max(1), raw }
    }

    /// 占地格数。
    pub fn cell_count(&self) -> u32 {
        u32::from(self.width).saturating_mul(u32::from(self.height))
    }
}

impl<'de> Deserialize<'de> for Foundation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FoundationVisitor;

        impl<'de> Visitor<'de> for FoundationVisitor {
            type Value = Foundation;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("Foundation= WxH string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Foundation::parse(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Foundation::parse(&v))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Foundation::default())
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Foundation::default())
            }
        }

        deserializer.deserialize_any(FoundationVisitor)
    }
}

/// 解析开头的 `(\d+)x(\d+)`，允许后缀字母。
pub fn parse_wh_prefix(raw: &str) -> Option<(u16, u16)> {
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i >= bytes.len() || bytes[i] != b'x' && bytes[i] != b'X' {
        return None;
    }
    let w: u16 = raw[..i].parse().ok()?;
    let j = i + 1;
    let mut k = j;
    while k < bytes.len() && bytes[k].is_ascii_digit() {
        k += 1;
    }
    if k == j {
        return None;
    }
    let h: u16 = raw[j..k].parse().ok()?;
    Some((w, h))
}
