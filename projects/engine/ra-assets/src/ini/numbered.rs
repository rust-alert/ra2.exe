//! 编号键字段拼接（地图 pack 等资源编码，不属于通用文档模型）。

use super::document::{IniDocument, IniSection};
use super::merge::{EntryMergePolicy, IniMergePolicy, LayeredIniView};

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

/// 在文档中查找节并拼接编号键值（经 [`LayeredIniView::numbered_pack_concat`]）。
pub fn numbered_section_concat(doc: &IniDocument, section: &str) -> Option<String> {
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::NumberedPack,
    };
    let docs = std::slice::from_ref(doc);
    LayeredIniView::new(docs, &policy).numbered_pack_concat(section)
}
