//! 玩法规则语义系统。

pub(crate) mod ai;
mod combat;
mod construction;
mod definitions_query;
mod deploy;
mod economy;
mod effects;
mod infiltrate;
mod movement;
mod powers;
mod production;
mod rules;
mod targeting;
mod tech_tree;
mod transport;
mod triggers;

pub(crate) use definitions_query::{
    building_power, deploy_into_type, factory_matches_category, factory_matches_unit, is_agent, is_construction_yard,
    is_power_plant, is_production_factory, is_refinery, owner_allows, requires_power_plant, starting_mcv_type_for_house,
};
pub(crate) use rules::{full_verses, verses_for};
pub(crate) use tech_tree::{build_limit_reached, is_type_eligible, living_structure_keys, TechTreePlayer};
pub use powers::{LightningStormState, start_lightning_storm, tick_lightning_storm};
pub use triggers::{TriggerRuntime, tick_triggers};
