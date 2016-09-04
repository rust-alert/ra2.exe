//! 从 rules 解析弹头 `Verses`（相对护甲倍率）。

use std::collections::HashMap;
use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, SeqAccess, Visitor};

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView};
use ra_types::WarheadVerses;

pub use ra_types::{ARMOR_ORDER, armor_index};

/// 弹头：对各护甲的伤害百分比（默认全 100）及溅射 / 卧倒倍率。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warhead {
    /// 弹头 id（大写）。
    pub id: String,
    /// 对应 [`ARMOR_ORDER`] 的百分比倍率。
    pub verses: WarheadVerses,
    /// `Spread=` 溅射半径（格）；缺省 0。
    pub spread: u32,
    /// `ProneDamage=` 对卧倒单位的伤害百分比；缺省 100。
    pub prone_damage: u32,
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
    /// `Verses=`：逗号列表一次落到 [`WarheadVerses`]，不再经 `Vec<String>`。
    #[serde(rename = "Verses", default, deserialize_with = "deserialize_warhead_verses")]
    verses: WarheadVerses,
    #[serde(rename = "Spread", default)]
    spread: u32,
    #[serde(rename = "ProneDamage", default = "default_prone_damage")]
    prone_damage: u32,
}

fn default_prone_damage() -> u32 {
    100
}

fn parse_warhead(view: LayeredIniView<'_>, id: &str) -> Option<Warhead> {
    let section = view.section(id)?;
    let fields: WarheadSectionFields = section.deserialize().ok()?;
    Some(Warhead {
        id: id.to_string(),
        verses: fields.verses,
        spread: fields.spread,
        prone_damage: fields.prone_damage,
    })
}

/// 将 `Verses=` 标量（或逗号拆分序列）一次解码为 [`WarheadVerses`]。
///
/// - 缺键 / 空串 → 全 100
/// - 项可带尾随 `%`
/// - 不足 11 项的槽位保持 100；多于 11 项截断
/// - 单槽非法文本保持该槽缺省 100（不整段失败，对齐原版容错）
fn deserialize_warhead_verses<'de, D>(deserializer: D) -> Result<WarheadVerses, D::Error>
where
    D: Deserializer<'de>,
{
    struct VersesVisitor;

    impl<'de> Visitor<'de> for VersesVisitor {
        type Value = WarheadVerses;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("comma-separated Verses percentages (optional % suffix)")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(parse_verses_text(v))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(parse_verses_text(&v))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut out = WarheadVerses::default();
            let mut i = 0usize;
            while let Some(part) = seq.next_element::<String>()? {
                if i >= 11 {
                    break;
                }
                if let Some(v) = parse_verse_token(&part) {
                    out.0[i] = v;
                }
                i += 1;
            }
            Ok(out)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(WarheadVerses::default())
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(WarheadVerses::default())
        }
    }

    deserializer.deserialize_any(VersesVisitor)
}

fn parse_verses_text(raw: &str) -> WarheadVerses {
    let mut out = WarheadVerses::default();
    let mut i = 0usize;
    for part in raw.split(|c| c == ',' || c == ';') {
        if i >= 11 {
            break;
        }
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(v) = parse_verse_token(part) {
            out.0[i] = v;
        }
        i += 1;
    }
    out
}

fn parse_verse_token(raw: &str) -> Option<u32> {
    let s = raw.trim().trim_end_matches('%').trim();
    if s.is_empty() {
        return None;
    }
    s.parse().ok()
}
