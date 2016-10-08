//! 对局暂停层状态（Menu / AbortConfirm / InGameOptions）。

/// 暂停菜单及其二级页（仿真暂停期间叠在 HUD 上）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BattlePauseLayer {
    /// 主暂停四钮。
    #[default]
    Menu,
    /// 放弃任务确认（Leave / Cancel）。
    AbortConfirm,
    /// 局内选项 `0xBBB`。
    InGameOptions,
}

impl BattlePauseLayer {
    /// Esc 路由（对齐 vera 局内状态机）。
    pub fn on_escape(self) -> EscapeRoute {
        match self {
            Self::Menu => EscapeRoute::ResumeMission,
            Self::AbortConfirm => EscapeRoute::ResumeMission,
            Self::InGameOptions => EscapeRoute::ToLayer(Self::Menu),
        }
    }
}

/// Esc 处理后的去向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeRoute {
    /// 关闭暂停层并恢复对局。
    ResumeMission,
    /// 切到另一暂停子层（对局仍暂停）。
    ToLayer(BattlePauseLayer),
}
