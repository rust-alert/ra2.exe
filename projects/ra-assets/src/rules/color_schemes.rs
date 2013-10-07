//! rules `[Colors]`：名称 → HSV 方案（INI 字段解释，供呈现 / 适配使用）。

use std::collections::HashMap;

use super::house_remap::Hsv;
use crate::{image::pal::Palette, ini::IniDocument};

/// 零售 `[Colors]` 表。
#[derive(Debug, Clone, Default)]
pub struct ColorSchemes {
    by_name: HashMap<String, Hsv>,
}

impl ColorSchemes {
    /// 从 rules 文档解析；缺节则空表。
    pub fn from_rules(doc: &IniDocument) -> Self {
        let mut by_name = HashMap::new();
        let Some(sec) = doc.section("Colors")
        else {
            return Self { by_name };
        };
        for (name, value) in sec.pairs() {
            if let Some(hsv) = parse_hsv(value) {
                by_name.insert(name.to_ascii_uppercase(), hsv);
            }
        }
        Self { by_name }
    }

    /// 按方案名取 HSV（大小写不敏感）。
    pub fn get(&self, name: &str) -> Option<Hsv> {
        self.by_name.get(&name.to_ascii_uppercase()).copied()
    }

    /// 阵营节 `Color=` → HSV；中立等跳过。
    pub fn hsv_for_house(&self, rules: &IniDocument, house: &str) -> Option<Hsv> {
        let up = house.to_ascii_uppercase();
        if matches!(up.as_str(), "NEUTRAL" | "SPECIAL" | "CIVILIAN") {
            return None;
        }
        let scheme = rules.get(house, "Color")?;
        self.get(scheme)
    }

    /// 阵营 HSV remap；无方案时回退 `Palette::for_owner`。
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
