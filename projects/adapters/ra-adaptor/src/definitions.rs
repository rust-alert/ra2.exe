//! 从 `RulesSystem` / INI 解释生成冻结 [`RuntimeDefinitions`]。
//!
//! 引擎不得再按外部类型名猜测电力、工厂、部署关系。

use ra_assets::TechnoKind;
use ra_types::{
    BuiltinCapability, DeployableDefinition, DeploymentPlacement, GameEdition, PowerProfile, PrerequisiteGroups, ProductionCategory,
    ProductionProfile, ProjectileDefinition, ProjectileId, ProjectileName, RaResult, RuntimeDefinitions, StolenTechKind, StructureDefinition,
    SuperWeaponDefinition, TechnoClass, TechnoDefinition, TypeId, WarheadDefinition, WarheadId, WarheadName, WeaponDefinition, WeaponId,
};
use std::collections::HashMap;

use crate::{RulesSystem, rules_system_from_ini_bytes};

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
    let mut next_projectile = 1u32;
    let mut alloc_projectile = || {
        let id = ProjectileId(next_projectile);
        next_projectile = next_projectile.saturating_add(1);
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
            weapon_id: WeaponId(0),
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
            armor: tt.armor,
            speed: tt.speed,
            owner: tt.owner.clone(),
            tech_level: tt.tech_level,
            naval: tt.naval,
            agent: tt.agent,
            engineer: tt.engineer,
            harvester: tt.harvester,
            category: tt.category,
            sight: tt.sight,
            primary: tt.primary.clone(),
            primary_id: WeaponId(0),
            secondary: tt.secondary.clone(),
            secondary_id: WeaponId(0),
            warhead: tt.warhead.clone(),
            warhead_id: WarheadId(0),
            prerequisite: tt.prerequisite.clone().into_vec(),
            prerequisite_override: tt.prerequisite_override.clone().into_vec(),
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
                let target_key = tt.deploys_into.as_str().to_string();
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
        let build_cat = tt.build_cat;
        let capturable = tt.capturable;
        let production = tt.factory.map(|category| ProductionProfile { category });

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
        let super_weapon_id = super_weapon.as_ref().and_then(|k| defs.super_weapons.get(k.as_str()).map(|sw| sw.id));
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
        let foundation = tt.foundation.clone();
        let height = tt.height.unwrap_or(2).max(1);
        defs.structures.insert(StructureDefinition {
            id,
            type_key: key,
            power: PowerProfile { output, drain, requires_power: powered },
            cost: tt.cost as i32,
            strength: tt.strength.max(1),
            armor: tt.armor,
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

    // 武器表：按 techno `Primary`/`Secondary` 与超武 `Weapon=` 去重投影，再绑弹头 / 抛射体 id。
    for tt in rules.techno_types.iter() {
        for (key, damage, range, rof, warhead, projectile) in [
            (
                tt.primary.as_str().to_string(),
                tt.damage,
                tt.range,
                tt.rof,
                tt.warhead.clone(),
                tt.projectile.clone(),
            ),
            (
                tt.secondary.as_str().to_string(),
                tt.secondary_damage,
                tt.secondary_range,
                tt.secondary_rof,
                tt.secondary_warhead.clone(),
                tt.secondary_projectile.clone(),
            ),
        ] {
            if key.is_empty() || defs.weapons.get(&key).is_some() {
                continue;
            }
            let id = alloc_weapon();
            defs.weapons.insert(WeaponDefinition {
                id,
                type_key: key,
                damage,
                range,
                rof,
                warhead,
                warhead_id: WarheadId(0),
                projectile,
                projectile_id: ProjectileId(0),
            });
        }
    }
    for sw in rules.super_weapons.iter() {
        let key = sw.weapon.as_str().to_string();
        if key.is_empty() || defs.weapons.get(&key).is_some() {
            continue;
        }
        let id = alloc_weapon();
        defs.weapons.insert(WeaponDefinition {
            id,
            type_key: key,
            damage: sw.weapon_damage,
            range: sw.weapon_range,
            rof: sw.weapon_rof,
            warhead: sw.weapon_warhead.clone(),
            warhead_id: WarheadId(0),
            projectile: sw.weapon_projectile.clone(),
            projectile_id: ProjectileId(0),
        });
    }

    let mut warhead_keys: Vec<WarheadName> = defs
        .weapons
        .iter()
        .map(|w| w.warhead.clone())
        .chain(defs.techno.iter().map(|t| t.warhead.clone()))
        .chain(rules.techno_types.iter().map(|t| t.secondary_warhead.clone()))
        .filter(|w| !w.is_empty())
        .collect();
    warhead_keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    warhead_keys.dedup();
    for key in warhead_keys {
        let loaded = rules.warheads.get(key.as_str());
        let verses = loaded.map(|w| w.verses).unwrap_or_default();
        let spread = loaded.map(|w| w.spread).unwrap_or(0);
        let prone_damage = loaded.map(|w| w.prone_damage).unwrap_or(100);
        let id = alloc_warhead();
        defs.warheads.insert(WarheadDefinition {
            id,
            type_key: key.as_str().to_string(),
            verses,
            spread,
            prone_damage,
        });
    }

    let mut projectile_keys: Vec<ProjectileName> = defs
        .weapons
        .iter()
        .map(|w| w.projectile.clone())
        .chain(rules.techno_types.iter().map(|t| t.projectile.clone()))
        .chain(rules.techno_types.iter().map(|t| t.secondary_projectile.clone()))
        .filter(|p| !p.is_empty())
        .collect();
    projectile_keys.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    projectile_keys.dedup();
    for key in projectile_keys {
        let id = alloc_projectile();
        defs.projectiles.insert(ProjectileDefinition {
            id,
            type_key: key.as_str().to_string(),
        });
    }

    for weapon in defs.weapons.iter_mut() {
        weapon.warhead_id = if weapon.warhead.is_empty() {
            WarheadId(0)
        } else {
            defs.warheads.get(weapon.warhead.as_str()).map(|w| w.id).unwrap_or(WarheadId(0))
        };
        weapon.projectile_id = if weapon.projectile.is_empty() {
            ProjectileId(0)
        } else {
            defs.projectiles
                .get(weapon.projectile.as_str())
                .map(|p| p.id)
                .unwrap_or(ProjectileId(0))
        };
    }
    for techno in defs.techno.iter_mut() {
        techno.primary_id = if techno.primary.is_empty() {
            WeaponId(0)
        } else {
            defs.weapons.get(techno.primary.as_str()).map(|w| w.id).unwrap_or(WeaponId(0))
        };
        techno.secondary_id = if techno.secondary.is_empty() {
            WeaponId(0)
        } else {
            defs.weapons.get(techno.secondary.as_str()).map(|w| w.id).unwrap_or(WeaponId(0))
        };
        techno.warhead_id = if let Some(w) = defs.weapons.get_by_id(techno.primary_id) {
            w.warhead_id
        } else if techno.warhead.is_empty() {
            WarheadId(0)
        } else {
            defs.warheads.get(techno.warhead.as_str()).map(|w| w.id).unwrap_or(WarheadId(0))
        };
    }
    for sw in defs.super_weapons.iter_mut() {
        sw.weapon_id = if sw.weapon.is_empty() {
            WeaponId(0)
        } else {
            defs.weapons.get(sw.weapon.as_str()).map(|w| w.id).unwrap_or(WeaponId(0))
        };
    }

    defs.terrain_spawners = rules.terrain_spawners.clone();
    defs.overlays = rules.overlay_types.clone();

    defs
}

/// 从内联 rules/art 字节直接投影冻结定义（测试 / 无资源树夹具）。
pub fn runtime_definitions_from_ini_bytes(
    edition: GameEdition,
    rules_ini: &[u8],
    art_ini: Option<&[u8]>,
) -> RaResult<RuntimeDefinitions> {
    let rules = rules_system_from_ini_bytes(edition, rules_ini, art_ini)?;
    Ok(build_runtime_definitions(&rules))
}

#[doc(hidden)]
pub fn parse_factory_category(raw: &str) -> ProductionCategory {
    ProductionCategory::parse(raw)
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
