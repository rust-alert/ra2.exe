//! GUI 自动化计划模型（平台无关）。

use std::{path::PathBuf, time::Duration};

/// 屏幕坐标。实际执行器须先将其映射到目标窗口客户区。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiPoint {
    /// 客户区 X（像素）。
    pub x: i32,
    /// 客户区 Y（像素）。
    pub y: i32,
}

/// GUI 测试的语义化操作。执行器由平台专属路径提供。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiAction {
    /// 等待主窗口出现。
    WaitForWindow {
        /// 最长等待时间。
        timeout: Duration,
    },
    /// 左键单击。
    Click {
        /// 点击位置。
        point: GuiPoint,
    },
    /// 右键单击。
    RightClick {
        /// 点击位置。
        point: GuiPoint,
    },
    /// 按键（虚拟键名，由执行器映射）。
    Key {
        /// 虚拟键标识。
        virtual_key: String,
    },
    /// 固定等待。
    Wait {
        /// 等待时长。
        duration: Duration,
    },
    /// 截图并按名归档。
    Capture {
        /// 截图基线名。
        name: String,
    },
    /// 轮询状态旁路直至谓词成立（由执行器解释 `expect` 键）。
    WaitStatus {
        /// 旁路文件路径。
        path: PathBuf,
        /// 例如 `tick>=1` / `outcome!=none`。
        expect: String,
        /// 最长轮询时间。
        timeout: Duration,
    },
    /// 请求进程退出。
    Exit,
}

/// GUI 测试的可观察结果。不能以渲染帧数断言仿真正确性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiExpectation {
    /// 窗口标题包含子串。
    WindowTitleContains(String),
    /// 截图与基线一致。
    ScreenshotMatches {
        /// 截图名。
        name: String,
        /// 基线文件路径。
        baseline: PathBuf,
    },
    /// 状态旁路满足期望串。
    StatusMatches {
        /// 旁路路径。
        path: PathBuf,
        /// 期望串。
        expect: String,
    },
    /// 进程以成功码退出。
    ProcessExitedSuccessfully,
}

/// 平台无关的桌面测试计划。
///
/// 后续 Windows 执行器可采用 UI Automation 负责窗口与输入，截图比较仍由本计划
/// 保持统一。不要把 OS 自动化库引入 `ra-desktop`。
#[derive(Debug, Clone)]
pub struct GuiAutomationPlan {
    /// 计划名称。
    pub name: String,
    /// 待启动可执行文件。
    pub executable: PathBuf,
    /// 工作目录。
    pub working_directory: PathBuf,
    /// 命令行参数。
    pub args: Vec<String>,
    /// 额外环境变量。
    pub env: Vec<(String, String)>,
    /// 有序动作列表。
    pub actions: Vec<GuiAction>,
    /// 结束时期望。
    pub expectations: Vec<GuiExpectation>,
}

/// Alpha 合成 duel 场景的首个 GUI 计划（选中 / 移动 / 攻击 / 胜负 / 退出）。
///
/// 坐标为相对 1280×720 客户区的粗略点位，执行器接入后需按实际标记位置校准。
pub fn standard_duel_gui_plan(executable: PathBuf, working_directory: PathBuf, status_path: PathBuf) -> GuiAutomationPlan {
    let status = status_path.display().to_string();
    GuiAutomationPlan {
        name: "standard-duel".into(),
        executable,
        working_directory,
        args: vec!["--test-scene=duel".into()],
        env: vec![("RA2_TEST_SCENE".into(), "duel".into()), ("RA2_TEST_STATUS_PATH".into(), status)],
        actions: vec![
            GuiAction::WaitForWindow { timeout: Duration::from_secs(30) },
            GuiAction::WaitStatus { path: status_path.clone(), expect: "tick>=1".into(), timeout: Duration::from_secs(15) },
            GuiAction::Click { point: GuiPoint { x: 420, y: 360 } },
            GuiAction::Wait { duration: Duration::from_millis(200) },
            GuiAction::RightClick { point: GuiPoint { x: 520, y: 360 } },
            GuiAction::Wait { duration: Duration::from_secs(1) },
            GuiAction::RightClick { point: GuiPoint { x: 640, y: 360 } },
            GuiAction::WaitStatus {
                path: status_path.clone(),
                expect: "outcome!=none".into(),
                timeout: Duration::from_secs(60),
            },
            GuiAction::Capture { name: "duel-victory".into() },
            GuiAction::Exit,
        ],
        expectations: vec![
            GuiExpectation::WindowTitleContains("ra2".into()),
            GuiExpectation::StatusMatches { path: status_path, expect: "outcome!=none".into() },
            GuiExpectation::ProcessExitedSuccessfully,
        ],
    }
}

/// 与 `ra-desktop` 自动测试 `dump_key_ui_screenshots_for_acceptance` 对齐的稳定截图名（无扩展名）。
///
/// 执行器接入后可用 `GuiAction::Capture { name }` 对照这些基线；当前无 GPU 窗口时由桌面测试直接写 PNG。
pub fn pre_alpha_acceptance_capture_names() -> &'static [&'static str] {
    &[
        "main_menu",
        "main_menu_hover",
        "single_player_menu",
        "skirmish_lobby",
        "skirmish_lobby_alt",
        "load_screen",
        "options",
        "network",
        "match",
        "match_paused",
        "match_reject",
        "results",
        "results_lobby_hover",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_alpha_capture_names_cover_entry_flow() {
        let names = pre_alpha_acceptance_capture_names();
        assert!(names.contains(&"main_menu"));
        assert!(names.contains(&"skirmish_lobby"));
        assert!(names.contains(&"match"));
        assert!(names.contains(&"results"));
        assert!(names.len() >= 8);
    }
}
