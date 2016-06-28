//! 从 `RulesSystem` / INI 解释生成冻结 [`RuntimeDefinitions`]。
//!
//! 引擎不得再按外部类型名猜测电力、工厂、部署关系。

use ra_assets::TechnoKind;
use ra_types::{
    BuildCat, BuiltinCapability, DeployableDefinition, DeploymentPlacement, Foundation, PowerProfile, PrerequisiteGroups, ProductionCategory,
    ProductionProfile, RuntimeDefinitions, StolenTechKind, StructureDefinition, SuperWeaponDefinition, TechnoClass, TechnoDefinition, TypeId,
    WarheadDefinition,
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

    defs.prerequisite_groups = parse_prerequisite_groups(&rules.rules);
    defs.default_tech_level = ini_i32(&rules.rules, "MultiplayerDialogSettings", "TechLevel").unwrap_or(10).max(0);
    // `[General]` 侧栏扳手：缺键回落原版库存默认。
    defs.repair_percent = ini_i32(&rules.rules, "General", "RepairPercent")
        .map(|v| v.max(0) as u32)
        .unwrap_or(15);
    defs.repair_step = ini_i32(&rules.rules, "General", "RepairStep")
        .map(|v| v.max(1) as u32)
        .unwrap_or(8);
    defs.repair_interval_ticks = ini_string(&rules.rules, "General", "RepairRate")
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(repair_rate_minutes_to_ticks)
        .unwrap_or(14);
    for country in rules.countries.countries() {
        if let Some(kind) = StolenTechKind::from_side(&country.side) {
            defs.stolen_tech_by_house.insert(&country.id, kind);
        }
    }

    for sw_key in list_section_type_keys(&rules.rules, "SuperWeaponTypes") {
        let id = alloc();
        let ui_name = ini_string(&rules.rules, &sw_key, "UIName").unwrap_or_default();
        let kind = ini_string(&rules.rules, &sw_key, "Type")
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_default();
        let action = ini_string(&rules.rules, &sw_key, "Action")
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_default();
        let recharge_time = ini_i32(&rules.rules, &sw_key, "RechargeTime").unwrap_or(0).max(0);
        let sidebar_image = ini_string(&rules.rules, &sw_key, "SidebarImage").unwrap_or_default();
        let weapon = ini_string(&rules.rules, &sw_key, "Weapon")
            .map(|s| s.to_ascii_uppercase())
            .unwrap_or_default();
        defs.super_weapons.insert(SuperWeaponDefinition {
            id,
            type_key: sw_key,
            ui_name,
            kind,
            action,
            recharge_time,
            sidebar_image,
            weapon,
        });
    }
    if !defs.super_weapons.is_empty() && !defs.capabilities.builtins.contains(&BuiltinCapability::SuperWeapon) {
        defs.capabilities.builtins.push(BuiltinCapability::SuperWeapon);
    }

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
            engineer: tt.engineer,
            harvester: tt.harvester,
            category: tt.category.clone(),
            sight: tt.sight,
            damage: tt.damage,
            range: tt.range,
            rof: tt.rof,
            warhead: tt.warhead.to_ascii_uppercase(),
            prerequisite: ini_csv_tokens(&rules.rules, &key, "Prerequisite"),
            prerequisite_override: ini_csv_tokens(&rules.rules, &key, "PrerequisiteOverride"),
            required_houses: ini_csv_tokens(&rules.rules, &key, "RequiredHouses"),
            forbidden_houses: ini_csv_tokens(&rules.rules, &key, "ForbiddenHouses"),
            build_limit: ini_i32(&rules.rules, &key, "BuildLimit").unwrap_or(0).max(0),
            build_time: ini_i32(&rules.rules, &key, "BuildTime").unwrap_or(0).max(0) as u32,
            requires_stolen_allied_tech: ini_bool(&rules.rules, &key, "RequiresStolenAlliedTech").unwrap_or(false),
            requires_stolen_soviet_tech: ini_bool(&rules.rules, &key, "RequiresStolenSovietTech").unwrap_or(false),
            requires_stolen_third_tech: ini_bool(&rules.rules, &key, "RequiresStolenThirdTech").unwrap_or(false),
        });

        if tt.harvester {
            if !defs.capabilities.builtins.contains(&BuiltinCapability::Harvester) {
                defs.capabilities.builtins.push(BuiltinCapability::Harvester);
            }
        }

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
        let radar = ini_bool(&rules.rules, &key, "Radar").unwrap_or(false);
        let build_cat = BuildCat::parse(&ini_string(&rules.rules, &key, "BuildCat").unwrap_or_default());
        let capturable = ini_bool(&rules.rules, &key, "Capturable").unwrap_or(false);
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
        if radar {
            capabilities.push(BuiltinCapability::Radar);
        }
        if capturable {
            capabilities.push(BuiltinCapability::Capturable);
        }
        if production.is_some() {
            capabilities.push(BuiltinCapability::Producer);
        }
        let super_weapon = ini_string(&rules.rules, &key, "SuperWeapon").map(|s| s.to_ascii_uppercase());
        if super_weapon.is_some() {
            capabilities.push(BuiltinCapability::SuperWeapon);
            if !defs.capabilities.builtins.contains(&BuiltinCapability::SuperWeapon) {
                defs.capabilities.builtins.push(BuiltinCapability::SuperWeapon);
            }
        }
        for c in &capabilities {
            if !defs.capabilities.builtins.contains(c) {
                defs.capabilities.builtins.push(*c);
            }
        }

        let foundation = Foundation::parse(&ini_string(&rules.rules, &key, "Foundation").unwrap_or_default());
        defs.structures.insert(StructureDefinition {
            id,
            type_key: key,
            power: PowerProfile { output, drain, requires_power: powered },
            cost: tt.cost as i32,
            strength: tt.strength.max(1),
            armor: tt.armor.clone(),
            construction_yard,
            refinery,
            radar,
            build_cat,
            capturable,
            production,
            owner: tt.owner.clone(),
            foundation,
            super_weapon,
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

fn parse_prerequisite_groups(doc: &ra_assets::IniDocument) -> PrerequisiteGroups {
    PrerequisiteGroups {
        power: ini_csv_tokens(doc, "General", "PrerequisitePower"),
        factory: ini_csv_tokens(doc, "General", "PrerequisiteFactory"),
        barracks: ini_csv_tokens(doc, "General", "PrerequisiteBarracks"),
        radar: ini_csv_tokens(doc, "General", "PrerequisiteRadar"),
        tech: ini_csv_tokens(doc, "General", "PrerequisiteTech"),
        proc: ini_csv_tokens(doc, "General", "PrerequisiteProc"),
        proc_alternate: ini_csv_tokens(doc, "General", "PrerequisiteProcAlternate"),
    }
}

/// 读取列表节（如 `[SuperWeaponTypes]`）的类型键，保序、大写、去空。
fn list_section_type_keys(doc: &ra_assets::IniDocument, section: &str) -> Vec<String> {
    let Some(sec) = doc.section(section)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_idx, value) in sec.pairs() {
        let key = value.trim();
        if key.is_empty() {
            continue;
        }
        let upper = key.to_ascii_uppercase();
        if !out.iter().any(|k: &String| k == &upper) {
            out.push(upper);
        }
    }
    out
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

/// 原版 `RepairRate`（分钟）→ 逻辑 tick：`ftol(rate * 900)`，至少 1。
fn repair_rate_minutes_to_ticks(rate_minutes: f64) -> u64 {
    if !rate_minutes.is_finite() || rate_minutes <= 0.0 {
        return 14;
    }
    let ticks = (rate_minutes * 900.0).trunc() as i64;
    ticks.max(1) as u64
}

fn ini_bool(doc: &ra_assets::IniDocument, section: &str, key: &str) -> Option<bool> {
    let v = ini_string(doc, section, key)?.to_ascii_lowercase();
    match v.as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

/// 逗号 / 分号分隔 token，统一大写；空段丢弃。
fn ini_csv_tokens(doc: &ra_assets::IniDocument, section: &str, key: &str) -> Vec<String> {
    let Some(raw) = ini_string(doc, section, key)
    else {
        return Vec::new();
    };
    raw.split(|c| c == ',' || c == ';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::{
        ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry,
    };
    use ra_types::GameEdition;

    fn rules_from(text: &[u8]) -> RulesSystem {
        let rules = IniDocument::parse(text).expect("test ini");
        RulesSystem {
            edition: GameEdition::Ra2,
            rules: rules.clone(),
            art: IniDocument::default(),
            overlay_types: OverlayTypeRegistry::default(),
            color_schemes: ColorSchemes::default(),
            countries: CountryRegistry::default(),
            techno_types: TechnoTypeRegistry::from_rules(&rules),
            warheads: WarheadRegistry::default(),
        }
    }

    #[test]
    fn repair_rate_minutes_to_ticks_matches_stock_general() {
        assert_eq!(repair_rate_minutes_to_ticks(0.016), 14);
        assert_eq!(repair_rate_minutes_to_ticks(0.0), 14);
        assert_eq!(repair_rate_minutes_to_ticks(-1.0), 14);
    }

    #[test]
    fn build_runtime_definitions_reads_general_repair_keys() {
        let rules = rules_from(
            b"[General]\nRepairPercent=25\nRepairStep=16\nRepairRate=.032\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\n",
        );
        let defs = build_runtime_definitions(&rules);
        assert_eq!(defs.repair_percent, 25);
        assert_eq!(defs.repair_step, 16);
        assert_eq!(defs.repair_interval_ticks, 28);
    }

    #[test]
    fn build_runtime_definitions_falls_back_to_stock_repair_defaults() {
        let rules = rules_from(b"[BuildingTypes]\n0=GAPOWR\n[GAPOWR]\nCost=1\nStrength=1\n");
        let defs = build_runtime_definitions(&rules);
        assert_eq!(defs.repair_percent, 15);
        assert_eq!(defs.repair_step, 8);
        assert_eq!(defs.repair_interval_ticks, 14);
    }

    #[test]
    fn build_runtime_definitions_parses_super_weapon_types_and_building_link() {
        let rules = rules_from(
            b"[SuperWeaponTypes]\n0=LightningStorm\n\
[LightningStorm]\nUIName=Name:LightningStorm\nType=LightningStorm\nAction=LightningStorm\nRechargeTime=10\nSidebarImage=SSWLSICON\n\
[BuildingTypes]\n0=GACNST\n1=GATECH\n\
[GACNST]\nConstructionYard=yes\nCost=2500\nStrength=1000\n\
[GATECH]\nCost=1500\nStrength=600\nSuperWeapon=LightningStorm\n",
        );
        let defs = build_runtime_definitions(&rules);
        let sw = defs.super_weapons.get("LightningStorm").expect("SW");
        assert_eq!(sw.ui_name, "Name:LightningStorm");
        assert_eq!(sw.kind, "LIGHTNINGSTORM");
        assert_eq!(sw.action, "LIGHTNINGSTORM");
        assert_eq!(sw.recharge_time, 10);
        assert_eq!(sw.sidebar_image, "SSWLSICON");
        assert_eq!(
            defs.structures.get("GATECH").and_then(|s| s.super_weapon.as_deref()),
            Some("LIGHTNINGSTORM")
        );
        assert!(defs.capabilities.builtins.contains(&BuiltinCapability::SuperWeapon));
        assert!(defs
            .structures
            .get("GATECH")
            .expect("tech")
            .capabilities
            .contains(&BuiltinCapability::SuperWeapon));
    }
}
