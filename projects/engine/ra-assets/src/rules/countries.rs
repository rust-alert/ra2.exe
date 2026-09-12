//! rules `[Countries]` / `[Sides]`：国家与势力表（INI 字段解释，供大厅 / 装载使用）。

use std::fmt;

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView};
use ra_types::HouseAllowList;
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

/// 一个国家（house）定义。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
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
    /// `File.LoadScreen=` 装载背景 SHP（完整文件名）；空串由 edition adaptor 补。
    pub load_screen: String,
    /// `File.LoadScreenPAL=` 装载调色板；空串由 edition adaptor 补。
    pub load_screen_pal: String,
    /// `File.Flag=` 旗标 PCX；空串由 edition adaptor 补。
    pub flag: String,
    /// `LoadScreenText.Brief=` 装载介绍 CSF 键（可含 `LOADBRIEF:` / `STT:` 前缀）；空串由 adaptor 补。
    pub load_brief: String,
}

impl CountryDef {
    /// 是否出现在离线遭遇战国家下拉。
    pub fn visible_in_skirmish(&self) -> bool {
        self.multiplay && !self.multiplay_obsolete
    }
}

/// 一条 `[Sides]` 势力分组。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct SideGroup {
    /// 势力 id（节内键，如 `GDI` / `Nod` / `ThirdSide`）。
    pub id: String,
    /// 成员国家 id（保序）。
    pub countries: Vec<String>,
}

/// 势力壳层 chrome（Side 段键；任意势力 id，不限制阵营数量）。
///
/// 只解析 rules 显式键。缺 `MixFileIndex` / 装载艺术时由 **edition adaptor** 填库存映射，
/// 内核不猜苏盟二元、不按国名回退。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct SideChromeDef {
    /// 势力 id（与 [`SideGroup::id`] 一致）。
    pub id: String,
    /// `Sidebar.MixFileIndex`（1-based → `sidecNN`）；缺省为 `None`（交 adaptor）。
    pub mix_file_index: Option<u32>,
    /// `Sidebar.YuriFileNames`；缺键为 `false`。
    pub yuri_file_names: bool,
    /// `MultiplayerScore.Background`。
    pub score_background: Option<String>,
    /// `MultiplayerScore.Palette`。
    pub score_palette: Option<String>,
    /// `EVA.Tag`（可空；音频采样键优先用此标签）。
    pub eva_tag: Option<String>,
    /// 结算统计区是否叠半透明黑底；`None` 交 adaptor 填。
    pub score_stats_shade: Option<bool>,
}

/// rules 派生的国家 / 势力注册表。
#[derive(Debug, Clone, Default)]
#[doc(hidden)]
pub struct CountryRegistry {
    countries: Vec<CountryDef>,
    sides: Vec<SideGroup>,
    side_chromes: Vec<SideChromeDef>,
}

