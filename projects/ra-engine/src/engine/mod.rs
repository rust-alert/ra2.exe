//! 长期引擎实例：冻结定义、能力表、调度计划。

mod capabilities;
mod config;
mod schedule;
mod validation;
mod version;

pub use capabilities::CapabilityRegistry;
pub use config::EngineConfig;
pub use schedule::{SystemPhase, SystemSchedule};
pub use validation::SessionValidationError;
pub use version::EngineVersion;

use std::sync::Arc;

use ra_types::RuntimeDefinitions;

use crate::game::Game;
use crate::session::{Session, SessionSpec};
use crate::state::MatchState;

/// 引擎错误（骨架）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// 校验或开局失败说明。
    Msg(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Msg(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// 推进时只读运行上下文（避免系统抓取完整 `Engine` 外壳）。
#[derive(Debug, Clone, Copy)]
pub struct EngineRuntime<'a> {
    /// 冻结定义。
    pub definitions: &'a RuntimeDefinitions,
    /// 能力表。
    pub capabilities: &'a CapabilityRegistry,
    /// 系统调度。
    pub schedule: &'a SystemSchedule,
}

/// 长期引擎实例：可创建多局 `Session`，不持有某一局的权威状态。
#[derive(Debug, Clone)]
pub struct Engine {
    definitions: Arc<RuntimeDefinitions>,
    schedule: SystemSchedule,
    capabilities: CapabilityRegistry,
    version: EngineVersion,
    config: EngineConfig,
}

impl Engine {
    /// 用冻结定义构造引擎。
    pub fn new(definitions: Arc<RuntimeDefinitions>, config: EngineConfig) -> Result<Self, EngineError> {
        let capabilities = CapabilityRegistry::from_definitions(&definitions);
        Ok(Self {
            definitions,
            schedule: SystemSchedule::default(),
            capabilities,
            version: EngineVersion::default(),
            config,
        })
    }

    /// 冻结定义。
    pub fn definitions(&self) -> &RuntimeDefinitions {
        self.definitions.as_ref()
    }

    /// 共享 `Arc`。
    pub fn definitions_arc(&self) -> &Arc<RuntimeDefinitions> {
        &self.definitions
    }

    /// 能力表。
    pub fn capabilities(&self) -> &CapabilityRegistry {
        &self.capabilities
    }

    /// 调度计划。
    pub fn schedule(&self) -> &SystemSchedule {
        &self.schedule
    }

    /// 替换 tick 阶段计划（可省略阶段以关闭对应系统）。
    pub fn set_schedule(&mut self, schedule: SystemSchedule) {
        self.schedule = schedule;
    }

    /// 引擎版本信息。
    pub fn version(&self) -> &EngineVersion {
        &self.version
    }

    /// 配置。
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// 只读推进上下文。
    pub fn runtime(&self) -> EngineRuntime<'_> {
        EngineRuntime {
            definitions: self.definitions.as_ref(),
            capabilities: &self.capabilities,
            schedule: &self.schedule,
        }
    }

    /// 校验会话规格（骨架：始终成功）。
    pub fn validate_session_spec(&self, _spec: &SessionSpec) -> Result<(), SessionValidationError> {
        Ok(())
    }

    /// 创建空会话（尚未 `start` 一局 `Game`）。
    pub fn create_session(&self, spec: SessionSpec) -> Result<Session, EngineError> {
        self.validate_session_spec(&spec).map_err(|e| EngineError::Msg(e.to_string()))?;
        Ok(Session::new(spec))
    }

    /// 由已装载的权威状态直接打开带一局游戏的会话（boot / 测试便利）。
    pub fn open_game_session(&self, state: MatchState, boot_note: impl Into<String>) -> Result<Session, EngineError> {
        let mut session = self.create_session(SessionSpec::default())?;
        let game = Game::new(state, boot_note);
        session.attach_game(game);
        Ok(session)
    }
}
