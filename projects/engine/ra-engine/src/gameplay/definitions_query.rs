//! 对冻结定义的玩法查询（无外部内容名分支）。

use ra_types::{HouseId, ProductionCategory, RuntimeDefinitions, TechnoClass, TypeId};

/// 由稳定 [`HouseId`] 取房主键；未知 id 返回空串。
pub(crate) fn house_key_of(defs: &RuntimeDefinitions, id: HouseId) -> &str {
    defs.houses.get_by_id(id).map(|h| h.type_key.as_str()).unwrap_or("")
}

/// 由房主键解析稳定 [`HouseId`]。
pub(crate) fn house_id_of(defs: &RuntimeDefinitions, key: &str) -> Option<HouseId> {
    defs.houses.get(key).map(|h| h.id)
}

/// 由稳定 [`TypeId`] 取类型键；未知 id 返回空串。
pub(crate) fn type_key_of(defs: &RuntimeDefinitions, id: TypeId) -> &str {
    defs.techno.get_by_id(id).map(|t| t.type_key.as_str()).unwrap_or("")
}

/// 由类型键解析稳定 [`TypeId`]。
pub(crate) fn type_id_of(defs: &RuntimeDefinitions, key: &str) -> Option<TypeId> {
    defs.techno.get(key).map(|t| t.id)
}

/// 由超武类型键解析稳定 [`TypeId`]。
pub(crate) fn super_weapon_id_of(defs: &RuntimeDefinitions, key: &str) -> Option<TypeId> {
    defs.super_weapons.get(key).map(|sw| sw.id)
}

/// 单位是否间谍类（`Agent=yes`），可渗透敌方建筑。
pub(crate) fn is_agent(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.techno.get_by_id(type_id).is_some_and(|t| t.agent)
}

/// 单位是否工程师（`Engineer=yes`），可占领敌方可俘建筑。
pub(crate) fn is_engineer(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.techno.get_by_id(type_id).is_some_and(|t| t.engineer)
}

/// 建筑是否可被工程师占领（`Capturable=yes`）。
pub(crate) fn is_capturable(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.structures.get_by_id(type_id).is_some_and(|s| s.capturable)
}

/// 单位是否采矿车（`Harvester=yes`）。
pub(crate) fn is_harvester(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.techno.get_by_id(type_id).is_some_and(|t| t.harvester)
}

/// 建筑是否建造场。
pub(crate) fn is_construction_yard(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.structures.get_by_id(type_id).is_some_and(|s| s.construction_yard)
}

/// 建筑是否供电站（`power.output > 0`）。
pub(crate) fn is_power_plant(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.structures.get_by_id(type_id).is_some_and(|s| s.power.output > 0)
}

/// 建筑是否雷达（`Radar=yes`）。
pub(crate) fn is_radar(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.structures.get_by_id(type_id).is_some_and(|s| s.radar)
}

/// 建筑是否声明需电前置。
pub(crate) fn requires_power_plant(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.structures.get_by_id(type_id).is_some_and(|s| s.power.requires_power)
}

/// 建筑是否矿场。
pub(crate) fn is_refinery(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.structures.get_by_id(type_id).is_some_and(|s| s.refinery)
}

/// 是否生产工厂。
pub(crate) fn is_production_factory(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    defs.structures.get_by_id(type_id).is_some_and(|s| s.production.is_some())
}

/// 工厂是否可生产给定 techno 大类。
///
/// 建筑由建造场产出（`ConstructionYard=yes`），不依赖 `Factory=BuildingType`。
pub(crate) fn factory_matches_unit(defs: &RuntimeDefinitions, factory_type: TypeId, class: TechnoClass) -> bool {
    if class == TechnoClass::Building {
        return is_construction_yard(defs, factory_type);
    }
    let Some(cat) = class.production_category()
    else {
        return false;
    };
    defs.structures.get_by_id(factory_type).and_then(|s| s.production.as_ref()).is_some_and(|p| p.category == cat)
}

/// 工厂是否可生产给定生产类别。
pub(crate) fn factory_matches_category(defs: &RuntimeDefinitions, factory_type: TypeId, category: ProductionCategory) -> bool {
    defs.structures.get_by_id(factory_type).and_then(|s| s.production.as_ref()).is_some_and(|p| p.category == category)
}

