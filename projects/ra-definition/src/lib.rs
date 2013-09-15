//! 冻结运行时定义契约。adaptor 填充，`ra-engine` 消费；双方不得互相依赖实现。

#![deny(missing_docs)]

use ra_types::TypeId;

/// 对局启动时冻结的内容定义集（骨架）。
#[derive(Debug, Clone, Default)]
pub struct RuntimeDefinitions {
    /// 内容指纹（规则/地图等混入；细节后续补齐）。
    pub fingerprint: u64,
}

/// 单位/建筑等类型定义占位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeDefinitionId(pub TypeId);
