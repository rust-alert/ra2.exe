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
    /// 装载页特色兵种 CSF 键（资源链派生，非国家→兵种写死表）。
    ///
    /// 取自 rules：`RequiredHouses` 恰为本国的类型之 `UIName`；建筑若挂
    /// `SuperWeapon=` 则改用该超武的 `UIName`（如美军空降）。
    /// 空串表示无特色可画——原版/模组均允许缺失，装载页应跳过该行。
    pub special_ui_name: String,
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

/// 势力壳层 chrome（Side 段键；任意势力 id，不限制阵营数量）。
///
/// 来自 rules 中与 `[Sides]` 键同名的节，例如 `Sidebar.MixFileIndex`、
/// `MultiplayerScore.Background`。缺键时对应字段为空 / 默认，由上层用库存启发式补全。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideChromeDef {
    /// 势力 id（与 [`SideGroup::id`] 一致）。
    pub id: String,
    /// `Sidebar.MixFileIndex`（1-based → `sidecNN`）；缺省为 `None`。
    pub mix_file_index: Option<u32>,
    /// `Sidebar.YuriFileNames`。
    pub yuri_file_names: bool,
    /// `MultiplayerScore.Background`。
    pub score_background: Option<String>,
    /// `MultiplayerScore.Palette`。
    pub score_palette: Option<String>,
}

/// rules 派生的国家 / 势力注册表。
#[derive(Debug, Clone, Default)]
pub struct CountryRegistry {
    countries: Vec<CountryDef>,
    sides: Vec<SideGroup>,
    side_chromes: Vec<SideChromeDef>,
}

impl CountryRegistry {
    /// 从 rules 文档解析；缺节则空表。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let mut countries = parse_countries(rules);
        for c in &mut countries {
            c.special_ui_name = resolve_country_special_ui_name(rules, &c.id);
        }
        let sides = parse_sides(rules);
        let side_chromes = parse_side_chromes(rules, &sides);
        Self {
            countries,
            sides,
            side_chromes,
        }
    }

    /// 全部国家（`[Countries]` 列表序）。
    pub fn countries(&self) -> &[CountryDef] {
        &self.countries
    }

    /// 全部势力分组（`[Sides]` 键序）。
    pub fn sides(&self) -> &[SideGroup] {
        &self.sides
    }

    /// 全部势力壳层 chrome（与 `[Sides]` 键对齐；节缺失时仍有占位行）。
    pub fn side_chromes(&self) -> &[SideChromeDef] {
        &self.side_chromes
    }

    /// 按势力 id 查找 chrome（大小写不敏感）。
    pub fn side_chrome(&self, side_id: &str) -> Option<&SideChromeDef> {
        self.side_chromes
            .iter()
            .find(|c| c.id.eq_ignore_ascii_case(side_id))
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
        special_ui_name: String::new(),
    }
}

/// 解析该国装载页特色兵种 CSF 键（`RequiredHouses` → 类型/`SuperWeapon` 的 `UIName`）。
///
/// 扫描顺序：步兵 → 飞行器 → 载具 → 建筑（与常见「特色兵种」优先级一致；同国多条时取先命中）。
/// 未命中返回空串：调用方不得回退到写死表，装载页不画特色名即可。
pub fn resolve_country_special_ui_name(rules: &IniDocument, country_id: &str) -> String {
    for list in ["InfantryTypes", "AircraftTypes", "VehicleTypes", "BuildingTypes"] {
        let Some(sec) = rules.section(list)
        else {
            continue;
        };
        for (_key, type_id) in sec.pairs() {
            let type_id = type_id.trim();
            if type_id.is_empty() {
                continue;
            }
            let Some(techno) = rules.section(type_id)
            else {
                continue;
            };
            if !required_houses_is_exactly(techno.get("RequiredHouses").unwrap_or(""), country_id) {
                continue;
            }
            // 建筑特色常是「空指部挂空降」：优先超武 UIName，避免画出建筑名。
            if list == "BuildingTypes" {
                if let Some(sw) = techno.get("SuperWeapon").map(str::trim).filter(|s| !s.is_empty()) {
                    if let Some(sw_ui) = rules
                        .section(sw)
                        .and_then(|s| s.get("UIName"))
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                    {
                        return sw_ui.to_string();
                    }
                }
            }
            if let Some(ui) = techno.get("UIName").map(str::trim).filter(|s| !s.is_empty()) {
                return ui.to_string();
            }
        }
    }
    String::new()
}

