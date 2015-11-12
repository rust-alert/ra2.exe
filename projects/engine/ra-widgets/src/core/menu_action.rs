//! 菜单导航动作（与 [`crate::skin::slots`] / [`crate::input::hit`] 共用）。

/// 菜单导航动作（键盘或逻辑命中框触发）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// 进入单人页。
    OpenSinglePlayer,
    /// 网络（禁用）。
    OpenNetwork,
    /// 选项。
    OpenOptions,
    /// 打开退出确认页（尚未真正退出进程）。
    Exit,
    /// 确认退出进程。
    ConfirmExit,
    /// 进入遭遇战大厅。
    OpenSkirmish,
    /// 进入战役选边页。
    OpenCampaign,
    /// 选中战役侧：盟军。
    SelectCampaignAllied,
    /// 选中战役侧：新兵训练营。
    SelectCampaignTutorial,
    /// 选中战役侧：苏军。
    SelectCampaignSoviet,
    /// 循环战役难度（易 / 中 / 难）。
    CycleCampaignDifficulty,
    /// 返回上一级。
    Back,
    /// 开始装载遭遇战。
    StartSkirmish,
    /// 取消进行中的遭遇战装载。
    CancelLoad,
    /// 装载失败后在加载页重试。
    RetryLoad,
    /// 占位禁用项（不可点，无导航）。
    Noop,
    /// 循环遭遇战本地阵营（键盘快捷键；原版在玩家行下拉里改）。
    CycleSide,
    /// 循环遭遇战难度（键盘快捷键；原版在 AI 行下拉里改）。
    CycleDifficulty,
    /// 选项页接受（提交草稿）。
    OptionsAccept,
    /// 选项页取消（丢弃草稿）。
    OptionsCancel,
    /// 打开遭遇战选图页（对话框 `0x6B`）。
    ChooseMap,
    /// 选图页：使用当前选中地图并返回大厅。
    UseMap,
    /// 选中选图页游戏类型列表中的一项。
    SelectMode(usize),
    /// 选中选图页地图列表中的一项。
    SelectMap(usize),
}
