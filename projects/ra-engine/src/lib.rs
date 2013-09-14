//! 一局游戏运行时入口。
//!
//! 会话调度已内收于 `runtime`；世界状态仍过渡依赖 `ra-world`。不创建窗口、不初始化 GPU。

#![deny(missing_docs)]

mod runtime;

pub use runtime::{
    AnimState, DEFAULT_TICK_HZ, MAX_TICKS_PER_PUMP, MatchOutcome, MatchStats, RenderSnapshot, Session,
    SessionScreen, SkirmishOpenResult, SnapshotPlayer, SnapshotProduceQueue, SnapshotUnit,
    open_skirmish_session,
};
pub use ra_world::{
    ATTACK_COOLDOWN_TICKS, CELL_MOVE_COST, CommandReject, CommandRejectReason, DEFAULT_ATTACK_DAMAGE,
    DEFAULT_ATTACK_RANGE, GameCommand, HIT_FLASH_TICKS, InputFrame, ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS,
    PRODUCE_TICKS, PlayerState, TURRET_TURN_STEP, World, WorldEntity, decode_command, decode_commands,
    encode_command, encode_commands,
};

/// 对局运行时句柄。
///
/// 当前为 [`Session`] 的类型别名。
pub type Engine = Session;
