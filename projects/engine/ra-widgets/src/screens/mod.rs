//! 页面编排。

pub mod page;
pub mod options_dialog;
pub mod skirmish_setup;
pub mod campaign_setup;
pub mod startup_splash;
pub mod battle_hud;
pub mod battle_order_icons;
pub mod battle_pause_menu;

pub use page::*;
pub use options_dialog::*;
pub use skirmish_setup::*;
pub use campaign_setup::*;
pub use startup_splash::*;
// `battle_hud` / `battle_pause_menu` 仅通过子模块路径导出，避免同名 `hit_at` 冲突。
