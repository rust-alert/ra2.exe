//! 通用 ECS 基础设施。不含红警玩法语义。
//!
//! `EcsEntity` 仅供 `ra-engine` 内部映射；对外身份使用 `ra-types::EntityId`。

#![deny(missing_docs)]

/// ECS 内部实体句柄（槽位 + 世代）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EcsEntity {
    /// 槽位。
    pub slot: u32,
    /// 世代（防复用误命中）。
    pub generation: u32,
}

/// 组件存储世界（骨架；后续接入成熟 ECS 实现）。
#[derive(Debug, Default)]
pub struct EcsWorld;

/// 结构变更缓冲（骨架）。
#[derive(Debug, Default)]
pub struct EcsCommandBuffer;

impl EcsWorld {
    /// 是否仍包含该句柄（骨架恒为 false）。
    pub fn contains(&self, _entity: EcsEntity) -> bool {
        false
    }

    /// 取得命令缓冲（骨架）。
    pub fn commands(&mut self) -> EcsCommandBuffer {
        EcsCommandBuffer
    }

    /// 应用命令缓冲（骨架空操作）。
    pub fn apply(&mut self, _commands: EcsCommandBuffer) {}
}
