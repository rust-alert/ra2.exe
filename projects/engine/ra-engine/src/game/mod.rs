//! 一局具体 RTS 游戏。

mod ai;
pub mod capabilities;
mod commands;
mod orders;
mod outcome;
mod picking;
mod reject;
mod session;
mod snapshot;
mod types;

pub use ai::{difficulty_extra_produce, difficulty_skips_offensive};
pub use capabilities::{
    BattleCapabilitiesSnapshot, CapabilityItem, DeployCapability, SuperWeaponCapabilityItem, evaluate_build_availability,
    evaluate_produce_availability, living_structure_type_keys,
};
pub use commands::{
    GameCommand, InputFrame, decode_command, decode_commands, decode_scheduled, encode_command, encode_commands, encode_scheduled,
};
pub use outcome::{BattleOutcome, BattleStats, PlayerBattleStats};
pub use reject::{CommandReject, CommandRejectReason};
pub use session::BattleSession;
pub use snapshot::{HudSnapshot, RenderSnapshot, SnapshotPlayer, SnapshotProduceQueue, SnapshotUnit};
pub use types::{AnimState, DEFAULT_TICK_HZ, MAX_TICKS_PER_PUMP, SessionBootKind, SessionScreen};
