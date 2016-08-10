//! 战斗 HUD：chrome、解码、绘制与命中。

mod chrome;
mod command_bar;
mod decode;
mod hit_test;
mod layout;
mod render;

pub use chrome::{BATTLE_HUD_PAL, BattleHudChrome, COMMAND_BUTTON_SLOTS};
pub use decode::{
    RADAR_OPEN_FRAME_TICKS, decode_battle_hud_chrome, decode_battle_hud_chrome_resolved, decode_battle_hud_chrome_with, decode_cameo_sprite,
    radar_open_frame_range,
};
pub use hit_test::{BattleCameoPaint, BattleHudHit, hit_at, hit_at_with_chrome};
pub use render::{
    blit_battle_cameos, blit_battle_hud_chrome, blit_battle_hud_chrome_ex, blit_battle_hud_chrome_with_state, blit_command_bar_track,
    cameo_ready_flash_on, paint_battle_hud_chrome, paint_cameo_progress_clock,
};
