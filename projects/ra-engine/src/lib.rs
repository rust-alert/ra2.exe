//! 一局游戏运行时入口。
//!
//! 三个独立对象：[`Engine`]（长期）、[`Session`]（一次运行）、[`Game`]（一局 RTS）。
//! 不创建窗口、不初始化 GPU。

#![deny(missing_docs)]

mod engine;
mod game;
mod gameplay;
mod lifecycle;
mod persistence;
mod presentation;
mod session;
mod spatial;
mod state;

pub use engine::{
    CapabilityRegistry, Engine, EngineConfig, EngineError, EngineRuntime, EngineVersion, SessionValidationError,
    SystemSchedule,
};
pub use game::{
    AnimState, CommandReject, CommandRejectReason, DEFAULT_TICK_HZ, Game, GameCommand, InputFrame, MAX_TICKS_PER_PUMP,
    MatchOutcome, MatchStats, RenderSnapshot, SessionScreen, SnapshotPlayer, SnapshotProduceQueue, SnapshotUnit,
    decode_command, decode_commands, decode_scheduled, encode_command, encode_commands, encode_scheduled,
};
pub use session::{Session, SessionPhase, SessionSpec, SkirmishOpenResult, open_skirmish_session};
pub use state::{
    ATTACK_COOLDOWN_TICKS, CELL_MOVE_COST, DEFAULT_ATTACK_DAMAGE, DEFAULT_ATTACK_RANGE, HIT_FLASH_TICKS, MatchState,
    ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS, PRODUCE_TICKS, PlayerState, TURRET_TURN_STEP, WorldEntity,
};
