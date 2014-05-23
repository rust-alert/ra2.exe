//! `ra-desktop` 库面：供集成测试与二进制共用模块。
//!
//! 二进制入口仍为 `main.rs` → `rust-ra2`。

#![allow(missing_docs)]

pub mod boot;
pub mod config;
pub mod fs_source;
pub mod load_job;
pub mod local_player;
pub mod match_ctrl;
pub mod menu_action;
pub mod preview_job;
pub mod screen;
pub mod screenshot;
pub mod shell;
pub mod skirmish_setup;
#[cfg(feature = "test-harness")]
pub mod test_boot;
pub mod ui_assets;
pub mod ui_compose;
pub mod ui_decode;
pub mod ui_hit;
pub mod ui_layout;
pub mod ui_movie;
pub mod ui_page;
pub mod ui_resolve;
pub mod ui_slots;
pub mod ui_text;