impl CountryRegistry {
    /// 从 rules 文档解析；缺节则空表。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(rules);
        Self::from_layered(LayeredIniView::new(docs, &policy))
    }

    /// 从层叠 rules 视图解析国家 / 势力表。
    pub fn from_layered(view: LayeredIniView<'_>) -> Self {
        let mut countries = parse_countries(view);
        for c in &mut countries {
            c.special_ui_name = resolve_country_special_ui_name_layered(view, &c.id);
        }
        let sides = parse_sides(view);
        let side_chromes = parse_side_chromes(view, &sides);
        Self { countries, sides, side_chromes }
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
        self.side_chromes.iter().find(|c| c.id.eq_ignore_ascii_case(side_id))
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

#[doc(hidden)]
pub fn parse_countries(view: LayeredIniView<'_>) -> Vec<CountryDef> {
    let Some(list) = view.section("Countries")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (list_index, resolved) in list.numbered_resolved() {
        let id = resolved.value.trimmed().raw;
        if id.is_empty() {
            continue;
        }
        let id_key = id.to_ascii_uppercase();
        if !seen.insert(id_key) {
            continue;
        }
        out.push(parse_country(view, list_index, id));
    }
    out
}

#[doc(hidden)]
pub fn parse_country(view: LayeredIniView<'_>, list_index: u32, id: &str) -> CountryDef {
    let fields = view
        .section(id)
        .and_then(|s| s.deserialize::<CountrySectionFields>().ok())
        .unwrap_or_default();
    CountryDef {
        id: id.to_string(),
        list_index,
        ui_name: fields.ui_name.unwrap_or_default().trim().to_string(),
        prefix: fields.prefix.unwrap_or_default().trim().to_string(),
        color: fields.color.unwrap_or_default().trim().to_string(),
        side: fields.side.unwrap_or_default().trim().to_string(),
        multiplay: fields.multiplay.unwrap_or(false),
        multiplay_obsolete: fields.multiplay_obsolete.unwrap_or(false),
        special_ui_name: String::new(),
        load_screen: fields.load_screen.unwrap_or_default().trim().to_string(),
        load_screen_pal: fields.load_screen_pal.unwrap_or_default().trim().to_string(),
        flag: fields.flag.unwrap_or_default().trim().to_string(),
        load_brief: fields.load_brief.unwrap_or_default().trim().to_string(),
    }
}

/// 国家节字段（一次 Serde）。
#[derive(Debug, Default, Deserialize)]
struct CountrySectionFields {
    #[serde(rename = "UIName")]
    ui_name: Option<String>,
    #[serde(rename = "Prefix")]
    prefix: Option<String>,
    #[serde(rename = "Color")]
    color: Option<String>,
    #[serde(rename = "Side")]
    side: Option<String>,
    #[serde(rename = "Multiplay")]
    multiplay: Option<bool>,
    #[serde(rename = "MultiplayObsolete")]
    multiplay_obsolete: Option<bool>,
    #[serde(rename = "File.LoadScreen")]
    load_screen: Option<String>,
    #[serde(rename = "File.LoadScreenPAL")]
    load_screen_pal: Option<String>,
    #[serde(rename = "File.Flag")]
    flag: Option<String>,
    #[serde(rename = "LoadScreenText.Brief")]
    load_brief: Option<String>,
}

/// 解析该国装载页特色兵种 CSF 键（`RequiredHouses` → 类型/`SuperWeapon` 的 `UIName`）。
///
/// 扫描顺序：步兵 → 飞行器 → 载具 → 建筑（与常见「特色兵种」优先级一致；同国多条时取先命中）。
/// 未命中返回空串：调用方不得回退到写死表，装载页不画特色名即可。
pub fn resolve_country_special_ui_name(rules: &IniDocument, country_id: &str) -> String {
    let policy = IniMergePolicy::last_wins();
    let docs = std::slice::from_ref(rules);
    resolve_country_special_ui_name_layered(LayeredIniView::new(docs, &policy), country_id)
}

fn resolve_country_special_ui_name_layered(view: LayeredIniView<'_>, country_id: &str) -> String {
    for list in ["InfantryTypes", "AircraftTypes", "VehicleTypes", "BuildingTypes"] {
        let Some(sec) = view.section(list)
        else {
            continue;
        };
        for key in sec.keys() {
            let Some(type_val) = sec.get(key)
            else {
                continue;
            };
            let type_id = type_val.trimmed().raw;
            if type_id.is_empty() {
                continue;
            }
            let Some(techno) = view.section(type_id)
            else {
                continue;
            };
            let required = techno.get("RequiredHouses").map(|v| v.raw).unwrap_or("");
            if !required_houses_is_exactly(required, country_id) {
                continue;
            }
            // 建筑特色常是「空指部挂空降」：优先超武 UIName，避免画出建筑名。
            if list == "BuildingTypes" {
                if let Some(sw) = techno.get("SuperWeapon").map(|v| v.trimmed().raw).filter(|s| !s.is_empty()) {
                    if let Some(sw_ui) = view
                        .get(sw, "UIName")
                        .map(|v| v.trimmed().raw.to_string())
                        .filter(|s| !s.is_empty())
                    {
                        return sw_ui;
                    }
                }
            }
            if let Some(ui) = techno.get("UIName").map(|v| v.trimmed().raw.to_string()).filter(|s| !s.is_empty()) {
                return ui;
            }
        }
    }
    String::new()
}

#[doc(hidden)]
pub fn required_houses_is_exactly(raw: &str, country_id: &str) -> bool {
    let list = HouseAllowList::parse_csv(raw);
    list.len() == 1 && list.required_allows(country_id)
}

#[doc(hidden)]
pub fn parse_sides(view: LayeredIniView<'_>) -> Vec<SideGroup> {
    let Some(sec) = view.section("Sides")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in sec.keys() {
        let id = key.trim();
        if id.is_empty() {
            continue;
        }
        let Some(value) = sec.get(key)
        else {
            continue;
        };
        let countries: Vec<String> = crate::from_row::<Vec<String>>(value.raw)
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        out.push(SideGroup { id: id.to_string(), countries });
    }
    out
}

#[doc(hidden)]
pub fn parse_side_chromes(view: LayeredIniView<'_>, sides: &[SideGroup]) -> Vec<SideChromeDef> {
    let mut out = Vec::with_capacity(sides.len());
    for group in sides {
        let fields = view
            .section(&group.id)
            .and_then(|s| s.deserialize::<SideChromeSectionFields>().ok())
            .unwrap_or_default();
        out.push(SideChromeDef {
            id: group.id.clone(),
            mix_file_index: fields.mix_file_index,
            yuri_file_names: fields.yuri_file_names.unwrap_or(false),
            score_background: fields.score_background.filter(|s| !s.is_empty()),
            score_palette: fields.score_palette.filter(|s| !s.is_empty()),
            eva_tag: fields.eva_tag.filter(|s| !s.is_empty()),
            // rules 无独立键；由 edition adaptor stock 填。
            score_stats_shade: None,
        });
    }
    out
}

/// 势力壳层 chrome 节字段（一次 Serde）。
#[derive(Debug, Default, Deserialize)]
struct SideChromeSectionFields {
    #[serde(rename = "Sidebar.MixFileIndex", default, deserialize_with = "deserialize_optional_mix_file_index")]
    mix_file_index: Option<u32>,
    #[serde(rename = "Sidebar.YuriFileNames")]
    yuri_file_names: Option<bool>,
    #[serde(rename = "MultiplayerScore.Background")]
    score_background: Option<String>,
    #[serde(rename = "MultiplayerScore.Palette")]
    score_palette: Option<String>,
    #[serde(rename = "EVA.Tag")]
    eva_tag: Option<String>,
}

fn deserialize_optional_mix_file_index<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct MixIndexVisitor;

    impl<'de> Visitor<'de> for MixIndexVisitor {
        type Value = Option<u32>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("Sidebar.MixFileIndex >= 1")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(if v >= 1 { Some(v as u32) } else { None })
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            if v < 1 {
                return Ok(None);
            }
            Ok(Some(v as u32))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let Ok(n) = v.trim().parse::<u32>()
            else {
                return Ok(None);
            };
            Ok(if n >= 1 { Some(n) } else { None })
        }
    }

    deserializer.deserialize_any(MixIndexVisitor)
}

