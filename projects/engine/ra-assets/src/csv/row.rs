//! Westwood CSV 行（保序字段；可含空字段）。

/// 一行中的单个字段（已 trim；空串表示显式空列）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvField {
    /// 字段文本（已去掉首尾空白）。
    pub value: String,
}

impl CsvField {
    /// 是否为空列。
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// 借用原文。
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

/// 解析后的一行 Westwood CSV。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CsvRow {
    /// 按出现顺序的字段（保留空列）。
    pub fields: Vec<CsvField>,
}

impl CsvRow {
    /// 字段数。
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// 是否无字段。
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// 按下标取字段文本。
    pub fn get(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(CsvField::as_str)
    }
}
