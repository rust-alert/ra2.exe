//! 对局生命周期、时钟、命令与系统调度。

mod boot;
mod clock;
mod commands;
mod config;
mod engine;
mod phase;
mod reject;
mod schedule;
mod session;

pub use boot::{SkirmishOpenResult, open_skirmish_session};
pub use commands::{GameCommand, InputFrame, decode_command, decode_commands, encode_command, encode_commands};
pub use engine::{CapabilityRegistry, Engine, EngineConfig};
pub use reject::{CommandReject, CommandRejectReason};
pub use session::*;
