//! 从 rules 列表节解析 TechnoType 注册表（节字段经 Serde 一次解码）。

use std::collections::HashMap;

use serde::Deserialize;

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView};

/// 步兵 / 载具 / 飞行器 / 建筑的共用类型字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnoType {
    /// 类型 id（大写）。
    pub id: String,
    /// 所属大类。
    pub kind: TechnoKind,
    /// `Strength` 生命值。
    pub strength: u32,
    /// `Armor` 护甲名。
    pub armor: String,
    /// `Speed` 移动速度。
    pub speed: u32,
    /// `Sight` 视野。
    pub sight: u32,
    /// `Cost` 造价。
    pub cost: u32,
    /// `TechLevel`；缺省为 -1。
    pub tech_level: i32,
    /// `Owner` 所属阵营串。
    pub owner: String,
    /// `Image` 资源名（缺省等于 id）。
    pub image: String,
    /// `Category`（如 `Soldier` / `Dog`）；空表示未写。
    pub category: String,
    /// `Naval=yes`。
    pub naval: bool,
    /// `Agent=yes`（可渗透敌方建筑的间谍类单位）。
    pub agent: bool,
    /// `Engineer=yes`（可占领敌方可俘建筑）。
    pub engineer: bool,
    /// `Harvester=yes`（采矿车）。
    pub harvester: bool,
    /// 主武器名（`Primary`）；空表示未配置。
    pub primary: String,
    /// 主武器伤害（来自武器节 `Damage`）；0 表示未配置。
    pub damage: u32,
    /// 主武器射程（来自武器节 `Range`，格）；0 表示未配置。
    pub range: u32,
    /// 射速间隔（tick）；优先武器节 `ROF`，否则类型节；0 表示未配置。
    pub rof: u32,
    /// 主武器弹头名（武器节 `Warhead`）；空表示未配置。
    pub warhead: String,
    /// `Prerequisite` token（大写）。
    pub prerequisite: Vec<String>,
    /// `PrerequisiteOverride` token（大写）。
    pub prerequisite_override: Vec<String>,
    /// `RequiredHouses` token（大写）。
    pub required_houses: Vec<String>,
    /// `ForbiddenHouses` token（大写）。
    pub forbidden_houses: Vec<String>,
    /// `BuildLimit`；`0` 表示不限。
    pub build_limit: i32,
    /// `BuildTime`；`0` 表示缺省。
    pub build_time: u32,
    /// `RequiresStolenAlliedTech`。
    pub requires_stolen_allied_tech: bool,
    /// `RequiresStolenSovietTech`。
    pub requires_stolen_soviet_tech: bool,
    /// `RequiresStolenThirdTech`。
    pub requires_stolen_third_tech: bool,
    /// `PixelSelectionBracketDelta`。
    pub pixel_selection_bracket_delta: i32,
    /// `DeploysInto` 目标类型键（大写）；空表示无。
    pub deploys_into: String,
    /// `Power` 原始值（正产电、负耗电）。
    pub power: i32,
    /// `Powered`；缺省时由耗电推导。
    pub powered: Option<bool>,
    /// `ConstructionYard`。
    pub construction_yard: bool,
    /// `Refinery`。
    pub refinery: bool,
    /// `Radar`。
    pub radar: bool,
    /// `BuildCat` 原文。
    pub build_cat: String,
    /// `Capturable`。
    pub capturable: bool,
    /// `Factory` 原文。
    pub factory: String,
    /// `SuperWeapon` 键（大写）；空表示无。
    pub super_weapon: String,
    /// `Foundation` 原文（优先 art，否则 rules）；空表示未写。
    pub foundation: String,
    /// `Height`（优先 art，否则 rules）；`None` 表示未写。
    pub height: Option<u16>,
}

/// Techno 大类，对应 rules 列表节。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TechnoKind {
    /// `[InfantryTypes]`
    Infantry,
    /// `[VehicleTypes]`
    Vehicle,
    /// `[AircraftTypes]`
    Aircraft,
    /// `[BuildingTypes]`
    Building,
}

/// `type_id` → 解析后的 TechnoType。
#[derive(Debug, Clone, Default)]
pub struct TechnoTypeRegistry {
    by_id: HashMap<String, TechnoType>,
}

