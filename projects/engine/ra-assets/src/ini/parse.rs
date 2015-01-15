//! Westwood / RA2 方言 INI：自研行式解析。
//!
//! 与通用 INI 库差异过大，不依赖第三方：
//! - `;` 与行内 `//` 注释（含 section 头尾随）
//! - 无 `=` 的装饰行直接忽略
//! - `Key=` 空值合法
//! - 重复键保序；查找大小写不敏感
//! - 源字节偏移取自原文（不先删行再解析）

use ra_types::{RaError, RaResult};

use super::document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};

/// 解析 UTF-8 Westwood 方言 INI。
pub(crate) fn parse_westwood(bytes: &[u8], source: SourceId) -> RaResult<IniDocument> {
    let text = std::str::from_utf8(bytes).map_err(|e| RaError::Parse(e.to_string()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);

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
                span: Some(SourceSpan {
                    source,
                    start: line_start,
                    end: line_start + body.len(),
                }),
            });
            continue;
        }

        if let Some((key_raw, value_raw)) = parse_property(code) {
            let entry = IniEntry {
                key_key: key_raw.to_ascii_uppercase(),
                key_raw,
                value_raw,
                span: Some(SourceSpan {
                    source,
                    start: line_start,
                    end: line_start + body.len(),
                }),
            };
            if let Some(sec) = current.as_mut() {
                sec.entries.push(entry);
            } else {
                leading.push(entry);
            }
            continue;
        }

        // 装饰行 / 脏数据：原版会忽略，这里同样跳过。
    }

    if let Some(sec) = current.take() {
        sections.push(sec);
    }

    Ok(IniDocument { source, leading, sections })
}

/// 去掉行尾 `;` / `//` 注释；双引号内与 `://` 不截断。
fn strip_line_comment(line: &str) -> &str {
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

fn parse_section_name(code: &str) -> Option<String> {
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

fn parse_property(code: &str) -> Option<(String, String)> {
    let eq = code.find('=')?;
    let key = code[..eq].trim();
    if key.is_empty() {
        return None;
    }
    let value = code[eq + 1..].trim();
    Some((key.to_string(), value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_trailing_slash_slash_comment() {
        let raw = "[MirageWH]    // Supposed to be a heat ray.\nVerses=100%,100%,80%\n";
        let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
        assert!(doc.has_section("MirageWH"));
        assert_eq!(doc.get("MirageWH", "Verses"), Some("100%,100%,80%"));
    }

    #[test]
    fn drops_decorative_lines_without_equal() {
        let raw = "[General]\n***Crazy Ivan stuff***\nStrength=100\n// PCG comment\nName=Foo\n842-GAWETH_ED\n";
        let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
        assert_eq!(doc.get("General", "Strength"), Some("100"));
        assert_eq!(doc.get("General", "Name"), Some("Foo"));
        assert_eq!(doc.section("General").unwrap().entries.len(), 2);
    }

    #[test]
    fn empty_value_and_semicolon_comment() {
        let raw = "[A]\nEmpty=\nName=Tank ; unit name\n";
        let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
        assert_eq!(doc.get("A", "Empty"), Some(""));
        assert_eq!(doc.get("A", "Name"), Some("Tank"));
    }

    #[test]
    fn keeps_url_like_double_slash_in_value() {
        let raw = "[Net]\nUrl=http://example.com/path\n";
        let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
        assert_eq!(doc.get("Net", "Url"), Some("http://example.com/path"));
    }

    #[test]
    fn leading_properties_before_first_section() {
        let raw = "Pre=1\n[S]\nK=2\n";
        let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
        assert_eq!(doc.leading.len(), 1);
        assert_eq!(doc.leading[0].key_raw, "Pre");
        assert_eq!(doc.get("S", "K"), Some("2"));
    }

    #[test]
    fn span_uses_original_offsets() {
        let raw = "[A]\nKey=1\n";
        let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
        let sec = doc.section("A").unwrap();
        let span = sec.span.unwrap();
        assert_eq!(&raw[span.start..span.end], "[A]");
        let entry = &sec.entries[0];
        let es = entry.span.unwrap();
        assert_eq!(&raw[es.start..es.end], "Key=1");
    }
}
