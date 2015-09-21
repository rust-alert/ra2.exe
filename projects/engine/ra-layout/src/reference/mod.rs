//! 模板与遗留参照。

mod battle_hud;
mod campaign_page;
mod dlu;
mod exit_confirm_page;
mod from_template;
mod legacy;
mod load_screen;
mod network_page;
mod options_page;
mod shell_chrome;

pub use battle_hud::{
    battle_hud_layout_tree, battle_hud_layout_tree_with_metrics, solve_battle_hud,
    solve_battle_hud_with_metrics, BattleHudChromeMetrics, COMMAND_BAR_H, COMMAND_BUTTON_W,
    COMMAND_LENDCAP_W, COMMAND_RENDCAP_W,
};
pub use campaign_page::{campaign_content_layout_tree, solve_campaign};
pub use exit_confirm_page::{exit_confirm_content_layout_tree, solve_exit_confirm};
pub use dlu::{mul_div_round, DluRect, FontBaseUnits, MS_SANS_SERIF_8PT};
pub use from_template::{
    dialog_layout_tree, dialog_page_layout_tree, resolve_control_desc, resolve_dialog_template,
    shell_design_size, solve_choose_map, solve_dialog_template, solve_skirmish_lobby,
};
pub use legacy::{LegacyReference, LegacyRole, LegacySource};
pub use load_screen::{load_screen_layout_tree, solve_load_screen, LOAD_SCREEN_BUTTON_IDS};
pub use network_page::{network_page_layout_tree, solve_network_page, NETWORK_BUTTON_IDS};
pub use options_page::{
    options_content_layout_tree, options_page_layout_tree, solve_options_page, OPTIONS_CONTENT_IDS,
};
pub use shell_chrome::{
    right_rail_buttons_layout_tree, shell_chrome_layout_tree, shell_page_layout_tree,
    solve_shell_page,
};
