//! 能力注册表。

use ra_types::RuntimeDefinitions;

/// 与定义中 `CapabilitySet` 对齐的运行时索引。
#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    /// 已启用能力标签。
    pub tags: Vec<String>,
}

impl CapabilityRegistry {
    /// 从冻结定义镜像标签。
    pub fn from_definitions(definitions: &RuntimeDefinitions) -> Self {
        Self {
            tags: definitions.capabilities.tags.clone(),
        }
    }
}
