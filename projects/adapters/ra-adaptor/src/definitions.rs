//! 从 `RulesSystem` / INI 解释生成冻结 [`RuntimeDefinitions`]。
//!
//! 引擎不得再按外部类型名猜测电力、工厂、部署关系。

use ra_assets::TechnoKind;
use ra_types::{
    BuiltinCapability, CapabilityGapReport, CrateRules, DeployableDefinition, DeploymentPlacement, GameEdition, HouseAllowList,
    HouseDefinition, HouseId, HouseName, HouseRole, InfiltrationRules, LightningStormRules, PowerProfile, PrerequisiteGroups,
    ProductionCategory, ProductionProfile, ProjectileDefinition, ProjectileId, ProjectileName, RaError, RaResult, RuntimeDefinitions,
    StolenTechKind, StructureDefinition, StructureLightProfile, SuperWeaponDefinition, TechnoClass, TechnoDefinition, TechnoName, TypeId,
    WarheadDefinition, WarheadId, WarheadName, WeaponDefinition, WeaponId, WeaponName, super_weapon_kind_has_executor,
};
use std::collections::HashMap;

use crate::{RulesSystem, rules_system_from_ini_bytes};

/// 由已装载规则快照构建冻结运行时定义。
///
/// 空名称可选引用绑定为 `None`。非空名称必须能解析到表项，否则返回 [`RaError::UnknownReference`]。
pub fn build_runtime_definitions(rules: &RulesSystem) -> RaResult<RuntimeDefinitions> {
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

    let mut next_house = 1u32;
    let mut alloc_house = || {
        let id = HouseId(next_house);
        next_house = next_house.saturating_add(1);
        id
    };

    let g = &rules.globals;
    let prereq_power = g.prerequisite_power.clone();
    let prereq_factory = g.prerequisite_factory.clone();
    let prereq_barracks = g.prerequisite_barracks.clone();
    let prereq_radar = g.prerequisite_radar.clone();
    let prereq_tech = g.prerequisite_tech.clone();
    let prereq_proc = g.prerequisite_proc.clone();
    let prereq_proc_alternate = g.prerequisite_proc_alternate.clone();
    let base_unit_names = g.base_unit.clone();
    defs.default_tech_level = g.multiplayer_tech_level.unwrap_or(10).max(0);
    // `[General]` 侧栏扳手：缺键回落原版库存默认。
    defs.repair_percent = g.repair_percent.map(|v| v.max(0) as u32).unwrap_or(15);
    defs.refund_percent = g.refund_percent.map(|v| v.max(0) as u32).unwrap_or(50);
    defs.repair_step = g.repair_step.map(|v| v.max(1) as u32).unwrap_or(8);
    defs.repair_interval_ticks = g.repair_rate_minutes.map(repair_rate_minutes_to_ticks).unwrap_or(14);
    defs.speak_delay_ticks = speak_delay_minutes_to_ticks(g.speak_delay_minutes.unwrap_or(0.0));
    // 原版构造缺省 0.03 分钟；rules 显式键覆盖。
    defs.savour_delay_ticks = speak_delay_minutes_to_ticks(g.savour_delay_minutes.unwrap_or(0.03));
    defs.ai_base_spacing = g.ai_base_spacing.map(|v| v.max(0) as u32).unwrap_or(ra_types::DEFAULT_AI_BASE_SPACING);
    defs.ai_naval_yard_adjacency = g.ai_naval_yard_adjacency.map(|v| v.max(0) as u32).unwrap_or(ra_types::DEFAULT_AI_NAVAL_YARD_ADJACENCY);
    defs.lightning_storm = LightningStormRules {
        duration_ticks: g.lightning_storm_duration.map(|v| v.max(0) as u32).unwrap_or(LightningStormRules::default().duration_ticks),
        deferment_ticks: g.lightning_deferment.map(|v| v.max(0) as u32).unwrap_or(LightningStormRules::default().deferment_ticks),
    };
    defs.crate_rules = CrateRules {
        default_credits: g.crate_money.unwrap_or(CrateRules::default().default_credits).max(0),
        money_minimum: g.crate_minimum.map(|v| v.max(0)),
        money_maximum: g.crate_maximum.map(|v| v.max(0)),
    };
    // 渗透数值：当前 rules 无独立键时保持零售缺省；后续 adaptor 扩展可覆盖。
    defs.infiltration = InfiltrationRules::default();
    for country in rules.countries.countries() {
        let stolen_tech = StolenTechKind::from_side(&country.side);
        let id = alloc_house();
        if let Some(kind) = stolen_tech {
            defs.stolen_tech_by_house.insert(id, kind);
        }
        let role = HouseRole::from_stock_ambient_name(country.id.as_str()).unwrap_or(HouseRole::Playable);
        defs.houses.insert(HouseDefinition {
            id,
            type_key: country.id.clone(),
            side: country.side.clone(),
            stolen_tech,
            multiplay: country.visible_in_skirmish(),
            role,
        });
    }
    // 氛围房屋：规则 `[Countries]` 通常不列，但对局 / 地图 Owner 仍需稳定 `HouseId`。
    // 角色由 adaptor 原版兼容默认写入；engine 只认 `HouseRole`。
    for (ambient, role) in [("NEUTRAL", HouseRole::Neutral), ("SPECIAL", HouseRole::Special), ("CIVILIAN", HouseRole::Civilian)] {
        let type_key = HouseName::parse(ambient);
        if defs.houses.get_name(&type_key).is_some() {
            continue;
        }
        let id = alloc_house();
        defs.houses.insert(HouseDefinition {
            id,
            type_key,
            side: ra_types::SideName::default(),
            stolen_tech: None,
            multiplay: false,
            role,
        });
    }

    for sw in rules.super_weapons.iter() {
        let id = alloc();
        let def = SuperWeaponDefinition {
            id,
            type_key: sw.id.clone(),
            ui_name: sw.ui_name.clone(),
            kind: sw.kind.clone(),
            action: sw.action.clone(),
            recharge_time: sw.recharge_time,
            sidebar_image: sw.sidebar_image.clone(),
            weapon: sw.weapon.clone(),
            weapon_id: None,
        };
        if !def.kind.is_empty() && !super_weapon_kind_has_executor(def.kind.as_str()) {
            defs.capability_gaps.push(CapabilityGapReport::new(
                format!("rules.superweapon.{} deferred", def.kind.as_str()),
                format!("超级武器 Type={}（{}）已冻结但尚无执行器", def.kind.as_str(), def.type_key.as_str()),
            ));
        }
        defs.super_weapons.insert(def);
    }
    if !defs.super_weapons.is_empty() && !defs.capabilities.builtins.contains(&BuiltinCapability::SuperWeapon) {
        defs.capabilities.builtins.push(BuiltinCapability::SuperWeapon);
    }

    let mut pending_deploys: Vec<(TypeId, TechnoName, TechnoName)> = Vec::new();
    let mut pending_undeploys: Vec<(TypeId, TechnoName, TechnoName)> = Vec::new();
    // FreeUnit 目标可能后于建筑入库（与 DeploysInto 同理），第二遍再绑 TypeId。
    let mut pending_free_units: Vec<(TechnoName, TechnoName)> = Vec::new();

    for tt in rules.techno_types.iter() {
        let key = tt.id.clone();
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
            owner_ids: ra_types::HouseIdAllowList::empty(),
            tech_level: tt.tech_level,
            naval: tt.naval,
            allowed_to_start_in_multiplayer: tt.allowed_to_start_in_multiplayer,
            agent: tt.agent,
            engineer: tt.engineer,
            harvester: tt.harvester,
            passengers: tt.passengers,
            category: tt.category,
            sight: tt.sight,
            primary: tt.primary.clone(),
            primary_id: None,
            secondary: tt.secondary.clone(),
            secondary_id: None,
            warhead: tt.warhead.clone(),
            warhead_id: None,
            prerequisite: tt.prerequisite.clone().into_vec(),
            prerequisite_override: tt.prerequisite_override.clone().into_vec(),
            required_houses: tt.required_houses.clone(),
            required_house_ids: ra_types::HouseIdAllowList::empty(),
            forbidden_houses: tt.forbidden_houses.clone(),
            forbidden_house_ids: ra_types::HouseIdAllowList::empty(),
            build_limit: tt.build_limit,
            build_time: tt.build_time,
            requires_stolen_allied_tech: tt.requires_stolen_allied_tech,
            requires_stolen_soviet_tech: tt.requires_stolen_soviet_tech,
            requires_stolen_third_tech: tt.requires_stolen_third_tech,
            required_stolen_tech: stolen_tech_requirements(
                tt.requires_stolen_allied_tech,
                tt.requires_stolen_soviet_tech,
                tt.requires_stolen_third_tech,
            ),
            pixel_selection_bracket_delta: tt.pixel_selection_bracket_delta,
            deployer: tt.deployer,
            undeploys_into_id: None,
        });

        if tt.harvester {
            if !defs.capabilities.builtins.contains(&BuiltinCapability::Harvester) {
                defs.capabilities.builtins.push(BuiltinCapability::Harvester);
            }
        }
        if tt.passengers > 0 {
            if !defs.capabilities.builtins.contains(&BuiltinCapability::Transport) {
                defs.capabilities.builtins.push(BuiltinCapability::Transport);
            }
        }

        if tt.kind != TechnoKind::Building {
            // 部署关系可挂在载具 / 步兵上；目标 TypeId 在 techno 全表入库后绑定。
            if !tt.deploys_into.is_empty() {
                pending_deploys.push((id, key.clone(), tt.deploys_into.clone()));
                if !defs.capabilities.builtins.contains(&BuiltinCapability::Deployable) {
                    defs.capabilities.builtins.push(BuiltinCapability::Deployable);
                }
            }
            if tt.deployer {
                if !defs.capabilities.builtins.contains(&BuiltinCapability::Deployable) {
                    defs.capabilities.builtins.push(BuiltinCapability::Deployable);
                }
            }
            if !tt.undeploys_into.is_empty() {
                pending_undeploys.push((id, key.clone(), tt.undeploys_into.clone()));
                if !defs.capabilities.builtins.contains(&BuiltinCapability::Deployable) {
                    defs.capabilities.builtins.push(BuiltinCapability::Deployable);
                }
            }
            continue;
        }

        let power_raw = tt.power;
        let (output, drain) = if power_raw >= 0 { (power_raw, 0) } else { (0, -power_raw) };
        let powered = tt.powered.unwrap_or(drain > 0);
        let construction_yard = tt.construction_yard;
        let refinery = tt.refinery;
        let wall = tt.wall;
        let base_normal = tt.base_normal;
        let adjacent = tt.adjacent;
        let guard_range = tt.guard_range;
        let wants_extra_space = tt.wants_extra_space;
        if !tt.free_unit.is_empty() {
            pending_free_units.push((key.clone(), tt.free_unit.clone()));
        }
        let radar = tt.radar;
        let build_cat = tt.build_cat;
        let capturable = tt.capturable;
        let unsellable = tt.unsellable;
        let soylent = tt.soylent;
        let water_bound = tt.water_bound;
        let produce_cash =
            ra_types::ProduceCashProfile { startup: tt.produce_cash_startup, amount: tt.produce_cash_amount, delay: tt.produce_cash_delay };
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
        if !unsellable {
            capabilities.push(BuiltinCapability::Sellable);
        }
        if produce_cash.ticks_income() || produce_cash.startup > 0 {
            capabilities.push(BuiltinCapability::CashProducer);
        }
        if production.is_some() {
            capabilities.push(BuiltinCapability::Producer);
        }
        let super_weapon = if tt.super_weapon.is_empty() { None } else { Some(tt.super_weapon.clone()) };
        let super_weapon_id = match &super_weapon {
            None => None,
            Some(k) => Some(defs.super_weapons.get(k.as_str()).map(|sw| sw.id).ok_or_else(|| RaError::UnknownReference {
                kind: "super_weapon",
                name: k.as_str().to_string(),
                owner: key.as_str().to_string(),
            })?),
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
        let foundation = tt.foundation.clone();
        let height = tt.height.unwrap_or(2).max(1);
        let light =
            StructureLightProfile::from_rules_floats(tt.light_intensity, tt.light_visibility, tt.light_red, tt.light_green, tt.light_blue);
        defs.structures.insert(StructureDefinition {
            id,
            type_key: key,
            power: PowerProfile { output, drain, requires_power: powered },
            cost: tt.cost as i32,
            strength: tt.strength.max(1),
            armor: tt.armor,
            construction_yard,
            refinery,
            wall,
            base_normal,
            adjacent,
            guard_range,
            wants_extra_space,
            free_unit: None,
            radar,
            build_cat,
            capturable,
            unsellable,
            soylent,
            produce_cash,
            water_bound,
            production,
            owner: tt.owner.clone(),
            owner_ids: ra_types::HouseIdAllowList::empty(),
            foundation,
            height,
            super_weapon,
            super_weapon_id,
            light,
            capabilities,
        });
    }

    // 第二遍：绑定 DeploysInto 目标 TypeId（目标可能后于源解析）；未知目标为装载错误。
    for (source, source_key, target_key) in pending_deploys {
        let Some(t) = defs.techno.get_name(&target_key)
        else {
            return Err(RaError::UnknownReference {
                kind: "techno",
                name: target_key.as_str().to_string(),
                owner: format!("DeploysInto:{}", source_key.as_str()),
            });
        };
        defs.deployables.insert(DeployableDefinition { source, source_key, target: t.id, target_key, placement: DeploymentPlacement::InPlace });
    }

    // 第二遍：绑定 UndeploysInto 目标 TypeId。
    for (source, source_key, target_key) in pending_undeploys {
        let Some(t) = defs.techno.get_name(&target_key)
        else {
            return Err(RaError::UnknownReference {
                kind: "techno",
                name: target_key.as_str().to_string(),
                owner: format!("UndeploysInto:{}", source_key.as_str()),
            });
        };
        let target_id = t.id;
        let Some(src) = defs.techno.iter_mut().find(|tt| tt.id == source)
        else {
            return Err(RaError::UnknownReference {
                kind: "techno",
                name: source_key.as_str().to_string(),
                owner: format!("UndeploysInto:{}", target_key.as_str()),
            });
        };
        src.undeploys_into_id = Some(target_id);
    }

    // 第二遍：绑定建筑 `FreeUnit=` → 单位 TypeId；未知目标为装载错误。
    for (structure_key, free_unit_key) in pending_free_units {
        let Some(unit) = defs.techno.get_name(&free_unit_key)
        else {
            return Err(RaError::UnknownReference {
                kind: "techno",
                name: free_unit_key.as_str().to_string(),
                owner: format!("FreeUnit:{}", structure_key.as_str()),
            });
        };
        let unit_id = unit.id;
        let Some(structure) = defs.structures.iter_mut().find(|s| s.type_key == structure_key)
        else {
            return Err(RaError::UnknownReference {
                kind: "structure",
                name: structure_key.as_str().to_string(),
                owner: format!("FreeUnit:{}", free_unit_key.as_str()),
            });
        };
        structure.free_unit = Some(unit_id);
    }

    // 前置 token：UnboundType → TypeId；仍未绑定则为装载错误（禁止靠名称在引擎里兜底）。
    let type_ids: HashMap<TechnoName, TypeId> = defs.techno.iter().map(|t| (t.type_key.clone(), t.id)).collect();
    let resolve = |key: &TechnoName| type_ids.get(key).copied();
    for techno in defs.techno.iter_mut() {
        let key = techno.type_key.clone();
        techno.prerequisite = bind_prerequisite_tokens(std::mem::take(&mut techno.prerequisite), &resolve, &key, "Prerequisite")?;
        techno.prerequisite_override =
            bind_prerequisite_tokens(std::mem::take(&mut techno.prerequisite_override), &resolve, &key, "PrerequisiteOverride")?;
    }

    defs.prerequisite_groups = PrerequisiteGroups {
        power: bind_techno_name_list(&defs, &prereq_power, "PrerequisiteGroups:PrerequisitePower")?,
        factory: bind_techno_name_list(&defs, &prereq_factory, "PrerequisiteGroups:PrerequisiteFactory")?,
        barracks: bind_techno_name_list(&defs, &prereq_barracks, "PrerequisiteGroups:PrerequisiteBarracks")?,
        radar: bind_techno_name_list(&defs, &prereq_radar, "PrerequisiteGroups:PrerequisiteRadar")?,
        tech: bind_techno_name_list(&defs, &prereq_tech, "PrerequisiteGroups:PrerequisiteTech")?,
        proc: bind_techno_name_list(&defs, &prereq_proc, "PrerequisiteGroups:PrerequisiteProc")?,
        proc_alternate: bind_techno_name_list(&defs, &prereq_proc_alternate, "PrerequisiteGroups:PrerequisiteProcAlternate")?,
    };
    defs.base_units = bind_techno_name_list(&defs, &base_unit_names, "General:BaseUnit")?;
    // `[AI] Build*` 须在 techno 入库后绑定；未知名软跳过。
    defs.ai_controls = ra_types::AiControls {
        build_const: soft_bind_techno_name_list(&defs, &g.ai_build_const),
        build_power: soft_bind_techno_name_list(&defs, &g.ai_build_power),
        power_surplus: g.ai_power_surplus.unwrap_or(50),
        build_refinery: ra_types::AiBuildCategory {
            candidates: soft_bind_techno_name_list(&defs, &g.ai_build_refinery),
            ratio_millis: ratio_to_millis(g.ai_refinery_ratio),
            limit: g.ai_refinery_limit.map(|v| v.max(0) as u32).unwrap_or(0),
        },
        build_barracks: ra_types::AiBuildCategory {
            candidates: soft_bind_techno_name_list(&defs, &g.ai_build_barracks),
            ratio_millis: ratio_to_millis(g.ai_barracks_ratio),
            limit: g.ai_barracks_limit.map(|v| v.max(0) as u32).unwrap_or(0),
        },
        build_weapons: ra_types::AiBuildCategory {
            candidates: soft_bind_techno_name_list(&defs, &g.ai_build_weapons),
            ratio_millis: ratio_to_millis(g.ai_war_ratio),
            limit: g.ai_war_limit.map(|v| v.max(0) as u32).unwrap_or(0),
        },
        build_radar: soft_bind_techno_name_list(&defs, &g.ai_build_radar),
        build_tech: soft_bind_techno_name_list(&defs, &g.ai_build_tech),
        build_naval_yard: soft_bind_techno_name_list(&defs, &g.ai_build_naval_yard),
        build_helipad: ra_types::AiBuildCategory {
            candidates: soft_bind_techno_name_list(&defs, &g.ai_build_helipad),
            ratio_millis: ratio_to_millis(g.ai_helipad_ratio),
            limit: g.ai_helipad_limit.map(|v| v.max(0) as u32).unwrap_or(0),
        },
        build_defense: ra_types::AiBuildCategory {
            candidates: soft_bind_techno_name_list(&defs, &g.ai_build_defense),
            ratio_millis: ratio_to_millis(g.ai_defense_ratio),
            limit: g.ai_defense_limit.map(|v| v.max(0) as u32).unwrap_or(0),
        },
        build_aa: ra_types::AiBuildCategory {
            candidates: soft_bind_techno_name_list(&defs, &g.ai_build_aa),
            ratio_millis: ratio_to_millis(g.ai_aa_ratio),
            limit: g.ai_aa_limit.map(|v| v.max(0) as u32).unwrap_or(0),
        },
        build_dummy: soft_bind_techno_name_list(&defs, &g.ai_build_dummy),
        base_spacing: defs.ai_base_spacing,
        base_size_add: g.ai_base_size_add.map(|v| v.max(0) as u32).unwrap_or(0),
        max_iq_levels: g.iq_max_levels.unwrap_or(5).max(0),
        iq_production: g.iq_production.unwrap_or(5).max(0),
        // 缺省对齐零售 rules `[IQ]` 常见取值；引擎只比较，不内置玩法字面量。
        iq_super_weapons: g.iq_super_weapons.unwrap_or(4).max(0),
        iq_guard_area: g.iq_guard_area.unwrap_or(2).max(0),
        iq_repair_sell: g.iq_repair_sell.unwrap_or(1).max(0),
        iq_auto_crush: g.iq_auto_crush.unwrap_or(2).max(0),
        iq_scatter: g.iq_scatter.unwrap_or(2).max(0),
        iq_content_scan: g.iq_content_scan.unwrap_or(3).max(0),
        iq_aircraft: g.iq_aircraft.unwrap_or(3).max(0),
        iq_harvester: g.iq_harvester.unwrap_or(2).max(0),
        iq_sell_back: g.iq_sell_back.unwrap_or(2).max(0),
    };

    defs.production.count = defs.structures.iter().filter(|s| s.production.is_some()).count() as u32;

    // 武器表：按 techno `Primary`/`Secondary` 与超武 `Weapon=` 去重投影，再绑弹头 / 抛射体 id。
    for tt in rules.techno_types.iter() {
        for (key, damage, range, rof, warhead, projectile, report) in [
            (tt.primary.clone(), tt.damage, tt.range, tt.rof, tt.warhead.clone(), tt.projectile.clone(), tt.report.clone()),
            (
                tt.secondary.clone(),
                tt.secondary_damage,
                tt.secondary_range,
                tt.secondary_rof,
                tt.secondary_warhead.clone(),
                tt.secondary_projectile.clone(),
                tt.secondary_report.clone(),
            ),
        ] {
            if key.is_empty() || defs.weapons.get_name(&key).is_some() {
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
                warhead_id: None,
                projectile,
                projectile_id: None,
                report,
            });
        }
    }
    for sw in rules.super_weapons.iter() {
        let key = sw.weapon.clone();
        if key.is_empty() || defs.weapons.get_name(&key).is_some() {
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
            warhead_id: None,
            projectile: sw.weapon_projectile.clone(),
            projectile_id: None,
            report: sw.weapon_report.clone(),
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
        let Some(loaded) = rules.warheads.get_name(&key)
        else {
            return Err(RaError::UnknownReference { kind: "warhead", name: key.as_str().to_string(), owner: "RuntimeDefinitions".into() });
        };
        let id = alloc_warhead();
        defs.warheads.insert(WarheadDefinition {
            id,
            type_key: key,
            verses: loaded.verses,
            spread: loaded.spread,
            prone_damage: loaded.prone_damage,
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
        defs.projectiles.insert(ProjectileDefinition { id, type_key: key });
    }

    let weapon_binds: Vec<(WeaponId, Option<WarheadId>, Option<ProjectileId>)> = defs
        .weapons
        .iter()
        .map(|weapon| {
            Ok((
                weapon.id,
                bind_warhead_id(&defs, &weapon.warhead, weapon.type_key.as_str())?,
                bind_projectile_id(&defs, &weapon.projectile, weapon.type_key.as_str())?,
            ))
        })
        .collect::<RaResult<_>>()?;
    for (id, warhead_id, projectile_id) in weapon_binds {
        if let Some(weapon) = defs.weapons.iter_mut().find(|w| w.id == id) {
            weapon.warhead_id = warhead_id;
            weapon.projectile_id = projectile_id;
            if warhead_id.is_some() {
                weapon.warhead = WarheadName::default();
            }
            if projectile_id.is_some() {
                weapon.projectile = ProjectileName::default();
            }
        }
    }

    // `[Countries]` 非空时校验 Owner / RequiredHouses / ForbiddenHouses；空表（测试夹具）跳过。
    for techno in defs.techno.iter() {
        validate_house_allow_list(&defs, &techno.owner, "Owner", techno.type_key.as_str())?;
        validate_house_allow_list(&defs, &techno.required_houses, "RequiredHouses", techno.type_key.as_str())?;
        validate_house_allow_list(&defs, &techno.forbidden_houses, "ForbiddenHouses", techno.type_key.as_str())?;
    }
    for structure in defs.structures.iter() {
        validate_house_allow_list(&defs, &structure.owner, "Owner", structure.type_key.as_str())?;
    }

    let house_binds: Vec<(TypeId, ra_types::HouseIdAllowList, ra_types::HouseIdAllowList, ra_types::HouseIdAllowList)> = defs
        .techno
        .iter()
        .map(|techno| {
            Ok((
                techno.id,
                bind_house_allow_list(&defs, &techno.owner, "Owner", techno.type_key.as_str())?,
                bind_house_allow_list(&defs, &techno.required_houses, "RequiredHouses", techno.type_key.as_str())?,
                bind_house_allow_list(&defs, &techno.forbidden_houses, "ForbiddenHouses", techno.type_key.as_str())?,
            ))
        })
        .collect::<RaResult<_>>()?;
    let clear_house_names = has_rule_countries(&defs);
    for (id, owner_ids, required_house_ids, forbidden_house_ids) in house_binds {
        if let Some(techno) = defs.techno.iter_mut().find(|t| t.id == id) {
            techno.owner_ids = owner_ids;
            techno.required_house_ids = required_house_ids;
            techno.forbidden_house_ids = forbidden_house_ids;
            // 有 `[Countries]` 时清空姓名单，避免运行时再走字符串回查。
            if clear_house_names {
                techno.owner = HouseAllowList::default();
                techno.required_houses = HouseAllowList::default();
                techno.forbidden_houses = HouseAllowList::default();
            }
        }
    }
    let structure_owner_binds: Vec<(TypeId, ra_types::HouseIdAllowList)> = defs
        .structures
        .iter()
        .map(|structure| Ok((structure.id, bind_house_allow_list(&defs, &structure.owner, "Owner", structure.type_key.as_str())?)))
        .collect::<RaResult<_>>()?;
    for (id, owner_ids) in structure_owner_binds {
        if let Some(structure) = defs.structures.iter_mut().find(|s| s.id == id) {
            structure.owner_ids = owner_ids;
            if clear_house_names {
                structure.owner = HouseAllowList::default();
            }
        }
    }

    let techno_binds: Vec<(TypeId, Option<WeaponId>, Option<WeaponId>, Option<WarheadId>)> = defs
        .techno
        .iter()
        .map(|techno| {
            let primary_id = bind_weapon_id(&defs, &techno.primary, techno.type_key.as_str())?;
            let secondary_id = bind_weapon_id(&defs, &techno.secondary, techno.type_key.as_str())?;
            let warhead_id = if let Some(w) = primary_id.and_then(|id| defs.weapons.get_by_id(id)) {
                w.warhead_id
            }
            else {
                bind_warhead_id(&defs, &techno.warhead, techno.type_key.as_str())?
            };
            Ok((techno.id, primary_id, secondary_id, warhead_id))
        })
        .collect::<RaResult<_>>()?;
    for (id, primary_id, secondary_id, warhead_id) in techno_binds {
        if let Some(techno) = defs.techno.iter_mut().find(|t| t.id == id) {
            techno.primary_id = primary_id;
            techno.secondary_id = secondary_id;
            techno.warhead_id = warhead_id;
            // 绑定成功后清空 Name：运行时只走稳定 id，禁止再靠字符串回查。
            if primary_id.is_some() {
                techno.primary = WeaponName::default();
            }
            if secondary_id.is_some() {
                techno.secondary = WeaponName::default();
            }
            if warhead_id.is_some() {
                techno.warhead = WarheadName::default();
            }
        }
    }

    let sw_binds: Vec<(TypeId, Option<WeaponId>)> =
        defs.super_weapons.iter().map(|sw| Ok((sw.id, bind_weapon_id(&defs, &sw.weapon, sw.type_key.as_str())?))).collect::<RaResult<_>>()?;
    for (id, weapon_id) in sw_binds {
        if let Some(sw) = defs.super_weapons.iter_mut().find(|s| s.id == id) {
            sw.weapon_id = weapon_id;
            if weapon_id.is_some() {
                sw.weapon = WeaponName::default();
            }
        }
    }

    for structure in defs.structures.iter_mut() {
        if structure.super_weapon_id.is_some() {
            structure.super_weapon = None;
        }
    }

    defs.terrain_spawners = rules.terrain_spawners.clone();
    defs.overlays = rules.overlay_types.clone();

    Ok(defs)
}

fn bind_weapon_id(defs: &RuntimeDefinitions, name: &WeaponName, owner: &str) -> RaResult<Option<WeaponId>> {
    if name.is_empty() {
        return Ok(None);
    }
    defs.weapons.get_name(name).map(|w| Some(w.id)).ok_or_else(|| RaError::UnknownReference {
        kind: "weapon",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_warhead_id(defs: &RuntimeDefinitions, name: &WarheadName, owner: &str) -> RaResult<Option<WarheadId>> {
    if name.is_empty() {
        return Ok(None);
    }
    defs.warheads.get_name(name).map(|w| Some(w.id)).ok_or_else(|| RaError::UnknownReference {
        kind: "warhead",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_projectile_id(defs: &RuntimeDefinitions, name: &ProjectileName, owner: &str) -> RaResult<Option<ProjectileId>> {
    if name.is_empty() {
        return Ok(None);
    }
    defs.projectiles.get_name(name).map(|p| Some(p.id)).ok_or_else(|| RaError::UnknownReference {
        kind: "projectile",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_prerequisite_tokens(
    tokens: Vec<ra_types::PrerequisiteToken>,
    resolve: &impl Fn(&TechnoName) -> Option<TypeId>,
    owner: &TechnoName,
    field: &str,
) -> RaResult<Vec<ra_types::PrerequisiteToken>> {
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        let bound = token.bind_type_id(resolve);
        if let ra_types::PrerequisiteToken::UnboundType(ref key) = bound {
            return Err(RaError::UnknownReference {
                kind: "techno",
                name: key.as_str().to_string(),
                owner: format!("{field}:{}", owner.as_str()),
            });
        }
        out.push(bound);
    }
    Ok(out)
}

fn bind_techno_name_list(defs: &RuntimeDefinitions, names: &[TechnoName], owner: &str) -> RaResult<Vec<TypeId>> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        if name.is_empty() {
            continue;
        }
        let Some(t) = defs.techno.get_name(name)
        else {
            return Err(RaError::UnknownReference { kind: "techno", name: name.as_str().to_string(), owner: owner.to_string() });
        };
        out.push(t.id);
    }
    Ok(out)
}

/// `[AI] Build*` 姓名单：未知名软跳过（不拖垮整表装载）。
fn soft_bind_techno_name_list(defs: &RuntimeDefinitions, names: &[TechnoName]) -> Vec<TypeId> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        if name.is_empty() {
            continue;
        }
        if let Some(t) = defs.techno.get_name(name) {
            out.push(t.id);
        }
    }
    out
}

/// rules 浮点比例 → 千分比整数；非法 / 缺省为 0。
fn ratio_to_millis(ratio: Option<f64>) -> u32 {
    let Some(r) = ratio
    else {
        return 0;
    };
    if !r.is_finite() || r <= 0.0 {
        return 0;
    }
    (r * 1000.0).round().clamp(0.0, 1_000_000.0) as u32
}

/// 氛围房屋：可不在 `[Countries]` 出现，但仍可写在 `Owner=` 等名单中。
fn is_ambient_house(defs: &RuntimeDefinitions, name: &HouseName) -> bool {
    if let Some(h) = defs.houses.get_name(name) {
        return h.role.is_ambient();
    }
    HouseRole::from_stock_ambient_name(name.as_str()).is_some()
}

fn has_rule_countries(defs: &RuntimeDefinitions) -> bool {
    defs.houses.iter().any(|h| !h.role.is_ambient())
}

fn validate_house_allow_list(defs: &RuntimeDefinitions, list: &HouseAllowList, field: &str, owner: &str) -> RaResult<()> {
    // 仅有氛围房屋、尚无 `[Countries]` 投影时，跳过 Owner 名单强制校验（测试夹具）。
    if !has_rule_countries(defs) {
        return Ok(());
    }
    for name in list.iter() {
        if is_ambient_house(defs, name) || defs.houses.get_name(name).is_some() {
            continue;
        }
        return Err(RaError::UnknownReference { kind: "house", name: name.as_str().to_string(), owner: format!("{field}:{owner}") });
    }
    Ok(())
}

fn stolen_tech_requirements(allied: bool, soviet: bool, third: bool) -> Vec<StolenTechKind> {
    let mut out = Vec::new();
    if allied {
        out.push(StolenTechKind::Allied);
    }
    if soviet {
        out.push(StolenTechKind::Soviet);
    }
    if third {
        out.push(StolenTechKind::Third);
    }
    out
}

/// 将姓名单绑成稳定 id 名单。无 `[Countries]` 时保持空 id（测试夹具仍可读名名单）。
fn bind_house_allow_list(defs: &RuntimeDefinitions, list: &HouseAllowList, field: &str, owner: &str) -> RaResult<ra_types::HouseIdAllowList> {
    if list.is_empty() {
        return Ok(ra_types::HouseIdAllowList::empty());
    }
    if !has_rule_countries(defs) {
        return Ok(ra_types::HouseIdAllowList::empty());
    }
    let mut ids = Vec::with_capacity(list.len());
    for name in list.iter() {
        let Some(house) = defs.houses.get_name(name)
        else {
            return Err(RaError::UnknownReference { kind: "house", name: name.as_str().to_string(), owner: format!("{field}:{owner}") });
        };
        ids.push(house.id);
    }
    Ok(ra_types::HouseIdAllowList::from_ids(ids))
}

/// 从内联 rules/art 字节直接投影冻结定义（测试 / 无资源树夹具）。
pub fn runtime_definitions_from_ini_bytes(edition: GameEdition, rules_ini: &[u8], art_ini: Option<&[u8]>) -> RaResult<RuntimeDefinitions> {
    let rules = rules_system_from_ini_bytes(edition, rules_ini, art_ini)?;
    build_runtime_definitions(&rules)
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
