//! INI 字段值的借用视图（仅用于同一次类型解码）。

use super::SourceSpan;

/// 尚未类型化的字段值视图（带 section/key 路径，供诊断与 `deserialize_with`）。
#[derive(Debug, Clone, Copy)]
pub struct IniValue<'a> {
    /// 原文（已 trim 与否由调用方决定；此处保留 section 中的原始文本）。
    pub raw: &'a str,
    /// 来源位置。
    pub span: Option<SourceSpan>,
    /// 所属节名（原始拼写；无标题前导节为空串）。
    pub section: &'a str,
    /// 键名（原始拼写）。
    pub key: &'a str,
}

impl<'a> IniValue<'a> {
    /// 构造。
    pub fn new(raw: &'a str, span: Option<SourceSpan>, section: &'a str, key: &'a str) -> Self {
        Self { raw, span, section, key }
    }

    /// 去首尾空白后的原文（保留路径）。
    pub fn trimmed(self) -> Self {
        Self {
            raw: self.raw.trim(),
            span: self.span,
            section: self.section,
            key: self.key,
        }
    }
}
