//! UI 组件语义与画面组合（壳层 UI 组件）。
//!
//! 无宿主窗口、不直调 wgpu 事件循环；布局几何来自 `ra-layout`。

#![allow(missing_docs)]

pub mod fs_source;
pub mod menu_action;
pub mod options_dialog;
pub mod original_screen;
pub mod shell_slide;
pub mod skirmish_setup;
pub mod startup_splash;
pub mod ui_assets;
pub mod ui_compose;
pub mod ui_decode;
pub mod ui_hit;
pub mod ui_movie;
pub mod ui_page;
pub mod ui_present;
pub mod ui_resolve;
pub mod ui_slots;
pub mod ui_text;
pub mod ui_typewriter;

pub use menu_action::*;
pub use original_screen::*;
