use std::{path::PathBuf, time::Duration};

/// 屏幕坐标。实际执行器须先将其映射到目标窗口客户区。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiPoint {
    pub x: i32,
    pub y: i32,
}

/// GUI 测试的语义化操作。执行器由平台专属 crate 提供。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiAction {
    WaitForWindow { timeout: Duration },
    Click { point: GuiPoint },
    RightClick { point: GuiPoint },
    Key { virtual_key: String },
    Wait { duration: Duration },
    Capture { name: String },
    Exit,
}

/// GUI 测试的可观察结果。不能以渲染帧数断言仿真正确性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiExpectation {
    WindowTitleContains(String),
    ScreenshotMatches { name: String, baseline: PathBuf },
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
    pub actions: Vec<GuiAction>,
    pub expectations: Vec<GuiExpectation>,
}
