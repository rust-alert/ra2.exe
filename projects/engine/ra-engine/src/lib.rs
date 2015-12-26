//! 战斗运行时入口。
//!
//! 三个独立对象：[`Engine`]（长期）、[`Session`]（一次运行）、[`BattleSession`]（一场战斗仿真）。
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
    CapabilityRegistry, Engine, EngineConfig, EngineError, EngineRuntime, EngineVersion, SessionValidationError, SystemPhase, SystemSchedule,
};
pub use game::{
    evaluate_build_availability, evaluate_produce_availability, living_structure_type_keys, AnimState,
    BattleCapabilitiesSnapshot, BattleOutcome, BattleSession, BattleStats, CapabilityItem, CommandReject,
    CommandRejectReason, DEFAULT_TICK_HZ, DeployCapability, GameCommand, HudSnapshot, InputFrame, MAX_TICKS_PER_PUMP,
    RenderSnapshot, SessionBootKind, SessionScreen, SnapshotPlayer, SnapshotProduceQueue, SnapshotUnit, decode_command,
    decode_commands, decode_scheduled, difficulty_extra_produce, difficulty_skips_offensive, encode_command,
    encode_commands, encode_scheduled,
};
pub use gameplay::{LightningStormState, TriggerRuntime, start_lightning_storm, tick_lightning_storm, tick_triggers};
pub use presentation::DirtyEntitySet;
pub use session::{Session, SessionPhase, SessionSpec, SkirmishOpenResult, open_campaign_session, open_skirmish_session};
pub use state::{
    ATTACK_COOLDOWN_TICKS, CELL_MOVE_COST, DEFAULT_ATTACK_DAMAGE, DEFAULT_ATTACK_RANGE, EcsCombatView, HIT_FLASH_TICKS, BattleState,
    ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS, PRODUCE_TICKS, PlayerState, TURRET_TURN_STEP,
};
