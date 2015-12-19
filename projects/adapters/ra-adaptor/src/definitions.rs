//! 从 `RulesSystem` / INI 解释生成冻结 [`RuntimeDefinitions`]。
//!
//! 引擎不得再按外部类型名猜测电力、工厂、部署关系。

use ra_assets::TechnoKind;
use ra_types::{
    BuiltinCapability, DeployableDefinition, DeploymentPlacement, PowerProfile, ProductionCategory, ProductionProfile, RuntimeDefinitions,
    StructureDefinition, TechnoClass, TechnoDefinition, TypeId, WarheadDefinition,
};

use crate::RulesSystem;

/// 由已装载规则快照构建冻结运行时定义。
pub fn build_runtime_definitions(rules: &RulesSystem) -> RuntimeDefinitions {
    let mut defs = RuntimeDefinitions::default();
    let mut next_id = 1u32;
    let mut alloc = || {
        let id = TypeId(next_id);
        next_id = next_id.saturating_add(1);
        id
    };

    for tt in rules.techno_types.iter() {
        let key = tt.id.to_ascii_uppercase();
        let class = match tt.kind {
            TechnoKind::Infantry => TechnoClass::Infantry,
            TechnoKind::Vehicle => TechnoClass::Vehicle,
            TechnoKind::Aircraft => TechnoClass::Aircraft,
            TechnoKind::Building => TechnoClass::Building,
        };
        let id = alloc();
        defs.techno.insert(TechnoDefinition {
            id,
            type_key: key.clone(),
            class,
            cost: tt.cost as i32,
            strength: tt.strength,
            armor: tt.armor.clone(),
            speed: tt.speed,
            owner: tt.owner.clone(),
            tech_level: tt.tech_level,
            naval: tt.naval,
            agent: tt.agent,
            category: tt.category.clone(),
            sight: tt.sight,
            damage: tt.damage,
            range: tt.range,
            rof: tt.rof,
            warhead: tt.warhead.to_ascii_uppercase(),
        });

        if tt.kind != TechnoKind::Building {
            // 部署关系可挂在载具上
            if let Some(target) = ini_string(&rules.rules, &key, "DeploysInto") {
                let target_key = target.to_ascii_uppercase();
                let target_id = defs.techno.get(&target_key).map(|t| t.id).unwrap_or(TypeId(0));
                defs.deployables.insert(DeployableDefinition {
                    source: id,
                    source_key: key.clone(),
                    target: target_id,
                    target_key,
                    placement: DeploymentPlacement::InPlace,
                });
                defs.capabilities.builtins.push(BuiltinCapability::Deployable);
            }
            continue;
        }

        let power_raw = ini_i32(&rules.rules, &key, "Power").unwrap_or(0);
        let (output, drain) = if power_raw >= 0 { (power_raw, 0) } else { (0, -power_raw) };
        let powered = ini_bool(&rules.rules, &key, "Powered").unwrap_or(drain > 0);
        let construction_yard = ini_bool(&rules.rules, &key, "ConstructionYard").unwrap_or(false);
        let refinery = ini_bool(&rules.rules, &key, "Refinery").unwrap_or(false);
        let factory = ini_string(&rules.rules, &key, "Factory").map(|s| parse_factory_category(&s));
        let production = factory.map(|category| ProductionProfile { category });

        let mut capabilities = vec![BuiltinCapability::Structure];
        if output > 0 {
            capabilities.push(BuiltinCapability::PowerProducer);
        }
        if drain > 0 {
            capabilities.push(BuiltinCapability::PowerConsumer);
        }
        if construction_yard {
            capabilities.push(BuiltinCapability::ConstructionYard);
        }
        if refinery {
            capabilities.push(BuiltinCapability::Refinery);
        }
        if production.is_some() {
            capabilities.push(BuiltinCapability::Producer);
        }
        for c in &capabilities {
            if !defs.capabilities.builtins.contains(c) {
                defs.capabilities.builtins.push(*c);
            }
        }

        defs.structures.insert(StructureDefinition {
            id,
            type_key: key,
            power: PowerProfile { output, drain, requires_power: powered },
            cost: tt.cost as i32,
            strength: tt.strength.max(1),
            armor: tt.armor.clone(),
            construction_yard,
            refinery,
            production,
            owner: tt.owner.clone(),
            capabilities,
        });
    }

    // 第二遍：修正 deployables 的 target TypeId（目标可能后于源解析）
    let mut fixed = Vec::new();
    for d in defs.deployables.iter() {
        let mut d = d.clone();
        if let Some(t) = defs.techno.get(&d.target_key) {
            d.target = t.id;
        }
        fixed.push(d);
    }
    defs.deployables = Default::default();
    for d in fixed {
        defs.deployables.insert(d);
    }

    defs.production.count = defs.structures.iter().filter(|s| s.production.is_some()).count() as u32;

    let mut warhead_keys: Vec<String> = defs.techno.iter().map(|t| t.warhead.clone()).filter(|w| !w.is_empty()).collect();
    warhead_keys.sort();
    warhead_keys.dedup();
    for key in warhead_keys {
        let verses = rules.warheads.get(&key).map(|w| w.verses).unwrap_or([100; 11]);
        let id = alloc();
        defs.warheads.insert(WarheadDefinition { id, type_key: key, verses });
    }

    defs
}

fn parse_factory_category(raw: &str) -> ProductionCategory {
    match raw.trim().to_ascii_lowercase().as_str() {
        "infantrytype" | "infantry" => ProductionCategory::Infantry,
        "unittype" | "vehicle" | "unit" => ProductionCategory::Vehicle,
        "aircrafttype" | "aircraft" => ProductionCategory::Aircraft,
        "buildingtype" | "building" => ProductionCategory::Building,
        _ => ProductionCategory::Vehicle,
    }
}

fn ini_string(doc: &ra_assets::IniDocument, section: &str, key: &str) -> Option<String> {
    doc.get(section, key).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn ini_i32(doc: &ra_assets::IniDocument, section: &str, key: &str) -> Option<i32> {
    ini_string(doc, section, key)?.parse().ok()
}

fn ini_bool(doc: &ra_assets::IniDocument, section: &str, key: &str) -> Option<bool> {
    let v = ini_string(doc, section, key)?.to_ascii_lowercase();
    match v.as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}
