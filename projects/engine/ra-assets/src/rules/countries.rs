//! rules `[Countries]` / `[Sides]`：国家与势力表（INI 字段解释，供大厅 / 装载使用）。

use crate::ini::IniDocument;

/// 一个国家（house）定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountryDef {
    /// 节名 / house id（如 `Americans`）。
    pub id: String,
    /// `[Countries]` 列表下标。
    pub list_index: u32,
    /// `UIName=` CSF 键（如 `Name:Americans`）；缺省为空。
    pub ui_name: String,
    /// `Prefix=`（旗标 / 装载艺术常用前缀，如 `USA`）；缺省为空。
    pub prefix: String,
    /// `Color=` 方案名（如 `Gold`）；缺省为空。
    pub color: String,
    /// `Side=` 势力 id（如 `GDI` / `Nod` / `ThirdSide`）；缺省为空。
    pub side: String,
    /// `Multiplay=` 是否可在多人 / 遭遇战选用。
    pub multiplay: bool,
    /// `MultiplayObsolete=` 是否从多人表中废弃。
    pub multiplay_obsolete: bool,
}

impl CountryDef {
    /// 是否出现在离线遭遇战国家下拉。
    pub fn visible_in_skirmish(&self) -> bool {
        self.multiplay && !self.multiplay_obsolete
    }
}

/// 一条 `[Sides]` 势力分组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideGroup {
    /// 势力 id（节内键，如 `GDI` / `Nod` / `ThirdSide`）。
    pub id: String,
    /// 成员国家 id（保序）。
    pub countries: Vec<String>,
}

/// rules 派生的国家 / 势力注册表。
#[derive(Debug, Clone, Default)]
pub struct CountryRegistry {
    countries: Vec<CountryDef>,
    sides: Vec<SideGroup>,
}

impl CountryRegistry {
    /// 从 rules 文档解析；缺节则空表。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let countries = parse_countries(rules);
        let sides = parse_sides(rules);
        Self { countries, sides }
    }

    /// 全部国家（`[Countries]` 列表序）。
    pub fn countries(&self) -> &[CountryDef] {
        &self.countries
    }

    /// 全部势力分组（`[Sides]` 键序）。
    pub fn sides(&self) -> &[SideGroup] {
        &self.sides
    }

    /// 遭遇战可选国家（`Multiplay` 且非 `MultiplayObsolete`）。
    pub fn skirmish_countries(&self) -> Vec<&CountryDef> {
        self.countries.iter().filter(|c| c.visible_in_skirmish()).collect()
    }

    /// 按 id 查找（大小写不敏感）。
    pub fn get(&self, id: &str) -> Option<&CountryDef> {
        let key = id.to_ascii_uppercase();
        self.countries.iter().find(|c| c.id.eq_ignore_ascii_case(&key))
    }

    /// 已解析国家数。
    pub fn len(&self) -> usize {
        self.countries.len()
    }

    /// 是否为空表。
    pub fn is_empty(&self) -> bool {
        self.countries.is_empty()
    }
}

fn parse_countries(rules: &IniDocument) -> Vec<CountryDef> {
    let Some(list) = rules.section("Countries")
    else {
        return Vec::new();
    };
    let mut indexed: Vec<(u32, String)> = Vec::new();
    for (key, value) in list.pairs() {
        let Ok(n) = key.trim().parse::<u32>()
        else {
            continue;
        };
        let id = value.trim();
        if id.is_empty() {
            continue;
        }
        indexed.push((n, id.to_string()));
    }
    indexed.sort_by_key(|(n, _)| *n);
    let mut out = Vec::with_capacity(indexed.len());
    let mut seen = std::collections::HashSet::new();
    for (list_index, id) in indexed {
        let id_key = id.to_ascii_uppercase();
        if !seen.insert(id_key) {
            continue;
        }
        out.push(parse_country(rules, list_index, &id));
    }
    out
}

fn parse_country(rules: &IniDocument, list_index: u32, id: &str) -> CountryDef {
    let sec = rules.section(id);
    let get = |key: &str| sec.and_then(|s| s.get(key)).unwrap_or("").trim().to_string();
    let multiplay = sec.and_then(|s| s.get("Multiplay")).map(parse_ini_bool_loose).unwrap_or(false);
    let multiplay_obsolete = sec
        .and_then(|s| s.get("MultiplayObsolete"))
        .map(parse_ini_bool_loose)
        .unwrap_or(false);
    CountryDef {
        id: id.to_string(),
        list_index,
        ui_name: get("UIName"),
        prefix: get("Prefix"),
        color: get("Color"),
        side: get("Side"),
        multiplay,
        multiplay_obsolete,
    }
}

fn parse_sides(rules: &IniDocument) -> Vec<SideGroup> {
    let Some(sec) = rules.section("Sides")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in sec.pairs() {
        let id = key.trim();
        if id.is_empty() {
            continue;
        }
        let countries: Vec<String> = value
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        out.push(SideGroup {
            id: id.to_string(),
            countries,
        });
    }
    out
}

fn parse_ini_bool_loose(raw: &str) -> bool {
    matches!(raw.trim().to_ascii_lowercase().as_str(), "true" | "yes" | "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[Countries]
0=Americans
1=French
2=Russians
3=Unused
4=YuriCountry

[Americans]
UIName=Name:Americans
Prefix=USA
Color=Gold
Side=GDI
Multiplay=yes

[French]
UIName=Name:French
Prefix=FRA
Color=LightBlue
Side=GDI
Multiplay=yes

[Russians]
UIName=Name:Russians
Prefix=RUS
Color=DarkRed
Side=Nod
Multiplay=yes

[Unused]
Multiplay=no

[YuriCountry]
UIName=Name:Yuri
Prefix=YUR
Color=Purple
Side=ThirdSide
Multiplay=yes
MultiplayObsolete=no

[Sides]
GDI=Americans,French
Nod=Russians
ThirdSide=YuriCountry
Civilian=Neutral
"#;

    #[test]
    fn parses_countries_and_sides_in_list_order() {
        let doc = IniDocument::parse(SAMPLE.as_bytes()).unwrap();
        let reg = CountryRegistry::from_rules(&doc);
        assert_eq!(reg.len(), 5);
        assert_eq!(reg.countries()[0].id, "Americans");
        assert_eq!(reg.countries()[0].prefix, "USA");
        assert_eq!(reg.countries()[0].side, "GDI");
        assert!(reg.countries()[0].multiplay);
        assert!(!reg.countries()[3].multiplay);
        assert_eq!(reg.sides().len(), 4);
        assert_eq!(reg.sides()[0].id, "GDI");
        assert_eq!(reg.sides()[0].countries, vec!["Americans", "French"]);
    }

    #[test]
    fn skirmish_filter_drops_non_multiplay() {
        let doc = IniDocument::parse(SAMPLE.as_bytes()).unwrap();
        let reg = CountryRegistry::from_rules(&doc);
        let ids: Vec<_> = reg.skirmish_countries().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["Americans", "French", "Russians", "YuriCountry"]);
    }
}
