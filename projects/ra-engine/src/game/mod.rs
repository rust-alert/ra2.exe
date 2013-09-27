//! 一局具体 RTS 游戏。

mod commands;
mod game;
mod reject;

pub use commands::{
    GameCommand, InputFrame, decode_command, decode_commands, decode_scheduled, encode_command, encode_commands,
    encode_scheduled,
};
pub use game::*;
pub use reject::{CommandReject, CommandRejectReason};
