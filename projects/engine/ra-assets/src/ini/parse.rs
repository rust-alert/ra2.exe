//! Westwood / RA2 方言 INI：自研行式解析（只吃 UTF-8）。
//!
//! 入参字节先经 [`decode_westwood_text`] 转成 UTF-8，再按行解析。
//! 与通用 INI 库差异过大，不依赖第三方解析器：
//! - `;` 与行内 `//` 注释（含 section 头尾随）
//! - 无 `=` 的装饰行直接忽略
//! - `Key=` 空值合法
//! - 重复键保序；查找大小写不敏感
//! - span 相对**解码后**的 UTF-8 文本

use ra_types::RaResult;

use super::document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};

/// 解析 Westwood 方言 INI：非 UTF-8 先转码，再走 UTF-8 行式解析。
pub fn parse_westwood(bytes: &[u8], source: SourceId) -> RaResult<IniDocument> {
    let text = decode_westwood_text(bytes);
    Ok(parse_utf8(&text, source))
}

/// 将原版 / 资料片 INI 字节转为 UTF-8 文本。
///
/// 优先按 UTF-8（可带 BOM）；否则按 Windows-1252（`encoding_rs`）解码。
pub fn decode_westwood_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let (cow, _encoding_used, _had_errors) = encoding_rs::WINDOWS_1252.decode(bytes);
    cow.into_owned()
}

#[doc(hidden)]
pub fn parse_utf8(text: &str, source: SourceId) -> IniDocument {
    let mut leading: Vec<IniEntry> = Vec::new();
    let mut sections: Vec<IniSection> = Vec::new();
    let mut current: Option<IniSection> = None;

    let mut offset = 0usize;
    for raw_line in text.split_inclusive('\n') {
        let line_start = offset;
        offset += raw_line.len();

        let body = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let body = body.strip_suffix('\r').unwrap_or(body);
        let code = strip_line_comment(body).trim();
        if code.is_empty() {
            continue;
        }

        if let Some(name_raw) = parse_section_name(code) {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
            let name_key = name_raw.to_ascii_uppercase();
            current = Some(IniSection {
                name_raw,
                name_key,
                entries: Vec::new(),
                span: Some(SourceSpan { source, start: line_start, end: line_start + body.len() }),
            });
            continue;
        }

        if let Some((key_raw, value_raw)) = parse_property(code) {
            let entry = IniEntry {
                key_key: key_raw.to_ascii_uppercase(),
                key_raw,
                value_raw,
                span: Some(SourceSpan { source, start: line_start, end: line_start + body.len() }),
            };
            if let Some(sec) = current.as_mut() {
                sec.entries.push(entry);
            }
            else {
                leading.push(entry);
            }
            continue;
        }

        // 装饰行 / 脏数据：原版会忽略，这里同样跳过。
    }

    if let Some(sec) = current.take() {
        sections.push(sec);
    }

    IniDocument { source, leading, sections }
}

/// 去掉行尾 `;` / `//` 注释；双引号内与 `://` 不截断。
pub fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut in_quotes = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if in_quotes {
            i += 1;
            continue;
        }
        if b == b';' {
            return &line[..i];
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            let ok = i == 0 || bytes[i - 1].is_ascii_whitespace();
            if ok {
                return &line[..i];
            }
        }
        i += 1;
    }
    line
}

#[doc(hidden)]
pub fn parse_section_name(code: &str) -> Option<String> {
    let code = code.trim();
    if !code.starts_with('[') {
        return None;
    }
    let end = code.find(']')?;
    let name = code[1..end].trim();
    if name.is_empty() {
        return None;
    }
    // `]` 后若还有非空白，视为畸形节头，交给装饰行逻辑丢掉。
    if !code[end + 1..].trim().is_empty() {
        return None;
    }
    Some(name.to_string())
}

#[doc(hidden)]
pub fn parse_property(code: &str) -> Option<(String, String)> {
    let eq = code.find('=')?;
    let key = code[..eq].trim();
    if key.is_empty() {
        return None;
    }
    let value = code[eq + 1..].trim();
    Some((key.to_string(), value.to_string()))
}
