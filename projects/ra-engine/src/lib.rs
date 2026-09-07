//! 一局游戏运行时入口。
//!
//! 内含权威世界状态与对局调度。不创建窗口、不初始化 GPU。

#![deny(missing_docs)]

mod runtime;
mod world;

pub use runtime::{
    AnimState, DEFAULT_TICK_HZ, MAX_TICKS_PER_PUMP, MatchOutcome, MatchStats, RenderSnapshot, Session,
    SessionScreen, SkirmishOpenResult, SnapshotPlayer, SnapshotProduceQueue, SnapshotUnit,
    open_skirmish_session,
};
pub use world::{
    ATTACK_COOLDOWN_TICKS, CELL_MOVE_COST, CommandReject, CommandRejectReason, DEFAULT_ATTACK_DAMAGE,
    DEFAULT_ATTACK_RANGE, GameCommand, HIT_FLASH_TICKS, InputFrame, ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS,
    PRODUCE_TICKS, PlayerState, TURRET_TURN_STEP, World, WorldEntity, decode_command, decode_commands,
    encode_command, encode_commands,
};

/// 对局运行时句柄。
///
/// 当前为 [`Session`] 的类型别名。
pub type Engine = Session;
