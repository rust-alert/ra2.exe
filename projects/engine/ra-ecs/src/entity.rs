//! 实体句柄与槽位元数据。

/// ECS 内部实体句柄（槽位 + 世代）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EcsEntity {
    /// 槽位。
    pub slot: u32,
    /// 世代（防复用误命中）。
    pub generation: u32,
}

/// 槽位存活与世代。
#[derive(Debug, Clone)]
pub(crate) struct EntityMeta {
    pub(crate) generation: u32,
    pub(crate) alive: bool,
}
