//! 稳定标识符。

/// 实体稳定 ID（对外身份；禁止把 ECS 槽位当下标契约）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EntityId(pub u64);

/// 本地 / 远端玩家编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PlayerId(pub u8);

/// 类型表中的类型编号（techno / 建筑等）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TypeId(pub u32);

/// 武器定义编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WeaponId(pub u32);

/// 弹头定义编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WarheadId(pub u32);

/// 阵营 / 房屋定义编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct HouseId(pub u32);

/// 对局会话编号（壳层 / 联机标识，非 tick）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SessionId(pub u64);

/// 运动学 / 移动器定义编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LocomotorId(pub u32);
