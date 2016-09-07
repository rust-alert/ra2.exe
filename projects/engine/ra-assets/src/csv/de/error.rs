//! CSV Serde 反序列化错误。

use std::fmt;

use serde::de;

/// 从 Westwood CSV 行反序列化失败。
#[derive(Debug, Clone)]
pub struct CsvDeError {
    /// 可读原因。
    pub message: String,
    /// 列下标（若有）。
    pub column: Option<usize>,
    /// 结构体字段名（若有）。
    pub field: Option<String>,
}

impl CsvDeError {
    /// 构造自定义错误。
    pub fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            message: msg.to_string(),
            column: None,
            field: None,
        }
    }

    /// 附列下标。
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// 附字段名。
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

impl fmt::Display for CsvDeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.field, self.column) {
            (Some(name), Some(col)) => write!(f, "CSV 列 {col} 字段 `{name}`: {}", self.message),
            (Some(name), None) => write!(f, "CSV 字段 `{name}`: {}", self.message),
            (None, Some(col)) => write!(f, "CSV 列 {col}: {}", self.message),
            (None, None) => write!(f, "CSV 反序列化: {}", self.message),
        }
    }
}

impl std::error::Error for CsvDeError {}

impl de::Error for CsvDeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::custom(msg)
    }
}
