//! 模板与遗留参照。

mod battle_hud;
mod campaign_page;
mod dlu;
mod exit_confirm_page;
mod from_template;
mod legacy;
mod shell_chrome;

pub use battle_hud::battle_hud_layout_tree;
pub use campaign_page::campaign_content_layout_tree;
pub use exit_confirm_page::exit_confirm_content_layout_tree;
pub use dlu::{mul_div_round, DluRect, FontBaseUnits, MS_SANS_SERIF_8PT};
pub use from_template::{
    dialog_layout_tree, resolve_control_desc, resolve_dialog_template, shell_design_size,
};
pub use legacy::{LegacyReference, LegacyRole, LegacySource};
pub use shell_chrome::{right_rail_buttons_layout_tree, shell_chrome_layout_tree};
