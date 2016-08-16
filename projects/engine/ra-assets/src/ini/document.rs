//! 通用 INI 文档（无红警语义；保序、保重复、保来源位置）。
//!
//! 由自研 Westwood 方言解析器产出。

use ra_types::RaResult;

use super::parse;

/// 输入来源编号（多文件栈中的一份）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceId(pub u32);

/// 源文本字节区间（`start..end`，相对**解码后** UTF-8 文本）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// 所属来源。
    pub source: SourceId,
    /// 起始字节偏移。
    pub start: usize,
    /// 结束字节偏移（不含）。
    pub end: usize,
}

/// 一条键值（可重复；查找时后者覆盖前者）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniEntry {
    /// 原始键拼写。
    pub key_raw: String,
    /// 比较键（大写）。
    pub key_key: String,
    /// 原始值文本（尚未做类型规范化）。
    pub value_raw: String,
    /// 源位置。
    pub span: Option<SourceSpan>,
}

/// 一个 section。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniSection {
    /// 原始 section 名拼写。
    pub name_raw: String,
    /// 比较名（大写；前导节为空串）。
    pub name_key: String,
    /// 节内条目（保序，可含重复键）。
    pub entries: Vec<IniEntry>,
    /// 源位置。
    pub span: Option<SourceSpan>,
}

impl IniSection {
    /// 保序键值对（原始拼写）。
    pub fn pairs(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.entries.iter().map(|e| (e.key_raw.as_str(), e.value_raw.as_str()))
    }

    /// 按比较键取最后一次出现的值。
    pub fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_ascii_uppercase();
        self.entries.iter().rev().find(|e| e.key_key == key).map(|e| e.value_raw.as_str())
    }

    /// 将本节一次性反序列化为强类型（字段名需与 INI 键拼写一致，可用 `serde(rename)`）。
    pub fn deserialize<'de, T>(&'de self) -> Result<T, super::de::IniDeError>
    where
        T: serde::Deserialize<'de>,
    {
        super::de::from_section(self)
    }
}

/// 完整 INI 文档。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IniDocument {
    /// 来源。
    pub source: SourceId,
    /// 无 section 标题前的全局属性。
    pub leading: Vec<IniEntry>,
    /// 具名 section（保序）。
    pub sections: Vec<IniSection>,
}

impl IniDocument {
    /// 解析 INI 字节（Westwood / RA2 方言）。
    ///
    /// 非 UTF-8 输入先经 `encoding_rs`（Windows-1252）转成 UTF-8，再解析。
    pub fn parse(bytes: &[u8]) -> RaResult<Self> {
        parse::parse_westwood(bytes, SourceId(0))
    }

    /// 按比较键查找最后一个匹配 section。
    pub fn section(&self, name: &str) -> Option<&IniSection> {
        let key = name.to_ascii_uppercase();
        self.sections.iter().rev().find(|s| s.name_key == key)
    }

    /// 是否存在匹配 section（大小写不敏感）。
    pub fn has_section(&self, name: &str) -> bool {
        self.section(name).is_some()
    }

    /// 在指定 section 上按比较键取最后一次出现的值。
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.section(section)?.get(key)
    }
}
