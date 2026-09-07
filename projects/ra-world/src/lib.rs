//! 世界状态类型转发。实现位于 `ra-engine`。

#![deny(missing_docs)]

pub use ra_engine::{
    ATTACK_COOLDOWN_TICKS, CELL_MOVE_COST, CommandReject, CommandRejectReason, DEFAULT_ATTACK_DAMAGE,
    DEFAULT_ATTACK_RANGE, GameCommand, HIT_FLASH_TICKS, InputFrame, ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS,
    PRODUCE_TICKS, PlayerState, TURRET_TURN_STEP, World, WorldEntity, decode_command, decode_commands,
    encode_command, encode_commands,
};
