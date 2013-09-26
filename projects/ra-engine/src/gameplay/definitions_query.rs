//! 对冻结定义的玩法查询（无外部内容名分支）。

use ra_types::{ProductionCategory, RuntimeDefinitions, TechnoClass};

/// 建筑是否建造场。
pub(crate) fn is_construction_yard(defs: &RuntimeDefinitions, type_id: &str) -> bool {
    defs.structures.get(type_id).is_some_and(|s| s.construction_yard)
}

/// 建筑是否供电站（`power.output > 0`）。
pub(crate) fn is_power_plant(defs: &RuntimeDefinitions, type_id: &str) -> bool {
    defs.structures.get(type_id).is_some_and(|s| s.power.output > 0)
}

/// 建筑是否声明需电前置。
pub(crate) fn requires_power_plant(defs: &RuntimeDefinitions, type_id: &str) -> bool {
    defs.structures.get(type_id).is_some_and(|s| s.power.requires_power)
}

/// 建筑是否矿场。
pub(crate) fn is_refinery(defs: &RuntimeDefinitions, type_id: &str) -> bool {
    defs.structures.get(type_id).is_some_and(|s| s.refinery)
}

/// 是否生产工厂。
pub(crate) fn is_production_factory(defs: &RuntimeDefinitions, type_id: &str) -> bool {
    defs.structures.get(type_id).is_some_and(|s| s.production.is_some())
}

/// 工厂是否可生产给定 techno 大类。
pub(crate) fn factory_matches_unit(defs: &RuntimeDefinitions, factory_type: &str, class: TechnoClass) -> bool {
    let Some(cat) = class.production_category()
    else {
        return false;
    };
    defs.structures
        .get(factory_type)
        .and_then(|s| s.production.as_ref())
        .is_some_and(|p| p.category == cat)
}

/// 工厂是否可生产给定生产类别。
pub(crate) fn factory_matches_category(defs: &RuntimeDefinitions, factory_type: &str, category: ProductionCategory) -> bool {
    defs.structures
        .get(factory_type)
        .and_then(|s| s.production.as_ref())
        .is_some_and(|p| p.category == category)
}

/// 电力增量（供电 / 耗电）；未知类型视为 0。
pub(crate) fn building_power(defs: &RuntimeDefinitions, type_id: &str) -> PowerProfileOrZero {
    match defs.structures.get(type_id) {
        Some(s) => PowerProfileOrZero {
            output: s.power.output,
            drain: s.power.drain,
        },
        None => PowerProfileOrZero { output: 0, drain: 0 },
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PowerProfileOrZero {
    pub output: i32,
    pub drain: i32,
}

/// 部署目标类型键。
pub(crate) fn deploy_into_type<'a>(defs: &'a RuntimeDefinitions, source_type: &str) -> Option<&'a str> {
    defs.deployables.get(source_type).map(|d| d.target_key.as_str())
}

/// Owner 串是否允许该阵营使用（空 Owner = 不限）。
pub(crate) fn owner_allows(owner_field: &str, house: &str) -> bool {
    let owner_field = owner_field.trim();
    if owner_field.is_empty() {
        return true;
    }
    owner_field
        .split(|c| c == ',' || c == ';' || c == '|')
        .map(str::trim)
        .any(|p| p.eq_ignore_ascii_case(house))
}
