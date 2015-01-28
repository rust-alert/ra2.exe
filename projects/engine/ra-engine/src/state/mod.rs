//! 一局权威状态。

pub(crate) mod components;
mod ecs_registry;
mod entities;
mod battle_state;
mod players;
mod resources;
mod rng;

pub use battle_state::*;
pub use players::PlayerState;
