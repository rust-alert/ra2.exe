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
    dialog_0x102_layout_tree, dialog_0x6b_layout_tree, dialog_layout_tree, dialog_template_0x102,
    dialog_template_0x6b, mul_div_round, resolve_control_desc, resolve_dialog_0x102,
    resolve_dialog_0x6b, resolve_dialog_template, shell_design_size, DluRect, FontBaseUnits,
    LegacyReference, LegacyRole, LegacySource, MS_SANS_SERIF_8PT,
};
pub use spec::{fixed_rect_leaf, root_with_fixed_children, LayoutId, LayoutNode};
pub use policy::{
    bottom_cover_button, right_panel_anchor, tile_snap_button, HorizontalRule, LayoutRules,
    RightPanelChrome, SizeRule, VerticalRule,
};
pub use ui_layout::*;
pub use viewport::Viewport;
