//! 与定义中能力声明对齐的运行时索引。

use ra_types::{BuiltinCapability, RuntimeDefinitions};

/// 与定义中 `CapabilitySet` 对齐的运行时索引。
#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    /// 已启用内置能力。
    pub builtins: Vec<BuiltinCapability>,
}

impl CapabilityRegistry {
    /// 从冻结定义镜像能力声明。
    pub fn from_definitions(definitions: &RuntimeDefinitions) -> Self {
        Self { builtins: definitions.capabilities.builtins.clone() }
    }

    /// 引擎定义是否声明了该内置能力。
    pub fn contains(&self, cap: BuiltinCapability) -> bool {
        self.builtins.contains(&cap)
    }
}
