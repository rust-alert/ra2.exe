//! UI 空间求解 + 壳层页面几何（壳层页面几何）。

#![allow(missing_docs)]

mod geometry;
mod viewport;
mod snapshot;
mod solver;
mod spec;
mod policy;
mod reference;

mod shell;

pub use solver::LayoutEngine;
pub use geometry::{Insets, Point2, Rect, Size2};
pub use snapshot::{HitRegion, HitTestMode, LayoutBox, LayoutElement, LayoutSnapshot};
pub use reference::{
    battle_hud_world_viewport, cameo_content_rect, cameo_slot_rect, cameo_visible_slot_count,
    hit_cameo_slot, mul_div_round, solve_battle_hud, solve_battle_hud_with_metrics,
    solve_battle_pause, solve_campaign, solve_choose_map, solve_exit_confirm, solve_load_screen,
    solve_network_page, solve_options_page, solve_shell_page, solve_skirmish_lobby,
    BattleHudChromeMetrics, DluRect, FontBaseUnits, LOAD_SCREEN_BUTTON_IDS, MS_SANS_SERIF_8PT,
    NETWORK_BUTTON_IDS, OPTIONS_CONTENT_IDS, CAMEO_CELL_H, CAMEO_CELL_W, CAMEO_COLS,
    CAMEO_ROW_STRIDE, COMMAND_BAR_BUTTON_COUNT, COMMAND_BAR_BUTTON_IDS, COMMAND_BAR_H,
    COMMAND_BUTTON_W, COMMAND_LENDCAP_W, COMMAND_RENDCAP_W, SIDEBAR_TAB_COUNT,
};
pub use spec::{
    column, fixed_rect_leaf, root_with_children, root_with_fixed_children, row, sized_leaf, LayoutId,
    LayoutNode,
};
pub use policy::{
    bottom_cover_button, right_panel_anchor, tile_snap_button, HorizontalRule, LayoutFlow, LayoutRules,
    RightPanelChrome, SizeRule, VerticalRule,
};
pub use shell::*;
pub use viewport::Viewport;
