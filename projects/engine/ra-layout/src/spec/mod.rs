//! 布局规格节点。

mod builder;
mod node;

pub use builder::{fixed_rect_leaf, root_with_fixed_children};
pub use node::{LayoutId, LayoutNode};
