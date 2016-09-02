//! 编号键字段拼接（地图 pack 等资源编码，不属于通用文档模型）。

use super::document::{IniDocument, IniSection};
use super::merge::{EntryMergePolicy, IniMergePolicy, LayeredIniView};

/// 将节内编号键（`1=` / `2=` …）按数值序拼接值文本。
pub fn concat_numbered_values(section: &IniSection) -> Option<String> {
    let mut pairs: Vec<(u32, &str)> = Vec::new();
    for (k, v) in section.pairs() {
        if let Ok(n) = k.parse::<u32>() {
            pairs.push((n, v));
        }
    }
    if pairs.is_empty() {
        return None;
    }
    pairs.sort_by_key(|(n, _)| *n);
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
