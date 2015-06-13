//! 原版产品页面（对玩家可见的 UI 页），与 `ra_engine::SessionPhase` 正交。
//!
//! 枚举值对齐 RA2/YR 主流程，不是自研「现代菜单」抽象。外观复刻原版；
//! 实现仍用 wgpu / 现代输入。

/// 当前顶层窗口内显示的**原版产品页**。
///
/// 不要与 [`ra_engine::SessionPhase`] 混用：页面可以没有会话，
/// 结算页仍可持有已 `Finished` 的会话快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OriginalScreen {
    /// 启动闪屏产品页（画面由 `startup_splash` owner 持有；最短展示 + 预处理后进主菜单）。
    #[default]
    Splash,
    /// 主菜单。
    MainMenu,
    /// 单人游戏入口（新战役 / 载入 / 遭遇战）。
    SinglePlayerMenu,
    /// 战役选边页（盟军 / 新兵训练营 / 苏军；Pre-Alpha 可进可回，开局属 Alpha）。
    Campaign,
    /// 遭遇战大厅（地图、槽位、规则）。
    SkirmishLobby,
    /// 遭遇战选图（自订战役；对话框 `0x6B`）。
    ChooseMap,
    /// 网络游戏入口（Alpha 可见禁用）。
    Network,
    /// 进战斗前装载页（遭遇战 / 战役共用；内容由 [`crate::LoadKind`] 区分；与启动闪屏无关）。
    LoadScreen,
    /// 战斗中（含 HUD；输入 → 命令 → tick → 渲染）。不含大厅 / 装载 / 结算。
    Battle,
    /// 结果 / 战报（不再推进 tick）。
    Results,
    /// 选项。
    Options,
    /// 退出确认（主菜单之上的二次确认）。
    ExitConfirm,
}

impl OriginalScreen {
    /// 稳定短名（日志 / 标题 / 与 `ui-states.json` id 对齐）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Splash => "splash",
            Self::MainMenu => "main_menu",
            Self::SinglePlayerMenu => "single_player_menu",
            Self::Campaign => "campaign",
            Self::SkirmishLobby => "skirmish_lobby",
            Self::ChooseMap => "choose_map",
            Self::Network => "network",
            Self::LoadScreen => "load_screen",
            Self::Battle => "battle",
            Self::Results => "results",
            Self::Options => "options",
            Self::ExitConfirm => "exit_confirm",
        }
    }

    /// 是否对战斗输入生成 `GameCommand`。
    pub fn accepts_battle_commands(self) -> bool {
        matches!(self, Self::Battle)
    }

    /// 是否推进会话仿真时钟。
    pub fn pumps_session(self) -> bool {
        matches!(self, Self::Battle)
    }

    /// 绘制是否依赖已创建的 `Session`。
    pub fn requires_session(self) -> bool {
        matches!(self, Self::Battle | Self::Results)
    }
}
