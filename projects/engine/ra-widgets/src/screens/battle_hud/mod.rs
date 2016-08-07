//! 战斗 HUD：chrome、解码、绘制与命中。

mod chrome;
mod command_bar;
mod decode;
mod hit_test;
mod layout;
mod render;

#[cfg(test)]
mod tests;

pub use chrome::{BattleHudChrome, BATTLE_HUD_PAL, COMMAND_BUTTON_SLOTS};
pub use decode::{
    decode_battle_hud_chrome, decode_battle_hud_chrome_resolved, decode_battle_hud_chrome_with, decode_cameo_sprite,
    radar_open_frame_range, RADAR_OPEN_FRAME_TICKS,
};
pub use hit_test::{hit_at, hit_at_with_chrome, BattleCameoPaint, BattleHudHit};
pub use render::{
    blit_battle_cameos, blit_battle_hud_chrome, blit_battle_hud_chrome_ex, blit_battle_hud_chrome_with_state, paint_battle_hud_chrome,
};
pub use render::blit_command_bar_track;
