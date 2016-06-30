//! 玩法规则语义系统。

pub(crate) mod ai;
mod ai_triggers;
mod combat;
mod construction;
mod capture;
mod definitions_query;
mod deploy;
mod economy;
mod effects;
pub(crate) mod eva_advice;
mod infiltrate;
mod movement;
mod powers;
mod production;
mod repair;
mod rules;
mod script_teams;
mod targeting;
mod tech_tree;
mod terrain_spawn;
mod terrain_spawn_tick;
mod transport;
mod triggers;

pub use ai::houses_are_allied;
pub(crate) use ai_triggers::tick_ai_triggers;
pub use ai_triggers::AiTriggerRuntime;
pub(crate) use definitions_query::{
    building_power, deploy_into_type, factory_matches_category, factory_matches_unit, is_agent, is_capturable,
    is_construction_yard, is_engineer, is_harvester, is_power_plant, is_production_factory, is_radar, is_refinery, owner_allows,
    requires_power_plant, starting_mcv_type_for_house,
};
pub(crate) use rules::{full_verses, verses_for};
pub(crate) use script_teams::{flush_pending_team_spawns, tick_script_teams};
pub use script_teams::ScriptTeamRuntime;
pub(crate) use tech_tree::{build_limit_reached, is_type_eligible, living_structure_keys, TechTreePlayer};
pub(crate) use production::produce_ticks_for;
pub(crate) use repair::tick_repairs;
pub use powers::{LightningStormState, start_lightning_storm, tick_lightning_storm};
pub use terrain_spawn::{
    TerrainSpawnerPhase, TerrainSpawnerState, TerrainSpawnerTick, seed_terrain_spawners, terrain_spawn_sample,
    terrain_spawner_frame_signature,
};
pub use triggers::{TriggerRuntime, tick_triggers};
