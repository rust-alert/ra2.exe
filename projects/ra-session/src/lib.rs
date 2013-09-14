//! 对局会话类型转发。实现位于 `ra-engine`。

#![deny(missing_docs)]

pub use ra_engine::{
    AnimState, DEFAULT_TICK_HZ, MAX_TICKS_PER_PUMP, MatchOutcome, MatchStats, RenderSnapshot, Session,
    SessionScreen, SkirmishOpenResult, SnapshotPlayer, SnapshotProduceQueue, SnapshotUnit,
    open_skirmish_session,
};