impl TechnoTypeRegistry {
    /// 扫描 `[InfantryTypes]` / `[VehicleTypes]` / `[AircraftTypes]` / `[BuildingTypes]`。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(rules);
        Self::from_layered(LayeredIniView::new(docs, &policy))
    }

    /// 从层叠 rules 视图扫描列表节并解码各类型（字段按视图策略合并）。
    pub fn from_layered(view: LayeredIniView<'_>) -> Self {
        let mut by_id = HashMap::new();
        for (section, kind) in [
            ("InfantryTypes", TechnoKind::Infantry),
            ("VehicleTypes", TechnoKind::Vehicle),
            ("AircraftTypes", TechnoKind::Aircraft),
            ("BuildingTypes", TechnoKind::Building),
        ] {
            let Some(list) = view.section(section)
            else {
                continue;
            };
            for key in list.keys() {
                let Some(name_val) = list.get(key)
                else {
                    continue;
                };
                let id = name_val.trimmed().raw;
                if id.is_empty() {
                    continue;
                }
                let id_up = id.to_ascii_uppercase();
                if by_id.contains_key(&id_up) {
                    continue;
                }
                if let Some(tt) = parse_techno(view, &id_up, kind) {
                    by_id.insert(id_up, tt);
                }
            }
        }
        Self { by_id }
    }

    /// 按 id 查找（大小写不敏感）。
    pub fn get(&self, id: &str) -> Option<&TechnoType> {
        self.by_id.get(&id.to_ascii_uppercase())
    }

    /// 已解析类型总数。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// 是否为空表。
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// 统计某一大类的数量。
    pub fn count_kind(&self, kind: TechnoKind) -> usize {
        self.by_id.values().filter(|t| t.kind == kind).count()
    }

    /// 遍历已解析类型。
    pub fn iter(&self) -> impl Iterator<Item = &TechnoType> {
        self.by_id.values()
    }

    /// 用 art（含 `Image=` 跳转）覆盖建筑的 `Foundation` / `Height`，缺键保留 rules。
    pub fn apply_art_geometry(&mut self, art: &IniDocument) {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(art);
        self.apply_art_geometry_layered(LayeredIniView::new(docs, &policy));
    }

    /// 用层叠 art 覆盖建筑几何字段。
    pub fn apply_art_geometry_layered(&mut self, art: LayeredIniView<'_>) {
        for tt in self.by_id.values_mut() {
            if tt.kind != TechnoKind::Building {
                continue;
            }
            if let Some(v) = art_geometry_string(art, &tt.id, "Foundation") {
                tt.foundation = v;
            }
            if let Some(v) = art_geometry_string(art, &tt.id, "Height") {
                if let Ok(h) = v.parse::<i32>() {
                    tt.height = Some(h.max(1) as u16);
                }
            }
        }
    }
}

/// 类型节字段（一次 Serde 解码；缺省与归一化在组装 `TechnoType` 时完成）。
#[derive(Debug, Deserialize)]
struct TechnoSectionFields {
    #[serde(rename = "Strength")]
    strength: Option<u32>,
    #[serde(rename = "Armor")]
    armor: Option<String>,
    #[serde(rename = "Speed")]
    speed: Option<u32>,
    #[serde(rename = "Sight")]
    sight: Option<u32>,
    #[serde(rename = "Cost")]
    cost: Option<u32>,
    #[serde(rename = "TechLevel")]
    tech_level: Option<i32>,
    #[serde(rename = "Owner")]
    owner: Option<String>,
    #[serde(rename = "Image")]
    image: Option<String>,
    #[serde(rename = "Category")]
    category: Option<String>,
    #[serde(rename = "Naval")]
    naval: Option<bool>,
    #[serde(rename = "Agent")]
    agent: Option<bool>,
    #[serde(rename = "Engineer")]
    engineer: Option<bool>,
    #[serde(rename = "Harvester")]
    harvester: Option<bool>,
    #[serde(rename = "Primary")]
    primary: Option<String>,
    #[serde(rename = "ROF")]
    rof: Option<u32>,
    #[serde(rename = "Prerequisite", default)]
    prerequisite: Vec<String>,
    #[serde(rename = "PrerequisiteOverride", default)]
    prerequisite_override: Vec<String>,
    #[serde(rename = "RequiredHouses", default)]
    required_houses: Vec<String>,
    #[serde(rename = "ForbiddenHouses", default)]
    forbidden_houses: Vec<String>,
    #[serde(rename = "BuildLimit")]
    build_limit: Option<i32>,
    #[serde(rename = "BuildTime")]
    build_time: Option<i32>,
    #[serde(rename = "RequiresStolenAlliedTech")]
    requires_stolen_allied_tech: Option<bool>,
    #[serde(rename = "RequiresStolenSovietTech")]
    requires_stolen_soviet_tech: Option<bool>,
    #[serde(rename = "RequiresStolenThirdTech")]
    requires_stolen_third_tech: Option<bool>,
    #[serde(rename = "PixelSelectionBracketDelta")]
    pixel_selection_bracket_delta: Option<i32>,
    #[serde(rename = "DeploysInto")]
    deploys_into: Option<String>,
    #[serde(rename = "Power")]
    power: Option<i32>,
    #[serde(rename = "Powered")]
    powered: Option<bool>,
    #[serde(rename = "ConstructionYard")]
    construction_yard: Option<bool>,
    #[serde(rename = "Refinery")]
    refinery: Option<bool>,
    #[serde(rename = "Radar")]
    radar: Option<bool>,
    #[serde(rename = "BuildCat")]
    build_cat: Option<String>,
    #[serde(rename = "Capturable")]
    capturable: Option<bool>,
    #[serde(rename = "Factory")]
    factory: Option<String>,
    #[serde(rename = "SuperWeapon")]
    super_weapon: Option<String>,
    #[serde(rename = "Foundation")]
    foundation: Option<String>,
    #[serde(rename = "Height")]
    height: Option<i32>,
}

