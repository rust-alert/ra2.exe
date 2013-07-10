use std::{path::PathBuf, time::Duration};

/// 屏幕坐标。实际执行器须先将其映射到目标窗口客户区。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiPoint {
    pub x: i32,
    pub y: i32,
}

/// GUI 测试的语义化操作。执行器由平台专属路径提供。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiAction {
    WaitForWindow { timeout: Duration },
    Click { point: GuiPoint },
    RightClick { point: GuiPoint },
    Key { virtual_key: String },
    Wait { duration: Duration },
    Capture { name: String },
    /// 轮询状态旁路直至谓词成立（由执行器解释 `expect` 键）。
    WaitStatus {
        path: PathBuf,
        /// 例如 `tick>=1` / `outcome!=none`。
        expect: String,
        timeout: Duration,
    },
    Exit,
}

/// GUI 测试的可观察结果。不能以渲染帧数断言仿真正确性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiExpectation {
    WindowTitleContains(String),
    ScreenshotMatches { name: String, baseline: PathBuf },
    StatusMatches { path: PathBuf, expect: String },
    ProcessExitedSuccessfully,
}

/// 平台无关的桌面测试计划。
///
/// 后续 Windows 执行器可采用 UI Automation 负责窗口与输入，截图比较仍由本计划
/// 保持统一。不要把 OS 自动化库引入 `ra-desktop`。
#[derive(Debug, Clone)]
pub struct GuiAutomationPlan {
    pub name: String,
    pub executable: PathBuf,
    pub working_directory: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub actions: Vec<GuiAction>,
    pub expectations: Vec<GuiExpectation>,
}

/// Alpha 合成 duel 场景的首个 GUI 计划（选中 / 移动 / 攻击 / 胜负 / 退出）。
///
/// 坐标为相对 1280×720 客户区的粗略点位，执行器接入后需按实际标记位置校准。
pub fn standard_duel_gui_plan(
    executable: PathBuf,
    working_directory: PathBuf,
    status_path: PathBuf,
) -> GuiAutomationPlan {
    let status = status_path.display().to_string();
    GuiAutomationPlan {
        name: "standard-duel".into(),
        executable,
        working_directory,
        args: vec!["--test-scene=duel".into()],
        env: vec![
            ("RA2_TEST_SCENE".into(), "duel".into()),
            ("RA2_TEST_STATUS_PATH".into(), status),
        ],
        actions: vec![
            GuiAction::WaitForWindow {
                timeout: Duration::from_secs(30),
            },
            GuiAction::WaitStatus {
                path: status_path.clone(),
                expect: "tick>=1".into(),
                timeout: Duration::from_secs(15),
            },
            // 粗略点在左侧己方单位附近（校准前仅作计划占位）。
            GuiAction::Click {
                point: GuiPoint { x: 420, y: 360 },
            },
            GuiAction::Wait {
                duration: Duration::from_millis(200),
            },
            GuiAction::RightClick {
                point: GuiPoint { x: 520, y: 360 },
            },
            GuiAction::Wait {
                duration: Duration::from_secs(1),
            },
            GuiAction::RightClick {
                point: GuiPoint { x: 640, y: 360 },
            },
            GuiAction::WaitStatus {
                path: status_path.clone(),
                expect: "outcome!=none".into(),
                timeout: Duration::from_secs(60),
            },
            GuiAction::Capture {
                name: "duel-victory".into(),
            },
            GuiAction::Exit,
        ],
        expectations: vec![
            GuiExpectation::WindowTitleContains("ra2".into()),
            GuiExpectation::StatusMatches {
                path: status_path,
                expect: "outcome!=none".into(),
            },
            GuiExpectation::ProcessExitedSuccessfully,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duel_plan_has_harness_args_and_status_waits() {
        let plan = standard_duel_gui_plan(
            PathBuf::from("ra2"),
            PathBuf::from("."),
            PathBuf::from("status.txt"),
        );
        assert_eq!(plan.name, "standard-duel");
        assert!(plan.args.iter().any(|a| a.contains("test-scene=duel")));
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, GuiAction::WaitStatus { .. })));
    }
}
