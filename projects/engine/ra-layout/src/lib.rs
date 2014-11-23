//! UI 空间求解 + 壳层页面几何（壳层页面几何）。

#![allow(missing_docs)]

mod engine;
mod geometry;
mod hit;
mod legacy;
mod node;
mod rules;
mod snapshot;
mod viewport;

pub mod ui_layout;

pub use engine::LayoutEngine;
pub use geometry::{Insets, Point2, Rect, Size2};
pub use hit::{HitRegion, HitTestMode};
pub use legacy::{LegacyReference, LegacyRole, LegacySource};
pub use node::{LayoutId, LayoutNode};
pub use rules::{HorizontalRule, LayoutRules, SizeRule, VerticalRule};
pub use snapshot::{LayoutBox, LayoutElement, LayoutSnapshot};
pub use ui_layout::*;
pub use viewport::Viewport;
