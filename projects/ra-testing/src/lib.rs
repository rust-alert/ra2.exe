//! `ra2` 的测试支撑：无窗口遭遇战夹具与 GUI 自动化测试计划。
//!
//! 这里不创建窗口、不初始化 GPU，也不访问用户游戏目录。headless 用例必须通过
//! 与产品相同的 `Session`、`World` 和 `GameCommand` 路径推进。

mod gui;
mod headless;
mod status;

pub use gui::{
    standard_duel_gui_plan, GuiAction, GuiAutomationPlan, GuiExpectation, GuiPoint,
};
pub use headless::{standard_duel, HeadlessCase, HeadlessObservation};
pub use status::TestStatus;
