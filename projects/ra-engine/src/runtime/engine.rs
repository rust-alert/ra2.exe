//! 引擎门面：持有冻结定义，创建对局会话。

use std::sync::Arc;

use ra_types::RuntimeDefinitions;

use super::session::Session;
use crate::state::World;

/// 引擎配置（骨架）。
#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    /// 逻辑 tick 频率；`0` 表示使用默认。
    pub tick_hz: u32,
}

/// 能力注册表（骨架：与定义中的 [`ra_types::CapabilitySet`] 对齐的运行时索引）。
#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    /// 已启用能力标签（过渡期直接镜像定义）。
    pub tags: Vec<String>,
}

/// 对局引擎：只认识冻结定义，不认识 MIX / INI / 安装目录。
#[derive(Debug, Clone)]
pub struct Engine {
    definitions: Arc<RuntimeDefinitions>,
    capabilities: CapabilityRegistry,
    config: EngineConfig,
}

impl Engine {
    /// 用已冻结的定义构造引擎。
    pub fn new(definitions: Arc<RuntimeDefinitions>, config: EngineConfig) -> Self {
        let capabilities = CapabilityRegistry { tags: definitions.capabilities.tags.clone() };
        Self { definitions, capabilities, config }
    }

    /// 共享的冻结定义。
    pub fn definitions(&self) -> &Arc<RuntimeDefinitions> {
        &self.definitions
    }

    /// 能力注册表。
    pub fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    /// 引擎配置。
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// 在已有权威 [`World`] 上创建会话（地图与实体仍由调用方经现有引导装入）。
    pub fn create_session(&self, world: World, map_name: impl Into<String>) -> Session {
        let _ = self.definitions.as_ref();
        Session::new(world, map_name)
    }
}
