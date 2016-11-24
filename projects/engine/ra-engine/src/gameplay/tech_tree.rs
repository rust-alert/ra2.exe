//! 科技树：按冻结定义与存活建筑判定类型是否 Eligible。
//!
//! 通用组只查 [`ra_types::PrerequisiteGroups`]，不按电力/工厂标志冒充。

use std::collections::HashSet;

use ra_map::MapEntityKind;
use ra_types::{PrerequisiteGroupKind, PrerequisiteToken, RuntimeDefinitions, TechnoClass, TechnoDefinition, TechnoName};

use crate::{
    gameplay::owner_allows,
    state::{
        BattleState, PlayerState,
        components::{Health, Identity, Owner},
    },
};

/// 科技树判定所需的玩家视图（避免把整局状态拖进纯函数）。
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct TechTreePlayer<'a> {
    /// 阵营名。
    pub house: &'a str,
    /// 科技上限。
    pub tech_level: i32,
    /// 已偷盟军科技。
    pub stolen_allied_tech: bool,
    /// 已偷苏军科技。
    pub stolen_soviet_tech: bool,
    /// 已偷第三势力科技。
    pub stolen_third_tech: bool,
}

impl<'a> TechTreePlayer<'a> {
    /// 从 [`PlayerState`] 投影。
    pub fn from_player(player: &'a PlayerState) -> Self {
        Self {
            house: player.house.as_ref(),
            tech_level: player.tech_level,
            stolen_allied_tech: player.stolen_allied_tech,
            stolen_soviet_tech: player.stolen_soviet_tech,
            stolen_third_tech: player.stolen_third_tech,
        }
    }
}

/// 收集某 house 当前存活建筑的类型键。
pub fn living_structure_keys(world: &BattleState, house: &str) -> HashSet<TechnoName> {
    let mut keys = HashSet::new();
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if world.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != house) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if identity.kind != MapEntityKind::Structure {
            continue;
        }
        keys.insert(TechnoName::parse(identity.type_id.as_ref()));
    }
    keys
}

/// 某 house 存活的指定类型数量（建筑与单位都计，供 BuildLimit）。
pub fn living_type_count(world: &BattleState, house: &str, type_key: &TechnoName) -> i32 {
    world
        .entities
        .iter()
        .filter(|e| {
            let id = e.id;
            !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && world.ecs_get::<Owner>(id).is_some_and(|o| o.house.eq_ignore_ascii_case(house))
                && world.ecs_get::<Identity>(id).is_some_and(|i| TechnoName::parse(i.type_id.as_ref()) == *type_key)
        })
        .count() as i32
}

/// 本阵营是否已达 BuildLimit（`build_limit == 0` 表示不限）。
pub fn build_limit_reached(world: &BattleState, house: &str, techno: &TechnoDefinition) -> bool {
    techno.build_limit > 0 && living_type_count(world, house, &techno.type_key) >= techno.build_limit
}

#[doc(hidden)]
pub fn owns_any(living: &HashSet<TechnoName>, types: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    types.into_iter().any(|t| living.contains(&TechnoName::parse(t.as_ref())))
}

#[doc(hidden)]
pub fn token_satisfied(defs: &RuntimeDefinitions, living: &HashSet<TechnoName>, token: &PrerequisiteToken) -> bool {
    match token {
        PrerequisiteToken::Group(PrerequisiteGroupKind::Proc) => owns_any(living, defs.prerequisite_groups.proc_all()),
        PrerequisiteToken::Group(kind) => owns_any(living, defs.prerequisite_groups.types_for_kind(*kind)),
        PrerequisiteToken::Type(id) => defs.techno.get_by_id(*id).is_some_and(|t| living.contains(&t.type_key)),
        PrerequisiteToken::UnboundType(key) => {
            if key.is_empty() {
                true
            } else {
                living.contains(key)
            }
        }
    }
}

#[doc(hidden)]
pub fn prerequisites_met(defs: &RuntimeDefinitions, living: &HashSet<TechnoName>, techno: &TechnoDefinition) -> bool {
    if !techno.prerequisite_override.is_empty() && techno.prerequisite_override.iter().any(|t| token_satisfied(defs, living, t)) {
        return true;
    }
    techno.prerequisite.iter().all(|t| token_satisfied(defs, living, t))
}

/// 类型是否对玩家 Eligible（可出现在建造/生产栏；不含资金与电力运作门槛）。
pub fn is_type_eligible(defs: &RuntimeDefinitions, player: TechTreePlayer<'_>, living: &HashSet<TechnoName>, type_key: &str) -> bool {
    let Some(techno) = defs.techno.get(type_key)
    else {
        return false;
    };
    if techno.tech_level < 0 || techno.tech_level > player.tech_level {
        return false;
    }
    if !owner_allows(&techno.owner, player.house) {
        return false;
    }
    if !techno.required_houses.required_allows(player.house) {
        return false;
    }
    if techno.forbidden_houses.forbids(player.house) {
        return false;
    }
    if techno.requires_stolen_allied_tech && !player.stolen_allied_tech {
        return false;
    }
    if techno.requires_stolen_soviet_tech && !player.stolen_soviet_tech {
        return false;
    }
    if techno.requires_stolen_third_tech && !player.stolen_third_tech {
        return false;
    }
    if techno.class == TechnoClass::Building {
        if defs.structures.get_name(&techno.type_key).is_some_and(|s| s.construction_yard) {
            return false;
        }
    }
    prerequisites_met(defs, living, techno)
}
