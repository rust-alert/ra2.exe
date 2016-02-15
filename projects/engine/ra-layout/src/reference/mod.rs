//! 页面模板与 solve_* 入口（几何权威为 LayoutSnapshot）。

mod battle_hud;
mod battle_pause;
mod campaign_page;
mod dlu;
mod exit_confirm_page;
mod from_template;
mod load_screen;
mod network_page;
mod options_page;
mod shell_chrome;
mod skirmish_score;

pub use battle_hud::{
    battle_hud_world_viewport, cameo_content_rect, cameo_slot_rect, cameo_visible_slot_count,
    hit_cameo_slot, solve_battle_hud, solve_battle_hud_with_metrics, BattleHudChromeMetrics,
    CAMEO_CELL_H, CAMEO_CELL_W, CAMEO_COLS, CAMEO_ROW_STRIDE, COMMAND_BAR_BUTTON_COUNT,
    COMMAND_BAR_BUTTON_IDS, COMMAND_BAR_H, COMMAND_BUTTON_W, COMMAND_LENDCAP_W, COMMAND_RENDCAP_W,
    SIDEBAR_TAB_COUNT,
};
pub use battle_pause::solve_battle_pause;
pub use campaign_page::solve_campaign;
pub use exit_confirm_page::solve_exit_confirm;
pub use dlu::{mul_div_round, DluRect, FontBaseUnits, MS_SANS_SERIF_8PT};
pub use from_template::{solve_choose_map, solve_skirmish_lobby};
pub use load_screen::{solve_load_screen, LOAD_SCREEN_BUTTON_IDS};
pub use network_page::{solve_network_page, NETWORK_BUTTON_IDS};
pub use options_page::{solve_options_page, OPTIONS_CONTENT_IDS};
pub use shell_chrome::solve_shell_page;
pub use skirmish_score::solve_skirmish_score;
