//! 遭遇战大厅 `Unit Count`：按阵营 Owner 与 `AllowedToStartInMultiplayer` 种开局部队。

use ra_types::{RuntimeDefinitions, TechnoClass, TypeId};

use crate::gameplay::{
    deploy_into_type, forbidden_houses_forbids, is_base_unit, owner_allows, required_houses_allows, starting_mcv_id_for_house, type_key_of,
};

/// 某 house 在给定科技上限下可作开局免费部队的类型（步兵 / 地面载具分表，按造价再按类型键稳定排序）。
///
/// 排除：飞行器、海军、`AllowedToStartInMultiplayer=no`、BaseUnit / MCV、科技或 Owner 不符者。
pub(crate) fn starting_unit_pools(defs: &RuntimeDefinitions, house: &str, tech_level: i32) -> (Vec<TypeId>, Vec<TypeId>) {
    let mut infantry = Vec::new();
    let mut vehicles = Vec::new();
    for techno in defs.techno.iter() {
        if !matches!(techno.class, TechnoClass::Infantry | TechnoClass::Vehicle) {
            continue;
        }
        if techno.naval || !techno.allowed_to_start_in_multiplayer {
            continue;
        }
        if is_base_unit(defs, techno.id) {
            continue;
        }
        if techno.tech_level < 0 || techno.tech_level > tech_level {
            continue;
        }
        if !owner_allows(defs, techno, house) {
            continue;
        }
        if !required_houses_allows(defs, techno, house) {
            continue;
        }
        if forbidden_houses_forbids(defs, techno, house) {
            continue;
        }
        match techno.class {
            TechnoClass::Infantry => infantry.push(techno),
            TechnoClass::Vehicle => vehicles.push(techno),
            _ => {}
        }
    }
    let sort_key = |a: &&ra_types::TechnoDefinition, b: &&ra_types::TechnoDefinition| {
        a.cost.cmp(&b.cost).then_with(|| a.type_key.as_str().cmp(b.type_key.as_str()))
    };
    infantry.sort_by(sort_key);
    vehicles.sort_by(sort_key);
    (infantry.into_iter().map(|t| t.id).collect(), vehicles.into_iter().map(|t| t.id).collect())
}

/// 交替取**最便宜**步兵与**最便宜**载具（各池造价升序首项），得到 `unit_count` 条类型 id。
///
/// 不轮转池内高价单位：开局 Unit Count 只重复基础兵与基础坦克（如 `E1`/`MTNK`、`E2`/`HTNK`）。
pub(crate) fn compose_starting_unit_ids(infantry: &[TypeId], vehicles: &[TypeId], unit_count: i32) -> Vec<TypeId> {
    let n = unit_count.max(0) as usize;
    if n == 0 || (infantry.is_empty() && vehicles.is_empty()) {
        return Vec::new();
    }
    let inf0 = infantry.first().copied();
    let veh0 = vehicles.first().copied();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let chosen = if i % 2 == 0 { inf0.or(veh0) } else { veh0.or(inf0) };
        if let Some(id) = chosen {
            out.push(id);
        }
    }
    out
}

/// 该 house 开局 MCV 展开后的建造场 Foundation 宽高（缺省 4×4）。
///
/// 开局部队不得种进此矩形，否则展开建造场后会把人封进院子。
pub(crate) fn starting_deploy_clearance(defs: &RuntimeDefinitions, house: &str) -> (u16, u16) {
    let Some(mcv) = starting_mcv_id_for_house(defs, house)
    else {
        return (4, 4);
    };
    let Some(yard) = deploy_into_type(defs, mcv)
    else {
        return (4, 4);
    };
    let foundation = defs.structures.get_by_id(yard).map(|s| s.foundation.clone()).unwrap_or_default();
    (foundation.width.max(1), foundation.height.max(1))
}

/// 诊断用：把类型 id 列成稳定键串。
pub(crate) fn format_type_keys(defs: &RuntimeDefinitions, ids: &[TypeId]) -> String {
    ids.iter().map(|id| type_key_of(defs, *id).to_string()).collect::<Vec<_>>().join(",")
}
