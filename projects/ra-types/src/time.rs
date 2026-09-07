//! 逻辑时间。

/// 逻辑 tick 计数（与墙钟无关）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

/// 每秒逻辑 tick 数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TickRate(pub u32);

impl Default for TickRate {
    fn default() -> Self {
        Self(15)
    }
}

/// 以 tick 计量的时长。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DurationTicks(pub u64);
