//! rules `[Colors]`：名称 → HSV 方案（INI 字段解释，供呈现 / 适配使用）。

use std::collections::HashMap;

use super::house_remap::Hsv;
use crate::{
    image::pal::Palette,
    ini::{IniDocument, IniMergePolicy, LayeredIniView},
};

/// 零售 `[Colors]` 表。
#[derive(Debug, Clone, Default)]
pub struct ColorSchemes {
    by_name: HashMap<String, Hsv>,
}

impl ColorSchemes {
    /// 从 rules 文档解析；缺节则空表。
    pub fn from_rules(doc: &IniDocument) -> Self {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(doc);
        Self::from_layered(LayeredIniView::new(docs, &policy))
    }

    /// 从层叠 rules 视图解析 `[Colors]`。
    pub fn from_layered(view: LayeredIniView<'_>) -> Self {
        let mut by_name = HashMap::new();
        let Some(sec) = view.section("Colors")
        else {
            return Self { by_name };
        };
        for key in sec.keys() {
            let Some(value) = sec.get(key)
            else {
                continue;
            };
            if let Some(hsv) = parse_hsv(value.trimmed().raw) {
                by_name.insert(key.to_ascii_uppercase(), hsv);
            }
        }
        Self { by_name }
    }

    /// 按方案名取 HSV（大小写不敏感）。
    pub fn get(&self, name: &str) -> Option<Hsv> {
        self.by_name.get(&name.to_ascii_uppercase()).copied()
    }

    /// 阵营节 `Color=` → HSV（含 `Neutral` / `Special` / `Civilian` 的 Grey 等方案）。
    pub fn hsv_for_house(&self, rules: &IniDocument, house: &str) -> Option<Hsv> {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(rules);
        self.hsv_for_house_layered(LayeredIniView::new(docs, &policy), house)
    }

    /// 层叠 rules 下阵营节 `Color=` → HSV。
    pub fn hsv_for_house_layered(&self, view: LayeredIniView<'_>, house: &str) -> Option<Hsv> {
        let scheme = view.get(house, "Color")?.trimmed().raw;
        self.get(scheme)
    }

    /// 阵营 HSV remap；无方案时回退 `Palette::for_owner`。
    ///
    /// 中立等必须走 `Color=Grey`：`unittem.pal` 默认 16..=31 色带偏红，跳过 remap 会把民房画成「有归属红」。
    pub fn palette_for_house(&self, rules: &IniDocument, base: &Palette, owner: &str) -> Palette {
        if let Some(hsv) = self.hsv_for_house(rules, owner) {
            return base.with_hsv_remap(hsv);
        }
        base.for_owner(owner)
    }

    /// 已登记方案数。
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// 是否为空表。
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

fn parse_hsv(value: &str) -> Option<Hsv> {
    let mut parts = value.split(',').map(str::trim);
    let h: u8 = parts.next()?.parse().ok()?;
    let s: u8 = parts.next()?.parse().ok()?;
    let v: u8 = parts.next()?.parse().ok()?;
    Some(Hsv { h, s, v })
}
