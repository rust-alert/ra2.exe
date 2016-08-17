//! 从 `RulesSystem` / INI 解释生成冻结 [`RuntimeDefinitions`]。
//!
//! 引擎不得再按外部类型名猜测电力、工厂、部署关系。

use ra_assets::TechnoKind;
use ra_types::{
    BuildCat, BuiltinCapability, DeployableDefinition, DeploymentPlacement, Foundation, PowerProfile, PrerequisiteGroups, ProductionCategory,
    ProductionProfile, RuntimeDefinitions, StolenTechKind, StructureDefinition, SuperWeaponDefinition, TechnoClass, TechnoDefinition,
    TerrainSpawnerDefinition, TypeId, WarheadDefinition,
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

    let g = &rules.globals;
    defs.prerequisite_groups = PrerequisiteGroups {
        power: g.prerequisite_power.clone(),
        factory: g.prerequisite_factory.clone(),
        barracks: g.prerequisite_barracks.clone(),
        radar: g.prerequisite_radar.clone(),
        tech: g.prerequisite_tech.clone(),
        proc: g.prerequisite_proc.clone(),
        proc_alternate: g.prerequisite_proc_alternate.clone(),
    };
    defs.default_tech_level = g.multiplayer_tech_level.unwrap_or(10).max(0);
    // `[General]` 侧栏扳手：缺键回落原版库存默认。
    defs.repair_percent = g.repair_percent.map(|v| v.max(0) as u32).unwrap_or(15);
    defs.repair_step = g.repair_step.map(|v| v.max(1) as u32).unwrap_or(8);
    defs.repair_interval_ticks = g.repair_rate_minutes.map(repair_rate_minutes_to_ticks).unwrap_or(14);
    defs.speak_delay_ticks = speak_delay_minutes_to_ticks(g.speak_delay_minutes.unwrap_or(0.0));
    for country in rules.countries.countries() {
        if let Some(kind) = StolenTechKind::from_side(&country.side) {
            defs.stolen_tech_by_house.insert(&country.id, kind);
        }
    }

    for sw in rules.super_weapons.iter() {
        let id = alloc();
        defs.super_weapons.insert(SuperWeaponDefinition {
            id,
            type_key: sw.id.clone(),
            ui_name: sw.ui_name.clone(),
            kind: sw.kind.clone(),
            action: sw.action.clone(),
            recharge_time: sw.recharge_time,
            sidebar_image: sw.sidebar_image.clone(),
            weapon: sw.weapon.clone(),
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
            prerequisite: tt.prerequisite.clone(),
            prerequisite_override: tt.prerequisite_override.clone(),
            required_houses: tt.required_houses.clone(),
            forbidden_houses: tt.forbidden_houses.clone(),
            build_limit: tt.build_limit,
            build_time: tt.build_time,
            requires_stolen_allied_tech: tt.requires_stolen_allied_tech,
            requires_stolen_soviet_tech: tt.requires_stolen_soviet_tech,
            requires_stolen_third_tech: tt.requires_stolen_third_tech,
            pixel_selection_bracket_delta: tt.pixel_selection_bracket_delta,
        });

        if tt.harvester {
            if !defs.capabilities.builtins.contains(&BuiltinCapability::Harvester) {
                defs.capabilities.builtins.push(BuiltinCapability::Harvester);
            }
        }

        if tt.kind != TechnoKind::Building {
            // 部署关系可挂在载具上
            if !tt.deploys_into.is_empty() {
                let target_key = tt.deploys_into.clone();
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

        let power_raw = tt.power;
        let (output, drain) = if power_raw >= 0 { (power_raw, 0) } else { (0, -power_raw) };
        let powered = tt.powered.unwrap_or(drain > 0);
        let construction_yard = tt.construction_yard;
        let refinery = tt.refinery;
        let radar = tt.radar;
        let build_cat = BuildCat::parse(&tt.build_cat);
        let capturable = tt.capturable;
        let factory = if tt.factory.trim().is_empty() {
            None
        } else {
            Some(parse_factory_category(&tt.factory))
        };
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
        let super_weapon = if tt.super_weapon.is_empty() {
            None
        } else {
            Some(tt.super_weapon.clone())
        };
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

        // `Foundation` / `Height` 已在装载期由 rules + art（含 `Image=`）解到 `TechnoType`。
        let foundation = Foundation::parse(&tt.foundation);
        let height = tt.height.unwrap_or(2).max(1);
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
            height,
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

    fill_terrain_spawners(&mut defs, &rules.rules);
    defs.overlays = rules.overlay_types.clone();

    defs
}

const TERRAIN_SPAWN_PROBABILITY_DENOMINATOR: f32 = 1_000_000.0;

#[derive(Debug, serde::Deserialize)]
struct TerrainSpawnerSectionFields {
    #[serde(rename = "SpawnsTiberium")]
    spawns_tiberium: Option<bool>,
    #[serde(rename = "IsAnimated")]
    is_animated: Option<bool>,
    #[serde(rename = "AnimationProbability")]
    animation_probability: Option<f32>,
    #[serde(rename = "AnimationRate")]
    animation_rate: Option<u16>,
}

fn fill_terrain_spawners(defs: &mut RuntimeDefinitions, rules: &ra_assets::IniDocument) {
    for section in &rules.sections {
        let Ok(fields) = section.deserialize::<TerrainSpawnerSectionFields>()
        else {
            continue;
        };
        if fields.spawns_tiberium != Some(true) || fields.is_animated != Some(true) {
            continue;
        }
        let probability = fields
            .animation_probability
            .map(|v| (v.clamp(0.0, 1.0) * TERRAIN_SPAWN_PROBABILITY_DENOMINATOR).round() as u32)
            .unwrap_or(0);
        let rate = fields.animation_rate.unwrap_or(1).max(1);
        let type_key = section.name_raw.trim().to_ascii_uppercase();
        if type_key.is_empty() {
            continue;
        }
        defs.terrain_spawners.insert(TerrainSpawnerDefinition {
            type_key,
            animation_probability_micros: probability,
            animation_rate_ticks: rate,
        });
    }
}

#[doc(hidden)]
pub fn parse_factory_category(raw: &str) -> ProductionCategory {
    match raw.trim().to_ascii_lowercase().as_str() {
        "infantrytype" | "infantry" => ProductionCategory::Infantry,
        "unittype" | "vehicle" | "unit" => ProductionCategory::Vehicle,
        "aircrafttype" | "aircraft" => ProductionCategory::Aircraft,
        "buildingtype" | "building" => ProductionCategory::Building,
        _ => ProductionCategory::Vehicle,
    }
}

/// 原版 `RepairRate`（分钟）→ 逻辑 tick：`ftol(rate * 900)`，至少 1。
pub fn repair_rate_minutes_to_ticks(rate_minutes: f64) -> u64 {
    if !rate_minutes.is_finite() || rate_minutes <= 0.0 {
        return 14;
    }
    let ticks = (rate_minutes * 900.0).trunc() as i64;
    ticks.max(1) as u64
}

/// `[AudioVisual] SpeakDelay`（分钟）× 900 → 逻辑 tick；非正数则为 0。
pub fn speak_delay_minutes_to_ticks(minutes: f64) -> u32 {
    if !(minutes > 0.0) {
        return 0;
    }
    (minutes * 900.0) as u32
}
