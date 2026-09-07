//! 对局生命周期、时钟、命令与系统调度。

mod boot;
mod clock;
mod commands;
mod config;
#[path = "match.rs"]
mod match_mod;
mod phase;
mod reject;
mod schedule;

pub use boot::{SkirmishOpenResult, open_skirmish_session};
pub use commands::{GameCommand, InputFrame, decode_command, decode_commands, encode_command, encode_commands};
pub use match_mod::*;
pub use reject::{CommandReject, CommandRejectReason};
