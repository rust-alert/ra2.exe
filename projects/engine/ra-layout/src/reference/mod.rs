//! 页面模板与 solve_* 入口（几何权威为 LayoutSnapshot）。

pub mod battle_hud;
mod battle_abort_confirm;
mod battle_in_game_options;
mod battle_pause;
pub mod campaign_page;
pub mod dlu;
pub mod exit_confirm_page;
pub mod from_template;
pub mod load_screen;
pub mod network_page;
pub mod options_page;
pub mod shell_chrome;
mod skirmish_score;

pub use battle_abort_confirm::{BATTLE_ABORT_CONFIRM_BUTTON_IDS, solve_battle_abort_confirm, solve_battle_abort_confirm_at};
pub use battle_hud::{
    BattleHudChromeMetrics, CAMEO_CELL_H, CAMEO_CELL_W, CAMEO_COLS, CAMEO_ROW_STRIDE, COMMAND_BAR_BUTTON_COUNT, COMMAND_BAR_BUTTON_IDS,
    COMMAND_BAR_H, COMMAND_BUTTON_W, COMMAND_LENDCAP_W, COMMAND_RENDCAP_W, SIDEBAR_TAB_COUNT, battle_hud_world_viewport, cameo_content_rect,
    cameo_slot_rect, cameo_visible_slot_count, hit_cameo_slot, solve_battle_hud, solve_battle_hud_with_metrics,
};
pub use battle_in_game_options::{BATTLE_IN_GAME_OPTIONS_BUTTON_IDS, solve_battle_in_game_options, solve_battle_in_game_options_at};
pub use battle_pause::{
    BATTLE_PAUSE_BASE_H, BATTLE_PAUSE_BASE_W, BATTLE_PAUSE_BUTTON_H, BATTLE_PAUSE_BUTTON_RIGHT_INSET, BATTLE_PAUSE_BUTTON_W,
    battle_sidebttn_rect, solve_battle_pause, solve_battle_pause_at,
};
pub use campaign_page::solve_campaign;
pub use dlu::{DluRect, FontBaseUnits, MS_SANS_SERIF_8PT, mul_div_round};
pub use exit_confirm_page::solve_exit_confirm;
pub use from_template::{solve_choose_map, solve_skirmish_lobby};
pub use load_screen::{LOAD_SCREEN_BUTTON_IDS, solve_load_screen};
pub use network_page::{NETWORK_BUTTON_IDS, solve_network_page};
pub use options_page::{OPTIONS_CONTENT_IDS, solve_options_page};
pub use shell_chrome::solve_shell_page;
pub use skirmish_score::{SCORE_ROW_SLOTS, solve_skirmish_score};
