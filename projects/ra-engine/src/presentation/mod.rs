//! 面向 UI、动画、音频与渲染的只读投影。

mod animation;
mod audio;
mod dirty;
mod hud;
mod render;
mod snapshot;

pub use dirty::DirtyEntitySet;
