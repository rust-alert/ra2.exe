//! 玩法规则语义系统。

pub(crate) mod ai;
mod combat;
mod construction;
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

pub(crate) use rules::{
    building_power_delta, deploy_into_type, factory_matches_unit, full_verses, is_construction_yard, is_power_plant,
    is_production_factory, is_refinery, requires_power_plant, verses_for,
};