/// 电力增量（供电 / 耗电）；未知类型视为 0。
pub(crate) fn building_power(defs: &RuntimeDefinitions, type_id: TypeId) -> PowerProfileOrZero {
    match defs.structures.get_by_id(type_id) {
        Some(s) => PowerProfileOrZero { output: s.power.output, drain: s.power.drain },
        None => PowerProfileOrZero { output: 0, drain: 0 },
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PowerProfileOrZero {
    pub output: i32,
    pub drain: i32,
}

/// 部署目标稳定 [`TypeId`]。
pub(crate) fn deploy_into_type(defs: &RuntimeDefinitions, source: TypeId) -> Option<TypeId> {
    defs.deployables.get_by_source(source).map(|d| d.target)
}

/// `Owner=` 名单是否允许该阵营使用（优先稳定 id；无 id 时回退名名单，供无 `[Countries]` 的测试夹具）。
pub(crate) fn owner_allows(defs: &RuntimeDefinitions, techno: &ra_types::TechnoDefinition, house: &str) -> bool {
    house_list_allows(defs, &techno.owner_ids, &techno.owner, house, HouseListKind::Owner)
}

fn house_list_allows(
    defs: &RuntimeDefinitions,
    ids: &ra_types::HouseIdAllowList,
    names: &ra_types::HouseAllowList,
    house: &str,
    kind: HouseListKind,
) -> bool {
    match kind {
        HouseListKind::Owner | HouseListKind::Required => {
            // 已绑定 id 时只看 id（含空 id = 不限）。姓名单仅作无 Countries 夹具回退。
            if !ids.is_empty() || names.is_empty() {
                return match defs.houses.get(house) {
                    Some(h) => ids.allows(h.id),
                    // 未知 house：空约束名单视为不限，否则拒绝。
                    None => ids.is_empty(),
                };
            }
            match kind {
                HouseListKind::Owner => names.owner_allows(house),
                HouseListKind::Required => names.required_allows(house),
                HouseListKind::Forbidden => unreachable!(),
            }
        }
        HouseListKind::Forbidden => {
            if !ids.is_empty() || names.is_empty() {
                return defs.houses.get(house).is_some_and(|h| ids.forbids(h.id));
            }
            names.forbids(house)
        }
    }
}

#[derive(Clone, Copy)]
enum HouseListKind {
    Owner,
    Required,
    Forbidden,
}

/// `RequiredHouses=` 是否允许。
pub(crate) fn required_houses_allows(defs: &RuntimeDefinitions, techno: &ra_types::TechnoDefinition, house: &str) -> bool {
    house_list_allows(defs, &techno.required_house_ids, &techno.required_houses, house, HouseListKind::Required)
}

/// `ForbiddenHouses=` 是否禁止。
pub(crate) fn forbidden_houses_forbids(defs: &RuntimeDefinitions, techno: &ra_types::TechnoDefinition, house: &str) -> bool {
    house_list_allows(defs, &techno.forbidden_house_ids, &techno.forbidden_houses, house, HouseListKind::Forbidden)
}

/// 为遭遇战开局席位挑选该 house 可用的 MCV 类型键。
///
/// 条件：`Vehicle` + `DeploysInto` 建造场 + `Owner` 允许该 house。多候选时按类型键排序取稳定第一项。
pub(crate) fn starting_mcv_type_for_house<'a>(defs: &'a RuntimeDefinitions, house: &str) -> Option<&'a str> {
    let mut keys: Vec<&str> = defs
        .deployables
        .iter()
        .filter_map(|d| {
            let techno = defs.techno.get_by_id(d.source)?;
            if techno.class != TechnoClass::Vehicle {
                return None;
            }
            if !is_construction_yard(defs, d.target) {
                return None;
            }
            if !owner_allows(defs, techno, house) {
                return None;
            }
            Some(d.source_key.as_str())
        })
        .collect();
    keys.sort_unstable();
    keys.first().copied()
}

/// 是否短局 `BaseUnit`（`[General] BaseUnit=`，缺表时回落为可部署成建造场的载具）。
pub(crate) fn is_base_unit(defs: &RuntimeDefinitions, type_id: TypeId) -> bool {
    if defs.base_units.iter().any(|id| *id == type_id) {
        return true;
    }
    if !defs.base_units.is_empty() {
        return false;
    }
    deploy_into_type(defs, type_id).is_some_and(|target| is_construction_yard(defs, target))
}
