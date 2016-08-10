//! 壳层像素辅助：`RectPx`、常量、视口映射与列表/弹出几何。
//! 页面槽位权威为各 `solve_*` → `LayoutSnapshot`。

mod choose_map;
mod constants;
pub mod map_viewport;
pub mod popup;
mod rect;
mod viewport_map;

pub use choose_map::*;
pub use constants::*;
pub use map_viewport::*;
pub use popup::*;
pub use rect::*;
pub use viewport_map::*;