/// 武器节字段。
#[derive(Debug, Deserialize)]
struct WeaponSectionFields {
    #[serde(rename = "Damage")]
    damage: Option<u32>,
    #[serde(rename = "Range")]
    range: Option<u32>,
    #[serde(rename = "ROF")]
    rof: Option<u32>,
    #[serde(rename = "Warhead")]
    warhead: Option<String>,
}

fn uppercase_tokens(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_techno(view: LayeredIniView<'_>, id: &str, kind: TechnoKind) -> Option<TechnoType> {
    let section = view.section(id)?;
    let fields: TechnoSectionFields = section.deserialize().ok()?;
    let primary = fields
        .primary
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    let techno_rof = fields.rof.unwrap_or(0);
    let (damage, range, rof, warhead) = resolve_primary_weapon(view, &primary, techno_rof);
    let image = fields
        .image
        .as_deref()
        .unwrap_or(id)
        .trim()
        .to_ascii_uppercase();
    Some(TechnoType {
        id: id.to_string(),
        kind,
        strength: fields.strength.unwrap_or(1),
        armor: fields.armor.unwrap_or_else(|| "none".into()),
        speed: fields.speed.unwrap_or(0),
        sight: fields.sight.unwrap_or(0),
        cost: fields.cost.unwrap_or(0),
        tech_level: fields.tech_level.unwrap_or(-1),
        owner: fields.owner.unwrap_or_default(),
        image,
        category: fields.category.unwrap_or_default().trim().to_string(),
        naval: fields.naval.unwrap_or(false),
        agent: fields.agent.unwrap_or(false),
        engineer: fields.engineer.unwrap_or(false),
        harvester: fields.harvester.unwrap_or(false),
        primary,
        damage,
        range,
        rof,
        warhead,
        prerequisite: uppercase_tokens(fields.prerequisite),
        prerequisite_override: uppercase_tokens(fields.prerequisite_override),
        required_houses: uppercase_tokens(fields.required_houses),
        forbidden_houses: uppercase_tokens(fields.forbidden_houses),
        build_limit: fields.build_limit.unwrap_or(0).max(0),
        build_time: fields.build_time.unwrap_or(0).max(0) as u32,
        requires_stolen_allied_tech: fields.requires_stolen_allied_tech.unwrap_or(false),
        requires_stolen_soviet_tech: fields.requires_stolen_soviet_tech.unwrap_or(false),
        requires_stolen_third_tech: fields.requires_stolen_third_tech.unwrap_or(false),
        pixel_selection_bracket_delta: fields.pixel_selection_bracket_delta.unwrap_or(0),
        deploys_into: fields
            .deploys_into
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_uppercase(),
        power: fields.power.unwrap_or(0),
        powered: fields.powered,
        construction_yard: fields.construction_yard.unwrap_or(false),
        refinery: fields.refinery.unwrap_or(false),
        radar: fields.radar.unwrap_or(false),
        build_cat: fields.build_cat.unwrap_or_default(),
        capturable: fields.capturable.unwrap_or(false),
        factory: fields.factory.unwrap_or_default(),
        super_weapon: fields
            .super_weapon
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_uppercase(),
        foundation: fields.foundation.unwrap_or_default().trim().to_string(),
        height: fields.height.map(|h| h.max(1) as u16),
    })
}

/// 先读 art 本节，再跟 `Image=` 指向的 art 节。
fn art_geometry_string(art: LayeredIniView<'_>, type_key: &str, key: &str) -> Option<String> {
    if let Some(v) = art.get(type_key, key) {
        let t = v.trimmed();
        if !t.raw.is_empty() {
            return Some(t.raw.to_string());
        }
    }
    let image = art.get(type_key, "Image")?.trimmed().raw.to_ascii_uppercase();
    if image.eq_ignore_ascii_case(type_key) {
        return None;
    }
    let v = art.get(&image, key)?.trimmed();
    if v.raw.is_empty() {
        None
    } else {
        Some(v.raw.to_string())
    }
}

/// 从 `Primary` 武器节读取伤害 / 射程 / ROF / 弹头；缺省时保留类型节 ROF。
fn resolve_primary_weapon(view: LayeredIniView<'_>, primary: &str, techno_rof: u32) -> (u32, u32, u32, String) {
    if primary.is_empty() {
        return (0, 0, techno_rof, String::new());
    }
    let Some(section) = view.section(primary)
    else {
        return (0, 0, techno_rof, String::new());
    };
    let Ok(w) = section.deserialize::<WeaponSectionFields>()
    else {
        return (0, 0, techno_rof, String::new());
    };
    let weapon_rof = w.rof.unwrap_or(0);
    let rof = if weapon_rof > 0 { weapon_rof } else { techno_rof };
    let warhead = w
        .warhead
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    (w.damage.unwrap_or(0), w.range.unwrap_or(0), rof, warhead)
}
