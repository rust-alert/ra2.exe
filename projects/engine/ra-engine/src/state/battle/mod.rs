//! 确定性世界推进。不依赖渲染器与文件系统。

mod commands;
mod ecs_mutation;
mod ecs_query;
mod hash;
mod init;
mod placement;
mod players;
mod presentation;
mod simulation;
mod types;

pub use types::{
    ATTACK_COOLDOWN_TICKS, BUILD_TIME_TICKS_PER_UNIT, BattleState, CELL_MOVE_COST, DEFAULT_ATTACK_DAMAGE, DEFAULT_ATTACK_RANGE, EcsCombatView,
    HIT_FLASH_TICKS, ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS, PRODUCE_TICKS, TURRET_TURN_STEP,
};
