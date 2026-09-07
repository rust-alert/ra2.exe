//! 将 `oak-ini` AST 转为 [`ParsedIniDocument`]。

use ra_types::{RaError, RaResult};

use super::document::{IniEntry, IniSectionNode, ParsedIniDocument, SourceId, SourceSpan};

/// 用 `oak-ini` Westwood 方言解析 UTF-8 文本字节，并消费其 AST。
pub fn parse_with_oak(bytes: &[u8], source: SourceId) -> RaResult<ParsedIniDocument> {
    let text = std::str::from_utf8(bytes).map_err(|e| RaError::Parse(e.to_string()))?;
    let root = oak_ini::parse_with(text, &oak_ini::IniLanguage::westwood()).map_err(RaError::Parse)?;
    Ok(from_oak_root(root, source))
}

fn from_oak_root(root: oak_ini::IniRoot, source: SourceId) -> ParsedIniDocument {
    let leading = root.properties.into_iter().map(|p| entry_from_oak(p, source)).collect();
    let sections = root
        .sections
        .into_iter()
        .map(|s| IniSectionNode {
            name_key: s.name.to_ascii_uppercase(),
            name_raw: s.name,
            entries: s.properties.into_iter().map(|p| entry_from_oak(p, source)).collect(),
            span: Some(SourceSpan {
                source,
                start: s.span.start,
                end: s.span.end,
            }),
        })
        .collect();
    ParsedIniDocument { source, leading, sections }
}

fn entry_from_oak(p: oak_ini::ast::Property, source: SourceId) -> IniEntry {
    IniEntry {
        key_key: p.key.to_ascii_uppercase(),
        key_raw: p.key,
        value_raw: p.value,
        span: Some(SourceSpan {
            source,
            start: p.span.start,
            end: p.span.end,
        }),
    }
}
