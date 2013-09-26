//! 玩法规则语义系统。

pub(crate) mod ai;
mod combat;
mod construction;
mod definitions_query;
mod deploy;
mod economy;
mod effects;
mod movement;
mod powers;
mod production;
mod rules;
mod targeting;
mod transport;
mod triggers;

pub(crate) use definitions_query::{
    building_power, deploy_into_type, factory_matches_category, factory_matches_unit, is_construction_yard, is_power_plant,
    is_production_factory, is_refinery, owner_allows, requires_power_plant,
};
pub(crate) use rules::{full_verses, verses_for};