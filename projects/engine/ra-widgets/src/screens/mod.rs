//! 页面编排。

pub mod battle_abort_confirm;
pub mod battle_diplomacy;
pub mod battle_hud;
pub mod battle_in_game_options;
pub mod battle_order_icons;
pub mod battle_pause_layer;
pub mod battle_pause_menu;
pub mod battle_selection_overlay;
pub mod campaign_setup;
pub mod options_dialog;
pub mod page;
pub mod selection_power_tip;
pub mod skirmish_setup;
pub mod startup_splash;

pub use campaign_setup::*;
pub use options_dialog::*;
pub use page::*;
pub use selection_power_tip::{
    TXT_POWER_DRAIN, TXT_POWER_DRAIN2, format_csf_percent_d, paint_selection_power_tip, selection_power_drain_caption,
    structure_selection_center_preview,
};
pub use skirmish_setup::*;
pub use startup_splash::*;
// `battle_hud` / pause 子页仅通过子模块路径导出，避免同名 `hit_at` 冲突。
