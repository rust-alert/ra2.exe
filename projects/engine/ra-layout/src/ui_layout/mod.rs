//! 壳层像素辅助：`RectPx`、常量、视口映射与列表/弹出几何。
//! 页面槽位权威为各 `solve_*` → `LayoutSnapshot`。

mod rect;
mod constants;
mod viewport_map;
mod popup;
mod choose_map;
mod battle_hud;
mod map_viewport;

pub use rect::*;
pub use constants::*;
pub use viewport_map::*;
pub use popup::*;
pub use choose_map::*;
pub use battle_hud::*;
pub use map_viewport::*;