/// 仅填空：把 edition adaptor 提供的库存 Side chrome 写入缺键行。
pub fn fill_side_chrome_gaps(chromes: &mut [SideChromeDef], stock: &[SideChromeDef]) {
    for src in stock {
        let Some(dst) = chromes.iter_mut().find(|c| c.id.eq_ignore_ascii_case(&src.id))
        else {
            continue;
        };
        let had_index = dst.mix_file_index.is_some();
        if !had_index {
            dst.mix_file_index = src.mix_file_index;
            dst.yuri_file_names = src.yuri_file_names;
        }
        if dst.score_background.is_none() {
            dst.score_background = src.score_background.clone();
        }
        if dst.score_palette.is_none() {
            dst.score_palette = src.score_palette.clone();
        }
        if dst.eva_tag.is_none() {
            dst.eva_tag = src.eva_tag.clone();
        }
        if dst.score_stats_shade.is_none() {
            dst.score_stats_shade = src.score_stats_shade;
        }
    }
}

/// 仅填空：库存国家装载 / 旗 / 介绍键。
pub fn fill_country_ui_gaps(countries: &mut [CountryDef], stock: &[CountryDef]) {
    for src in stock {
        let Some(dst) = countries.iter_mut().find(|c| c.id.eq_ignore_ascii_case(&src.id))
        else {
            continue;
        };
        if dst.load_screen.is_empty() && !src.load_screen.is_empty() {
            dst.load_screen = src.load_screen.clone();
        }
        if dst.load_screen_pal.is_empty() && !src.load_screen_pal.is_empty() {
            dst.load_screen_pal = src.load_screen_pal.clone();
        }
        if dst.flag.is_empty() && !src.flag.is_empty() {
            dst.flag = src.flag.clone();
        }
        if dst.load_brief.is_empty() && !src.load_brief.is_empty() {
            dst.load_brief = src.load_brief.clone();
        }
    }
}
