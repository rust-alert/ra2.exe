//! 通用 INI 文档模型（无红警语义；保序、保重复、保来源位置）。

/// 输入来源编号（多文件栈中的一份）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceId(pub u32);

/// 源文本字节区间（含端点语义与 `oak-ini` span 对齐：`start..end`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// 所属来源。
    pub source: SourceId,
    /// 起始字节偏移。
    pub start: usize,
    /// 结束字节偏移（不含）。
    pub end: usize,
}

/// 一条键值（可重复；后出现者在合并层覆盖）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniEntry {
    /// 原始键拼写。
    pub key_raw: String,
    /// 比较键（目前为大写）。
    pub key_key: String,
    /// 原始值文本（尚未做类型规范化）。
    pub value_raw: String,
    /// 源位置（若解析器提供）。
    pub span: Option<SourceSpan>,
}

/// 一个 section。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniSectionNode {
    /// 原始 section 名拼写。
    pub name_raw: String,
    /// 比较名（目前为大写；前导节为空串）。
    pub name_key: String,
    /// 节内条目（保序，可含重复键）。
    pub entries: Vec<IniEntry>,
    /// 源位置。
    pub span: Option<SourceSpan>,
}

/// 通用解析结果文档。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedIniDocument {
    /// 来源。
    pub source: SourceId,
    /// 无 section 标题前的全局属性。
    pub leading: Vec<IniEntry>,
    /// 具名 section（保序）。
    pub sections: Vec<IniSectionNode>,
}

impl ParsedIniDocument {
    /// 按比较键查找最后一个匹配 section。
    pub fn section_by_key(&self, name_key: &str) -> Option<&IniSectionNode> {
        let key = name_key.to_ascii_uppercase();
        self.sections.iter().rev().find(|s| s.name_key == key)
    }

    /// 在指定 section 上按比较键取**最后一次**出现的值（对齐旧「后者覆盖」查找语义）。
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        let sec = self.section_by_key(section)?;
        let key = key.to_ascii_uppercase();
        sec.entries.iter().rev().find(|e| e.key_key == key).map(|e| e.value_raw.as_str())
    }
}
