//! UI 空间求解 + 壳层页面几何（壳层页面几何）。

#![allow(missing_docs)]

mod geometry;
mod viewport;
mod snapshot;
mod solver;
mod spec;
mod policy;
mod reference;

pub mod ui_layout;

pub use solver::LayoutEngine;
pub use geometry::{Insets, Point2, Rect, Size2};
pub use snapshot::{HitRegion, HitTestMode, LayoutBox, LayoutElement, LayoutSnapshot};
pub use reference::{
    battle_hud_layout_tree, battle_hud_layout_tree_with_metrics, campaign_content_layout_tree,
    dialog_layout_tree, exit_confirm_content_layout_tree, load_screen_layout_tree, mul_div_round,
    network_page_layout_tree, options_content_layout_tree, options_page_layout_tree,
    resolve_control_desc, resolve_dialog_template, right_rail_buttons_layout_tree,
    shell_chrome_layout_tree, shell_design_size, shell_page_layout_tree, solve_campaign,
    solve_choose_map, solve_dialog_template, solve_exit_confirm, solve_load_screen,
    solve_network_page, solve_options_page, solve_shell_page, solve_skirmish_lobby,
    BattleHudChromeMetrics, DluRect, FontBaseUnits, LegacyReference, LegacyRole, LegacySource,
    LOAD_SCREEN_BUTTON_IDS, MS_SANS_SERIF_8PT, NETWORK_BUTTON_IDS, OPTIONS_CONTENT_IDS,
    COMMAND_BAR_H, COMMAND_BUTTON_W, COMMAND_LENDCAP_W, COMMAND_RENDCAP_W,
};
pub use spec::{fixed_rect_leaf, root_with_fixed_children, LayoutId, LayoutNode};
pub use policy::{
    bottom_cover_button, right_panel_anchor, tile_snap_button, HorizontalRule, LayoutRules,
    RightPanelChrome, SizeRule, VerticalRule,
};
pub use ui_layout::*;
pub use viewport::Viewport;
