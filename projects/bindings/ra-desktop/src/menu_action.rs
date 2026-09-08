//! 菜单导航动作（与 [`crate::ui_slots`] / [`crate::ui_hit`] 共用）。

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
    /// 循环遭遇战本地阵营。
    CycleSide,
    /// 循环遭遇战难度。
    CycleDifficulty,
    /// 选项页接受（提交草稿）。
    OptionsAccept,
    /// 选项页取消（丢弃草稿）。
    OptionsCancel,
    /// 选中大厅地图列表中的一项。
    SelectMap(usize),
}
