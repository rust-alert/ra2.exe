//! 一局权威状态。

mod battle_state;
pub(crate) mod components;
mod ecs_registry;
mod entities;
mod eva;
mod players;
mod resources;
mod rng;

pub use battle_state::*;
pub(crate) use eva::EvaBaseUnderAttackGate;
pub use eva::EvaCue;
pub use players::PlayerState;
