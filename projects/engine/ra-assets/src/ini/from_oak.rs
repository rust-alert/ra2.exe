//! 将 `oak-ini` AST 转为 [`IniDocument`]。

use ra_types::{RaError, RaResult};

use super::document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};

/// 用 `oak-ini` Westwood 方言解析 UTF-8 文本字节。
///
/// 解析前会做与原版一致的脏行软化：非节头、非 `;` 注释且不含 `=` 的行视为装饰/脏数据并丢掉，
/// 否则 `***Crazy Ivan stuff***`、`// …`、`842-GAWETH_ED` 等会让严格键值解析在 `=` 处失败。
pub(crate) fn parse_with_oak(bytes: &[u8], source: SourceId) -> RaResult<IniDocument> {
    let text = std::str::from_utf8(bytes).map_err(|e| RaError::Parse(e.to_string()))?;
    let sanitized = sanitize_westwood_ini(text);
    let root = oak_ini::parse_with(&sanitized, &oak_ini::IniLanguage::westwood()).map_err(RaError::Parse)?;
    Ok(from_oak_root(root, source))
}

/// 丢掉原版会忽略、但严格键值解析会当成缺 `=` 键的行。
fn sanitize_westwood_ini(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let (body, nl) = match line.strip_suffix('\n') {
            Some(b) => (b.strip_suffix('\r').unwrap_or(b), "\n"),
            None => (line.strip_suffix('\r').unwrap_or(line), ""),
        };
        let trimmed = body.trim();
        if should_drop_westwood_line(trimmed) {
            continue;
        }
        out.push_str(body);
        out.push_str(nl);
    }
    out
}

fn should_drop_westwood_line(trimmed: &str) -> bool {
    if trimmed.is_empty() || trimmed.starts_with(';') {
        return false;
    }
    if trimmed.starts_with('[') && trimmed.contains(']') {
        return false;
    }
    // 无 `=`：装饰行、C++ 风格 `//`、把 `=` 误打成 `-` 的死键等。
    !trimmed.contains('=')
}

fn from_oak_root(root: oak_ini::IniRoot, source: SourceId) -> IniDocument {
    let leading = root.properties.into_iter().map(|p| entry_from_oak(p, source)).collect();
    let sections = root
        .sections
        .into_iter()
        .map(|s| IniSection {
            name_key: s.name.to_ascii_uppercase(),
            name_raw: s.name,
            entries: s.properties.into_iter().map(|p| entry_from_oak(p, source)).collect(),
            span: Some(SourceSpan { source, start: s.span.start, end: s.span.end }),
        })
        .collect();
    IniDocument { source, leading, sections }
}

fn entry_from_oak(p: oak_ini::ast::Property, source: SourceId) -> IniEntry {
    IniEntry {
        key_key: p.key.to_ascii_uppercase(),
        key_raw: p.key,
        value_raw: p.value,
        span: Some(SourceSpan { source, start: p.span.start, end: p.span.end }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_decorative_lines_without_equal() {
        let raw = "[General]\n***Crazy Ivan stuff***\nStrength=100\n// PCG comment\nName=Foo\n842-GAWETH_ED\n";
        let clean = sanitize_westwood_ini(raw);
        assert!(!clean.contains("Crazy"));
        assert!(!clean.contains("// PCG"));
        assert!(!clean.contains("GAWETH"));
        assert!(clean.contains("Strength=100"));
        assert!(clean.contains("Name=Foo"));
        let doc = parse_with_oak(raw.as_bytes(), SourceId::default()).unwrap();
        assert_eq!(doc.get("General", "Strength"), Some("100"));
        assert_eq!(doc.get("General", "Name"), Some("Foo"));
    }
}
