//! 壳层页面几何（过渡期页面布局 API）。

mod rect;
mod constants;
mod viewport_map;
mod main_menu;
mod skirmish;
mod campaign;
mod exit_confirm;
mod choose_map;
mod battle_hud;
mod battle_pause;
mod load_screen;
mod map_viewport;

pub use rect::*;
pub use constants::*;
pub use viewport_map::*;
pub use main_menu::*;
pub use skirmish::*;
pub use campaign::*;
pub use exit_confirm::*;
pub use choose_map::*;
pub use battle_hud::*;
pub use battle_pause::*;
pub use load_screen::*;
pub use map_viewport::*;