fn required_houses_is_exactly(raw: &str, country_id: &str) -> bool {
    let houses: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    houses.len() == 1 && houses[0].eq_ignore_ascii_case(country_id)
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

fn parse_side_chromes(rules: &IniDocument, sides: &[SideGroup]) -> Vec<SideChromeDef> {
    let mut out = Vec::with_capacity(sides.len());
    for group in sides {
        let sec = rules.section(&group.id);
        let mix_file_index = sec
            .and_then(|s| s.get("Sidebar.MixFileIndex"))
            .and_then(|v| v.trim().parse::<u32>().ok())
            .filter(|n| *n >= 1);
        let yuri_file_names = sec
            .and_then(|s| s.get("Sidebar.YuriFileNames"))
            .map(parse_ini_bool_loose)
            .unwrap_or(false);
        let score_background = sec
            .and_then(|s| s.get("MultiplayerScore.Background"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let score_palette = sec
            .and_then(|s| s.get("MultiplayerScore.Palette"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        out.push(SideChromeDef {
            id: group.id.clone(),
            mix_file_index,
            yuri_file_names,
            score_background,
            score_palette,
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

[GDI]
Sidebar.MixFileIndex=1
Sidebar.YuriFileNames=yes

[Nod]
Sidebar.MixFileIndex=2

[FourthSide]
Sidebar.MixFileIndex=4
Sidebar.YuriFileNames=yes
MultiplayerScore.Background=mpfscrnl.shp
MultiplayerScore.Palette=mpsscrnlf.pal
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
        let gdi = reg.side_chrome("GDI").unwrap();
        assert_eq!(gdi.mix_file_index, Some(1));
        assert!(gdi.yuri_file_names);
        let nod = reg.side_chrome("Nod").unwrap();
        assert_eq!(nod.mix_file_index, Some(2));
        assert!(!nod.yuri_file_names);
        // ThirdSide 无独立节：仍有占位 chrome，键为空。
        let third = reg.side_chrome("ThirdSide").unwrap();
        assert_eq!(third.mix_file_index, None);
        assert!(!third.yuri_file_names);
    }

    #[test]
    fn parses_open_side_chrome_score_overrides() {
        const RULES: &str = r#"
[Countries]
0=Guild1

[Guild1]
Side=FifthSide
Multiplay=yes

[Sides]
GDI=Americans
FifthSide=Guild1

[FifthSide]
Sidebar.MixFileIndex=5
Sidebar.YuriFileNames=no
MultiplayerScore.Background=mpxscrnl.shp
MultiplayerScore.Palette=mpxscrn.pal
"#;
        let doc = IniDocument::parse(RULES.as_bytes()).unwrap();
        let reg = CountryRegistry::from_rules(&doc);
        let fifth = reg.side_chrome("FifthSide").unwrap();
        assert_eq!(fifth.mix_file_index, Some(5));
        assert!(!fifth.yuri_file_names);
        assert_eq!(fifth.score_background.as_deref(), Some("mpxscrnl.shp"));
        assert_eq!(fifth.score_palette.as_deref(), Some("mpxscrn.pal"));
    }

    #[test]
    fn skirmish_filter_drops_non_multiplay() {
        let doc = IniDocument::parse(SAMPLE.as_bytes()).unwrap();
        let reg = CountryRegistry::from_rules(&doc);
        let ids: Vec<_> = reg.skirmish_countries().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["Americans", "French", "Russians", "YuriCountry"]);
    }

    #[test]
    fn special_ui_name_from_required_houses_and_superweapon() {
        const RULES: &str = r#"
[Countries]
0=Americans
1=Confederation

[Americans]
Multiplay=yes
[Confederation]
Multiplay=yes

[InfantryTypes]
0=TERROR
1=E1

[TERROR]
UIName=Name:TERROR
RequiredHouses=Confederation

[E1]
UIName=Name:E1

[BuildingTypes]
0=GAPILE

[GAPILE]
UIName=Name:GAPILE
RequiredHouses=Americans
SuperWeapon=ParaDrop

[SuperWeaponTypes]
0=ParaDrop

[ParaDrop]
UIName=Name:PARA
"#;
        let doc = IniDocument::parse(RULES.as_bytes()).unwrap();
        let reg = CountryRegistry::from_rules(&doc);
        assert_eq!(reg.get("Confederation").unwrap().special_ui_name, "Name:TERROR");
        assert_eq!(reg.get("Americans").unwrap().special_ui_name, "Name:PARA");
    }
}
