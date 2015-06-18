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
pub mod ui_compose;

pub use core::*;

// 过渡期路径别名：保持旧 `ra_widgets::ui_*` 引用可编译。
pub use skin::fs_source;
pub use skin::assets as ui_assets;
pub use skin::text as ui_text;
pub use skin::slots as ui_slots;
pub use skin::decode as ui_decode;
pub use skin::resolve as ui_resolve;
pub use input::hit as ui_hit;
pub use animation::typewriter as ui_typewriter;
pub use animation::shell_slide;
pub use chrome::movie as ui_movie;
pub use render::present as ui_present;
pub use render::{RenderCommand, RenderPlan};
pub use screens::page as ui_page;
pub use screens::options_dialog;
pub use screens::skirmish_setup;
pub use screens::campaign_setup;
pub use screens::startup_splash;
pub use screens::battle_hud;
pub use screens::battle_pause_menu;
pub use core::load_kind;
pub use core::menu_action;
pub use core::original_screen;
