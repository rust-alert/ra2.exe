//! 从 INI 值中抽取资源文件名引用（无游戏语义）。

use std::collections::HashSet;

use super::document::IniDocument;

/// 收集文档中形如 `*.shp` 的资源名（去重、保首次出现顺序）。
///
/// 扫描 leading 与各 section 的值；按空白 / `,` / `;` 切分，剥引号。
pub fn collect_shp_refs(doc: &IniDocument) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut push_value = |raw: &str| {
        for part in raw.split(|c: char| c.is_ascii_whitespace() || c == ',' || c == ';') {
            let t = part.trim().trim_matches('"');
            if t.len() < 5 {
                continue;
            }
            if !t.to_ascii_lowercase().ends_with(".shp") {
                continue;
            }
            let key = t.to_ascii_lowercase();
            if seen.insert(key) {
                out.push(t.to_string());
            }
        }
    };
    for e in &doc.leading {
        push_value(&e.value_raw);
    }
    for sec in &doc.sections {
        for e in &sec.entries {
            push_value(&e.value_raw);
        }
    }
    out
}
