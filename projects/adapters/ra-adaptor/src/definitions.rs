//! 从 `RulesSystem` / INI 解释生成冻结 [`RuntimeDefinitions`]。
//!
//! 引擎不得再按外部类型名猜测电力、工厂、部署关系。

use ra_assets::TechnoKind;
use ra_types::{
    ArmorKind, BuildCat, BuiltinCapability, DeployableDefinition, DeploymentPlacement, Foundation, HouseAllowList, PowerProfile,
    PrerequisiteGroups, PrerequisiteToken, ProductionCategory, ProductionProfile, RuntimeDefinitions, StolenTechKind, StructureDefinition,
    SuperWeaponDefinition, TechnoClass, TechnoDefinition, TypeId, WarheadDefinition, WarheadId, WeaponDefinition, WeaponId,
};
use std::collections::HashMap;

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
    let mut next_weapon = 1u32;
    let mut alloc_weapon = || {
        let id = WeaponId(next_weapon);
        next_weapon = next_weapon.saturating_add(1);
        id
    };
    let mut next_warhead = 1u32;
    let mut alloc_warhead = || {
        let id = WarheadId(next_warhead);
        next_warhead = next_warhead.saturating_add(1);
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
            armor: ArmorKind::parse(&tt.armor),
            speed: tt.speed,
            owner: HouseAllowList::parse_owner(&tt.owner),
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
            primary: tt.primary.clone(),
            primary_id: WeaponId(0),
            warhead: tt.warhead.to_ascii_uppercase(),
            warhead_id: WarheadId(0),
            prerequisite: tt.prerequisite.iter().filter_map(|s| PrerequisiteToken::parse_raw(s)).collect(),
            prerequisite_override: tt.prerequisite_override.iter().filter_map(|s| PrerequisiteToken::parse_raw(s)).collect(),
            required_houses: HouseAllowList::from_tokens(&tt.required_houses),
            forbidden_houses: HouseAllowList::from_tokens(&tt.forbidden_houses),
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
        let super_weapon_id = super_weapon.as_ref().and_then(|k| defs.super_weapons.get(k).map(|sw| sw.id));
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
            armor: ArmorKind::parse(&tt.armor),
            construction_yard,
            refinery,
            radar,
            build_cat,
            capturable,
            production,
            owner: HouseAllowList::parse_owner(&tt.owner),
            foundation,
            height,
            super_weapon,
            super_weapon_id,
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

    // 前置 token：UnboundType → TypeId（全部 techno 已入库后）。
    let type_ids: HashMap<String, TypeId> = defs.techno.iter().map(|t| (t.type_key.clone(), t.id)).collect();
    let resolve = |key: &str| type_ids.get(&key.to_ascii_uppercase()).copied();
    for techno in defs.techno.iter_mut() {
        techno.prerequisite = std::mem::take(&mut techno.prerequisite)
            .into_iter()
            .map(|t| t.bind_type_id(&resolve))
            .collect();
        techno.prerequisite_override = std::mem::take(&mut techno.prerequisite_override)
            .into_iter()
            .map(|t| t.bind_type_id(&resolve))
            .collect();
    }

    defs.production.count = defs.structures.iter().filter(|s| s.production.is_some()).count() as u32;

    // 武器表：按 techno `Primary` 去重投影，再绑弹头 id。
    for tt in rules.techno_types.iter() {
        let key = tt.primary.trim().to_ascii_uppercase();
        if key.is_empty() || defs.weapons.get(&key).is_some() {
            continue;
        }
        let id = alloc_weapon();
        defs.weapons.insert(WeaponDefinition {
            id,
            type_key: key,
            damage: tt.damage,
            range: tt.range,
            rof: tt.rof,
            warhead: tt.warhead.to_ascii_uppercase(),
            warhead_id: WarheadId(0),
        });
    }

    let mut warhead_keys: Vec<String> = defs
        .weapons
        .iter()
        .map(|w| w.warhead.clone())
        .chain(defs.techno.iter().map(|t| t.warhead.clone()))
        .filter(|w| !w.is_empty())
        .collect();
    warhead_keys.sort();
    warhead_keys.dedup();
    for key in warhead_keys {
        let verses = rules.warheads.get(&key).map(|w| w.verses).unwrap_or([100; 11]);
        let id = alloc_warhead();
        defs.warheads.insert(WarheadDefinition { id, type_key: key, verses });
    }
    for weapon in defs.weapons.iter_mut() {
        weapon.warhead_id = if weapon.warhead.is_empty() {
            WarheadId(0)
        } else {
            defs.warheads.get(&weapon.warhead).map(|w| w.id).unwrap_or(WarheadId(0))
        };
    }
    for techno in defs.techno.iter_mut() {
        techno.primary_id = if techno.primary.is_empty() {
            WeaponId(0)
        } else {
            defs.weapons.get(&techno.primary).map(|w| w.id).unwrap_or(WeaponId(0))
        };
        techno.warhead_id = if let Some(w) = defs.weapons.get_by_id(techno.primary_id) {
            w.warhead_id
        } else if techno.warhead.is_empty() {
            WarheadId(0)
        } else {
            defs.warheads.get(&techno.warhead).map(|w| w.id).unwrap_or(WarheadId(0))
        };
    }

    defs.terrain_spawners = rules.terrain_spawners.clone();
    defs.overlays = rules.overlay_types.clone();

    defs
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
