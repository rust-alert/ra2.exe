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
    mul_div_round, DluRect, FontBaseUnits, LegacyReference, LegacyRole, LegacySource,
    MS_SANS_SERIF_8PT,
};
pub use spec::{LayoutId, LayoutNode};
pub use policy::{HorizontalRule, LayoutRules, SizeRule, VerticalRule};
pub use ui_layout::*;
pub use viewport::Viewport;
