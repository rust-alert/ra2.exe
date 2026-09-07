//! Alpha 冻结规则辅助（后续可由 adaptor 定义表替换）。

use ra_assets::{TechnoKind, WarheadRegistry};

/// 冻结竖切内 MCV → 建造场映射。
pub(crate) fn deploy_into_type(type_id: &str) -> Option<&'static str> {
    match type_id {
        "AMCV" => Some("GACNST"),
        "SMCV" => Some("NACNST"),
        _ => None,
    }
}

pub(crate) fn is_construction_yard(type_id: &str) -> bool {
    matches!(type_id, "GACNST" | "NACNST")
}

pub(crate) fn is_power_plant(type_id: &str) -> bool {
    matches!(type_id, "GAPOWR" | "NAPOWR")
}

pub(crate) fn requires_power_plant(type_id: &str) -> bool {
    matches!(type_id, "GAPILE" | "NAHAND" | "GAWEAP" | "NAWEAP" | "GAREFN" | "NAREFN")
}

pub(crate) fn is_refinery(type_id: &str) -> bool {
    matches!(type_id, "GAREFN" | "NAREFN")
}

pub(crate) fn factory_matches_unit(factory_type: &str, kind: TechnoKind) -> bool {
    match kind {
        TechnoKind::Infantry => matches!(factory_type, "GAPILE" | "NAHAND"),
        TechnoKind::Vehicle => matches!(factory_type, "GAWEAP" | "NAWEAP"),
        TechnoKind::Aircraft | TechnoKind::Building => false,
    }
}

pub(crate) fn is_production_factory(type_id: &str) -> bool {
    matches!(type_id, "GAPILE" | "NAHAND" | "GAWEAP" | "NAWEAP")
}

/// 冻结竖切建筑的电力增量（正=供电，负=耗电）。
pub(crate) fn building_power_delta(type_id: &str) -> i32 {
    match type_id {
        "GAPOWR" | "NAPOWR" => 200,
        "GAPILE" | "NAHAND" => -20,
        "GAWEAP" | "NAWEAP" => -30,
        "GAREFN" | "NAREFN" => -50,
        _ => 0,
    }
}

pub(crate) fn full_verses() -> [u32; 11] {
    [100; 11]
}

pub(crate) fn verses_for(warheads: &WarheadRegistry, warhead: &str) -> [u32; 11] {
    warheads.get(warhead).map(|w| w.verses).unwrap_or_else(full_verses)
}
