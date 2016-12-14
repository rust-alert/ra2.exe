//! 编号键字段拼接（地图 pack 等资源编码，不属于通用文档模型）。

use super::document::{IniDocument, IniSection};

/// 解析编号键（`0` / `1` …）；非数字返回 `None`。
pub fn parse_numbered_key(key: &str) -> Option<u32> {
    key.trim().parse().ok()
}

/// 收集节内编号键（`0=` / `1=` …），按数值升序返回 `(index, value)`。
pub fn numbered_pairs(section: &IniSection) -> Vec<(u32, &str)> {
    let mut pairs: Vec<(u32, &str)> = Vec::new();
    for (k, v) in section.pairs() {
        if let Some(n) = parse_numbered_key(k) {
            pairs.push((n, v));
        }
    }
    pairs.sort_by_key(|(n, _)| *n);
    pairs
}

/// 将节内编号键（`1=` / `2=` …）按数值序拼接值文本。
pub fn concat_numbered_values(section: &IniSection) -> Option<String> {
    let pairs = numbered_pairs(section);
    if pairs.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (_, v) in pairs {
        out.push_str(v);
    }
    Some(out)
}

/// 在文档中查找节并返回编号键分片（单层按索引序，不拼接）。
pub fn numbered_section_parts<'a>(doc: &'a IniDocument, section: &str) -> Option<Vec<&'a str>> {
    let section = doc.section(section)?;
    let pairs = numbered_pairs(section);
    if pairs.is_empty() {
        return None;
    }
    Some(pairs.into_iter().map(|(_, v)| v).collect())
}

/// 在文档中查找节并拼接编号键值。
///
/// 多层覆盖请用 [`super::merge::LayeredIniView::numbered_pack_parts`] /
/// [`super::merge::LayeredIniView::numbered_pack_concat`]。
/// 解码路径请优先 [`numbered_section_parts`]。
pub fn numbered_section_concat(doc: &IniDocument, section: &str) -> Option<String> {
    let parts = numbered_section_parts(doc, section)?;
    let mut out = String::new();
    for part in parts {
        out.push_str(part);
    }
    Some(out)
}
