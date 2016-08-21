//! 从 rules 解析弹头 `Verses`（相对护甲倍率）。

use std::collections::HashMap;

use serde::Deserialize;

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView};

pub use ra_types::{ARMOR_ORDER, armor_index};

/// 弹头：对各护甲的伤害百分比（默认全 100）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warhead {
    /// 弹头 id（大写）。
    pub id: String,
    /// 对应 [`ARMOR_ORDER`] 的百分比倍率。
    pub verses: [u32; 11],
}

/// `warhead_id` → 解析后的弹头。
#[derive(Debug, Clone, Default)]
pub struct WarheadRegistry {
    by_id: HashMap<String, Warhead>,
}

impl WarheadRegistry {
    /// 解析指定弹头名列表（大小写不敏感节名）。
    pub fn from_names(rules: &IniDocument, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(rules);
        Self::from_names_layered(LayeredIniView::new(docs, &policy), names)
    }

    /// 从层叠 rules 视图解析指定弹头名列表。
    pub fn from_names_layered(view: LayeredIniView<'_>, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut by_id = HashMap::new();
        for name in names {
            let id = name.as_ref().trim().to_ascii_uppercase();
            if id.is_empty() || by_id.contains_key(&id) {
                continue;
            }
            if let Some(wh) = parse_warhead(view, &id) {
                by_id.insert(id, wh);
            }
        }
        Self { by_id }
    }

    /// 按 id 查找（大小写不敏感）。
    pub fn get(&self, id: &str) -> Option<&Warhead> {
        self.by_id.get(&id.to_ascii_uppercase())
    }

    /// 已解析弹头数。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// 是否为空表。
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[derive(Debug, Deserialize)]
struct WarheadSectionFields {
    #[serde(rename = "Verses", default)]
    verses: Vec<String>,
}

fn parse_warhead(view: LayeredIniView<'_>, id: &str) -> Option<Warhead> {
    let section = view.section(id)?;
    let fields: WarheadSectionFields = section.deserialize().ok()?;
    let mut verses = [100u32; 11];
    for (i, part) in fields.verses.iter().enumerate().take(11) {
        let s = part.trim().trim_end_matches('%').trim();
        if let Ok(v) = s.parse::<u32>() {
            verses[i] = v;
        }
    }
    Some(Warhead { id: id.to_string(), verses })
}
