//! 一局游戏运行时入口。
//!
//! 内含权威状态、对局调度、玩法系统与呈现投影。不创建窗口、不初始化 GPU。

#![deny(missing_docs)]

mod gameplay;
mod lifecycle;
mod persistence;
mod presentation;
mod runtime;
mod spatial;
mod state;

pub use runtime::{
    AnimState, CommandReject, CommandRejectReason, DEFAULT_TICK_HZ, GameCommand, InputFrame, MAX_TICKS_PER_PUMP, MatchOutcome,
    MatchStats, RenderSnapshot, Session, SessionScreen, SkirmishOpenResult, SnapshotPlayer, SnapshotProduceQueue, SnapshotUnit,
    decode_command, decode_commands, encode_command, encode_commands, open_skirmish_session,
};
pub use state::{
    ATTACK_COOLDOWN_TICKS, CELL_MOVE_COST, DEFAULT_ATTACK_DAMAGE, DEFAULT_ATTACK_RANGE, HIT_FLASH_TICKS, ORE_INCOME_PER_TRIP,
    ORE_TRIP_TICKS, PRODUCE_TICKS, PlayerState, TURRET_TURN_STEP, World, WorldEntity,
};

/// 对局运行时句柄（过渡期为 [`Session`] 别名）。
pub type Engine = Session;

/// 理想对外对局句柄别名（与 [`Session`] 同一类型，后续内收字段）。
pub type Match = Session;
