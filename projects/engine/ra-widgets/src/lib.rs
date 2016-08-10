//! UI 组件语义与画面组合（壳层 UI 组件）。
//!
//! 无宿主窗口、不直调 wgpu 事件循环；布局几何来自 `ra-layout`。

#![allow(missing_docs)]

pub mod animation;
pub mod chrome;
pub mod compose;
pub mod core;
pub mod input;
pub mod render;
pub mod screens;
pub mod skin;

pub use core::*;

pub use animation::shell_slide;
pub use core::{load_kind, menu_action, original_screen};
pub use render::{RenderCommand, RenderPlan};
pub use screens::{
    battle_hud, battle_order_icons, battle_pause_menu, battle_selection_overlay, campaign_setup, options_dialog, skirmish_setup, startup_splash,
};
pub use skin::fs_source;
