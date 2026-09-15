//! 一局权威状态。

pub mod battle;
mod battle_sfx;
pub(crate) mod components;
mod ecs_registry;
mod entities;
mod eva;
mod players;
mod resources;
mod rng;

pub use battle::{BattleState, *};
pub use battle_sfx::BattleSfxCue;
pub(crate) use eva::EvaBaseUnderAttackGate;
pub use eva::{EvaCue, RadarEvent};
pub use players::PlayerState;
