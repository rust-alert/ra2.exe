//! UI 组件语义与画面组合（壳层 UI 组件）。
//!
//! 无宿主窗口、不直调 wgpu 事件循环；布局几何来自 `ra-layout`。

#![allow(missing_docs)]

pub mod core;
pub mod input;
pub mod animation;
pub mod chrome;
pub mod skin;
pub mod render;
pub mod screens;
pub mod compose;

pub use core::*;

pub use skin::fs_source;
pub use animation::shell_slide;
pub use render::{RenderCommand, RenderPlan};
pub use screens::options_dialog;
pub use screens::skirmish_setup;
pub use screens::campaign_setup;
pub use screens::startup_splash;
pub use screens::battle_hud;
pub use screens::battle_order_icons;
pub use screens::battle_selection_overlay;
pub use screens::battle_pause_menu;
pub use core::load_kind;
pub use core::menu_action;
pub use core::original_screen;
