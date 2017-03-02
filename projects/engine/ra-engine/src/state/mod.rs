//! 一局权威状态。

pub mod battle;
pub(crate) mod components;
mod ecs_registry;
mod entities;
mod eva;
mod battle_sfx;
mod players;
mod resources;
mod rng;

pub use battle::{BattleState, *};
pub(crate) use eva::EvaBaseUnderAttackGate;
pub use battle_sfx::BattleSfxCue;
pub use eva::EvaCue;
pub use players::PlayerState;
