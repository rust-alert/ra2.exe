//! 战役表键。

use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use super::ini_string::{deserialize_upper, parse_upper};

/// 战役表 `[Battles]` / 战役节 id（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CampaignName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl CampaignName {
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

impl Deref for CampaignName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for CampaignName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for CampaignName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for CampaignName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for CampaignName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for CampaignName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for CampaignName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<CampaignName> for str {
    fn eq(&self, other: &CampaignName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<CampaignName> for &str {
    fn eq(&self, other: &CampaignName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for CampaignName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}
