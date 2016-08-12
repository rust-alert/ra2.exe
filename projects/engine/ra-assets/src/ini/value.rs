//! INI 字段值的借用视图（仅用于同一次类型解码）。

use super::SourceSpan;

/// 尚未类型化的字段值视图。
#[derive(Debug, Clone, Copy)]
pub struct IniValue<'a> {
    /// 原文（已 trim 与否由调用方决定；此处保留 section 中的原始文本）。
    pub raw: &'a str,
    /// 来源位置。
    pub span: Option<SourceSpan>,
}

impl<'a> IniValue<'a> {
    /// 构造。
    pub fn new(raw: &'a str, span: Option<SourceSpan>) -> Self {
        Self { raw, span }
    }

    /// 去首尾空白后的原文。
    pub fn trimmed(self) -> Self {
        Self { raw: self.raw.trim(), span: self.span }
    }
}
