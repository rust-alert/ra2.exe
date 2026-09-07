//! `ra2` 的测试支撑：无窗口遭遇战夹具与 GUI 自动化测试计划。
//!
//! 这里不创建窗口、不初始化 GPU，也不访问用户游戏目录。headless 用例必须通过
//! 与产品相同的 `Session`、`World` 和 `GameCommand` 路径推进。

#![deny(missing_docs)]

mod alpha_slice;
mod gui;
mod headless;
mod status;

pub use alpha_slice::{
    ALPHA_SKIRMISH_SLICE_ID, AlphaSkirmishSlice, SliceBuilding, SliceBuildingRole, SliceUnit, SliceUnitRole,
    alpha_skirmish_v1,
};
pub use gui::{GuiAction, GuiAutomationPlan, GuiExpectation, GuiPoint, standard_duel_gui_plan};
pub use headless::{HeadlessCase, HeadlessObservation, mcv_deploy_open, standard_duel, yard_open};
pub use status::TestStatus;
