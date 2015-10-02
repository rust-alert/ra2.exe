//! 呈现适配。

pub mod plan;
pub mod present;
pub mod rasterize;

pub use plan::{RenderCommand, RenderPlan};
pub use present::*;

