//! 从 rules 列表节加载基础 TechnoType 字段。

use std::collections::HashMap;

use ra_assets::IniDocument;

/// 一份可战斗物类型的常用数值（预览 / 仿真起步用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnoType {
    pub id: String,
    pub kind: TechnoKind,
    pub strength: u32,
    pub armor: String,
    pub speed: u32,
    pub sight: u32,
    pub cost: u32,
    pub tech_level: i32,
    pub owner: String,
    pub image: String,
}

/// Techno 大类（对应 rules 列表节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TechnoKind {
    Infantry,
    Vehicle,
    Aircraft,
    Building,
}

/// `type_id`（大写）→ TechnoType。
#[derive(Debug, Clone, Default)]
pub struct TechnoTypeRegistry {
    by_id: HashMap<String, TechnoType>,
}

impl TechnoTypeRegistry {
    /// 解析 `[InfantryTypes]` / `[VehicleTypes]` / `[AircraftTypes]` / `[BuildingTypes]`。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let mut by_id = HashMap::new();
        for (section, kind) in [
            ("InfantryTypes", TechnoKind::Infantry),
            ("VehicleTypes", TechnoKind::Vehicle),
            ("AircraftTypes", TechnoKind::Aircraft),
            ("BuildingTypes", TechnoKind::Building),
        ] {
            let Some(list) = rules.sections.get(section) else {
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

    pub fn get(&self, id: &str) -> Option<&TechnoType> {
        self.by_id.get(&id.to_ascii_uppercase())
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn count_kind(&self, kind: TechnoKind) -> usize {
        self.by_id.values().filter(|t| t.kind == kind).count()
    }
}

fn parse_techno(rules: &IniDocument, id: &str, kind: TechnoKind) -> Option<TechnoType> {
    // 节名大小写与零售一致时直接取；否则扫一遍。
    let section_key = if rules.sections.contains_key(id) {
        id.to_string()
    } else {
        rules
            .sections
            .keys()
            .find(|k| k.eq_ignore_ascii_case(id))?
            .clone()
    };
    let strength = parse_u32(rules.get(&section_key, "Strength")).unwrap_or(1);
    let armor = rules
        .get(&section_key, "Armor")
        .unwrap_or("none")
        .to_string();
    let speed = parse_u32(rules.get(&section_key, "Speed")).unwrap_or(0);
    let sight = parse_u32(rules.get(&section_key, "Sight")).unwrap_or(0);
    let cost = parse_u32(rules.get(&section_key, "Cost")).unwrap_or(0);
    let tech_level = rules
        .get(&section_key, "TechLevel")
        .and_then(|s| s.parse().ok())
        .unwrap_or(-1);
    let owner = rules
        .get(&section_key, "Owner")
        .unwrap_or("")
        .to_string();
    let image = rules
        .get(&section_key, "Image")
        .unwrap_or(id)
        .to_ascii_uppercase();
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
    })
}

fn parse_u32(raw: Option<&str>) -> Option<u32> {
    raw?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vehicle_list() {
        let doc = IniDocument::parse(
            b"[VehicleTypes]\n0=MTNK\n1=HTNK\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nSight=6\nCost=800\nTechLevel=2\nOwner=Americans\nImage=MTNK\n\
[HTNK]\nStrength=600\nArmor=heavy\nSpeed=4\nSight=6\nCost=1400\nTechLevel=6\nOwner=Americans\n",
        )
        .unwrap();
        let reg = TechnoTypeRegistry::from_rules(&doc);
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.count_kind(TechnoKind::Vehicle), 2);
        let m = reg.get("mtnk").unwrap();
        assert_eq!(m.strength, 300);
        assert_eq!(m.speed, 6);
        assert_eq!(m.cost, 800);
        assert_eq!(m.image, "MTNK");
    }
}
