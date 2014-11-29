//! 通用 ECS 基础设施。不含红警玩法语义。
//!
//! `EcsEntity` 仅供 `ra-engine` 内部映射；对外身份使用 `ra-types::EntityId`。

#![deny(missing_docs)]

mod commands;
mod entity;
mod store;
mod world;

pub use commands::EcsCommandBuffer;
pub use entity::EcsEntity;
pub use world::EcsWorld;

/// 可放入 [`EcsWorld`] 的组件标记。
///
/// 玩法组件类型应定义在 `ra-engine`（或其它上层 crate），本 crate 只提供存储与查询。
/// `Send`：装载线程可携带含 ECS 的会话结果回主线程。
pub trait Component: Clone + Send + 'static {}

impl<T: Clone + Send + 'static> Component for T {}
