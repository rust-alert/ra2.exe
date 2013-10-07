//! 通用 INI 文档（无红警语义；保序、保重复、保来源位置）。
//!
//! 由 `oak-ini` Westwood 方言 AST 转换而来。

use ra_types::RaResult;

use super::from_oak;

/// 输入来源编号（多文件栈中的一份）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceId(pub u32);

/// 源文本字节区间（与 `oak-ini` span 对齐：`start..end`）。
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
    /// 解析 UTF-8 INI 字节（`oak-ini` Westwood 方言）。
    pub fn parse(bytes: &[u8]) -> RaResult<Self> {
        from_oak::parse_with_oak(bytes, SourceId(0))
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

    /// 将编号键（`1=` / `2=` …）按数值序拼接，用于 IsoMapPack5 / OverlayPack。
    pub fn numbered_section_concat(&self, section: &str) -> Option<String> {
        let sec = self.section(section)?;
        let mut pairs: Vec<(u32, &str)> = Vec::new();
        for (k, v) in sec.pairs() {
            if let Ok(n) = k.parse::<u32>() {
                pairs.push((n, v));
            }
        }
        if pairs.is_empty() {
            return None;
        }
        pairs.sort_by_key(|(n, _)| *n);
        let mut out = String::new();
        for (_, v) in pairs {
            out.push_str(v);
        }
        Some(out)
    }
}
