//! 从 rules 解析弹头 `Verses`（相对护甲倍率）。

use std::collections::HashMap;

use crate::ini::IniDocument;

/// RA2 护甲名在 `Verses` 列表中的固定顺序（11 项）。
pub const ARMOR_ORDER: [&str; 11] =
    ["none", "flak", "plate", "light", "medium", "heavy", "wood", "steel", "concrete", "special_1", "special_2"];

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
        let mut by_id = HashMap::new();
        for name in names {
            let id = name.as_ref().trim().to_ascii_uppercase();
            if id.is_empty() || by_id.contains_key(&id) {
                continue;
            }
            if let Some(wh) = parse_warhead(rules, &id) {
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

/// 护甲名 → `Verses` 下标；未知护甲按 `none`（0）。
pub fn armor_index(armor: &str) -> usize {
    let key = armor.trim().to_ascii_lowercase();
    ARMOR_ORDER.iter().position(|a| *a == key).unwrap_or(0)
}

fn parse_warhead(rules: &IniDocument, id: &str) -> Option<Warhead> {
    if !rules.has_section(id) {
        return None;
    }
    let mut verses = [100u32; 11];
    if let Some(raw) = rules.get(id, "Verses") {
        for (i, part) in raw.split(',').enumerate().take(11) {
            let s = part.trim().trim_end_matches('%').trim();
            if let Ok(v) = s.parse::<u32>() {
                verses[i] = v;
            }
        }
    }
    Some(Warhead { id: id.to_string(), verses })
}
