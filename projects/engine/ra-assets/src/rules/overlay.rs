//! 从 `rules.ini` 的 `[OverlayTypes]` 建立 id → 名称表，并标记可采矿格。

use crate::ini::IniDocument;

/// Overlay 类型注册表。
///
/// 内部 id 按 `[OverlayTypes]` **声明顺序**（值序列）编号，不按数字键留空洞。
/// 零售 `rules.ini` 常缺 `0=` / `40=` 等键；若按键号建表，矿/宝石会错位到桥/墙。
#[derive(Debug, Clone, Default)]
pub struct OverlayTypeRegistry {
    /// `overlay_id` → 类型名（大写）；下标即 OverlayPack 字节。
    names: Vec<String>,
    /// 与 `names` 对齐：该 id 是否可采（矿/宝石）。
    harvestable: Vec<bool>,
}

impl OverlayTypeRegistry {
    /// 解析 `[OverlayTypes]`：按节内条目顺序赋 id `0..n`，忽略键的数字字面量。
    ///
    /// 可采判定优先读类型节 `Tiberium=yes` / `SpawnsTiberium=yes` / `Land=`，
    /// 再回退类型名前缀 `TIB*` / `GEM*`。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let Some(section) = rules.section("OverlayTypes")
        else {
            return Self::default();
        };
        let mut names = Vec::new();
        let mut harvestable = Vec::new();
        for (_key, value) in section.pairs() {
            let name = value.trim();
            if name.is_empty() {
                continue;
            }
            let name_up = name.to_ascii_uppercase();
            let can_harvest = overlay_type_is_harvestable(rules, &name_up);
            names.push(name_up);
            harvestable.push(can_harvest);
        }
        Self { names, harvestable }
    }

    /// 已登记的类型数量。
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// 是否没有任何类型。
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// 按 overlay id 取类型名。
    pub fn name(&self, id: u8) -> Option<&str> {
        self.names.get(usize::from(id)).map(String::as_str)
    }

    /// 该 overlay id 是否可采矿/宝石。
    pub fn is_harvestable(&self, id: u8) -> bool {
        self.harvestable.get(usize::from(id)).copied().unwrap_or(false)
    }
}

/// 类型名是否像矿/宝石（无规则节时的回退）。
pub fn harvestable_overlay_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.starts_with("TIB") || upper.starts_with("GEM")
}

fn overlay_type_is_harvestable(rules: &IniDocument, name: &str) -> bool {
    if rules
        .get(name, "Tiberium")
        .is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "1"))
    {
        return true;
    }
    if rules
        .get(name, "SpawnsTiberium")
        .is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "1"))
    {
        return true;
    }
    if rules
        .get(name, "Land")
        .is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "tiberium" | "ore" | "gems"))
    {
        return true;
    }
    harvestable_overlay_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ini::IniDocument;

    #[test]
    fn marks_tiberium_flag_and_name_prefix() {
        let doc = IniDocument::parse(
            br#"
[OverlayTypes]
0=TIB01
1=BRIDGE1
2=GEM01
3=WALL1

[TIB01]
Tiberium=yes

[BRIDGE1]
Land=Road

[GEM01]
Land=Gems

[WALL1]
Land=Wall
"#,
        )
        .expect("ini");
        let reg = OverlayTypeRegistry::from_rules(&doc);
        assert_eq!(reg.name(0), Some("TIB01"));
        assert!(reg.is_harvestable(0));
        assert!(!reg.is_harvestable(1));
        assert!(reg.is_harvestable(2));
        assert!(!reg.is_harvestable(3));
    }
}
