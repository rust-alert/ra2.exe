//! UI / CSF 文案键。

use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use super::ini_string::{deserialize_upper, parse_upper};

/// CSF / `UIName=` 文案键（装载期大写归一，与 CSF 表一致）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct UiName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl UiName {
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

impl Deref for UiName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for UiName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for UiName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for UiName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for UiName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for UiName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for UiName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<UiName> for str {
    fn eq(&self, other: &UiName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<UiName> for &str {
    fn eq(&self, other: &UiName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for UiName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}
