//! 对局暂停层状态（Menu / AbortConfirm / InGameOptions / Diplomacy）。

/// 暂停菜单及其二级页（仿真暂停期间**替换** HUD overlay，不叠在 HUD 上）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BattlePauseLayer {
    /// 主暂停六钮。
    #[default]
    Menu,
    /// 放弃任务确认（Leave / Cancel）。
    AbortConfirm,
    /// 局内选项 `0xBBB`。
    InGameOptions,
    /// 外交花名册（`diplo_btn` 入口；只读同盟）。
    Diplomacy,
}

impl BattlePauseLayer {
    /// Esc：Menu / AbortConfirm / Diplomacy → 恢复对局；InGameOptions → 回 Menu。
    pub fn on_escape(self) -> EscapeRoute {
        match self {
            Self::Menu | Self::AbortConfirm | Self::Diplomacy => EscapeRoute::ResumeMission,
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
