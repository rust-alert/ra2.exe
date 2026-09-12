//! INI Serde 反序列化错误。

use std::fmt;

use serde::de;

/// 从 INI 节反序列化失败。
#[derive(Debug, Clone)]
pub struct IniDeError {
    /// 可读原因。
    pub message: String,
    /// 相关节名（原始拼写；若有）。
    pub section: Option<String>,
    /// 相关键（若有）。
    pub key: Option<String>,
}

impl IniDeError {
    /// 构造自定义错误。
    pub fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            message: msg.to_string(),
            section: None,
            key: None,
        }
    }

    /// 带节名的错误。
    pub fn with_section(mut self, section: impl Into<String>) -> Self {
        self.section = Some(section.into());
        self
    }

    /// 带键的错误。
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
}

impl fmt::Display for IniDeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.section, &self.key) {
            (Some(sec), Some(k)) => write!(f, "INI [{sec}] 字段 `{k}`: {}", self.message),
            (Some(sec), None) => write!(f, "INI [{sec}]: {}", self.message),
            (None, Some(k)) => write!(f, "INI 字段 `{k}`: {}", self.message),
            (None, None) => write!(f, "INI 反序列化: {}", self.message),
        }
    }
}

impl std::error::Error for IniDeError {}

impl de::Error for IniDeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::custom(msg)
    }
}
