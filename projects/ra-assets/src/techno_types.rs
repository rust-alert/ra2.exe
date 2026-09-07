//! 从 rules 列表节解析 TechnoType 注册表。

use std::collections::HashMap;

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
    /// 主武器名（`Primary`）；空表示未配置。
    pub primary: String,
    /// 主武器伤害（来自武器节 `Damage`）；0 表示未配置。
    pub damage: u32,
    /// 主武器射程（来自武器节 `Range`，格）；0 表示未配置。
    pub range: u32,
    /// 射速间隔（tick）；优先武器节 `ROF`，否则类型节；0 表示未配置。
    pub rof: u32,
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
            let Some(list) = rules.sections.get(section)
            else {
                continue;
            };
            for (_key, name) in &list.order {
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
}

fn parse_techno(rules: &IniDocument, id: &str, kind: TechnoKind) -> Option<TechnoType> {
    // 节名可能与列表项大小写不一致，先精确再忽略大小写匹配。
    let section_key = if rules.sections.contains_key(id) {
        id.to_string()
    }
    else {
        rules.sections.keys().find(|k| k.eq_ignore_ascii_case(id))?.clone()
    };
    let strength = parse_u32(rules.get(&section_key, "Strength")).unwrap_or(1);
    let armor = rules.get(&section_key, "Armor").unwrap_or("none").to_string();
    let speed = parse_u32(rules.get(&section_key, "Speed")).unwrap_or(0);
    let sight = parse_u32(rules.get(&section_key, "Sight")).unwrap_or(0);
    let cost = parse_u32(rules.get(&section_key, "Cost")).unwrap_or(0);
    let tech_level = rules.get(&section_key, "TechLevel").and_then(|s| s.parse().ok()).unwrap_or(-1);
    let owner = rules.get(&section_key, "Owner").unwrap_or("").to_string();
    let image = rules.get(&section_key, "Image").unwrap_or(id).to_ascii_uppercase();
    let primary = rules
        .get(&section_key, "Primary")
        .unwrap_or("")
        .trim()
        .to_ascii_uppercase();
    let techno_rof = parse_u32(rules.get(&section_key, "ROF")).unwrap_or(0);
    let (damage, range, rof) = resolve_primary_weapon(rules, &primary, techno_rof);
    Some(TechnoType {
        id: id.to_string(),
        kind,
        strength,
        armor,
        speed,
        sight,
        cost,
        tech_level,
        owner,
        image,
        primary,
        damage,
        range,
        rof,
    })
}

/// 从 `Primary` 武器节读取 `Damage` / `Range` / `ROF`；缺省时保留类型节 ROF。
fn resolve_primary_weapon(rules: &IniDocument, primary: &str, techno_rof: u32) -> (u32, u32, u32) {
    if primary.is_empty() {
        return (0, 0, techno_rof);
    }
    let section_key = if rules.sections.contains_key(primary) {
        primary.to_string()
    }
    else if let Some(k) = rules.sections.keys().find(|k| k.eq_ignore_ascii_case(primary)) {
        k.clone()
    }
    else {
        return (0, 0, techno_rof);
    };
    let damage = parse_u32(rules.get(&section_key, "Damage")).unwrap_or(0);
    let range = parse_u32(rules.get(&section_key, "Range")).unwrap_or(0);
    let weapon_rof = parse_u32(rules.get(&section_key, "ROF")).unwrap_or(0);
    let rof = if weapon_rof > 0 { weapon_rof } else { techno_rof };
    (damage, range, rof)
}

fn parse_u32(raw: Option<&str>) -> Option<u32> {
    raw?.trim().parse().ok()
}
