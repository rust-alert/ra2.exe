//! 稳定标识符。

/// 实体稳定 ID（仿真层将来取代 `usize` 下标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EntityId(pub u64);

/// 本地 / 远端玩家编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PlayerId(pub u8);

/// 类型表中的类型编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TypeId(pub u32);
