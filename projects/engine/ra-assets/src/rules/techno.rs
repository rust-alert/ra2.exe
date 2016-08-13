//! 从 rules 列表节解析 TechnoType 注册表（节字段经 Serde 一次解码）。

use std::collections::HashMap;

use serde::Deserialize;

use crate::ini::IniDocument;

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
        let mut by_id = HashMap::new();
        for (section, kind) in [
            ("InfantryTypes", TechnoKind::Infantry),
            ("VehicleTypes", TechnoKind::Vehicle),
            ("AircraftTypes", TechnoKind::Aircraft),
            ("BuildingTypes", TechnoKind::Building),
        ] {
            let Some(list) = rules.section(section)
            else {
                continue;
            };
            for (_key, name) in list.pairs() {
                let id = name.trim();
                if id.is_empty() {
                    continue;
                }
                let id_up = id.to_ascii_uppercase();
                if by_id.contains_key(&id_up) {
                    continue;
                }
                if let Some(tt) = parse_techno(rules, &id_up, kind) {
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

fn parse_techno(rules: &IniDocument, id: &str, kind: TechnoKind) -> Option<TechnoType> {
    let section = rules.section(id)?;
    let fields: TechnoSectionFields = section.deserialize().ok()?;
    let primary = fields
        .primary
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    let techno_rof = fields.rof.unwrap_or(0);
    let (damage, range, rof, warhead) = resolve_primary_weapon(rules, &primary, techno_rof);
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
    })
}

/// 从 `Primary` 武器节读取伤害 / 射程 / ROF / 弹头；缺省时保留类型节 ROF。
fn resolve_primary_weapon(rules: &IniDocument, primary: &str, techno_rof: u32) -> (u32, u32, u32, String) {
    if primary.is_empty() {
        return (0, 0, techno_rof, String::new());
    }
    let Some(section) = rules.section(primary)
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
