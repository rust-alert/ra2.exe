//! 战斗 HUD：chrome、解码、绘制与命中。

mod chrome;
mod command_bar;
mod decode;
mod hit_test;
mod layout;
mod power_meter;
mod radar_minimap;
mod render;

pub use chrome::{BATTLE_HUD_PAL, BattleHudChrome, COMMAND_BUTTON_SLOTS};
pub use decode::{
    RADAR_OPEN_FRAME_TICKS, decode_battle_hud_chrome, decode_battle_hud_chrome_resolved, decode_battle_hud_chrome_with, decode_cameo_sprite,
    radar_open_animation_done, radar_open_frame_index, radar_open_frame_range,
};
pub use hit_test::{BattleCameoPaint, BattleHudHit, hit_at, hit_at_with_chrome};
pub use power_meter::{
    POWER_METER_FULL_LEVEL, POWERP_FRAME_COUNT, POWERP_FRAME_GRAY, POWERP_FRAME_GREEN, POWERP_FRAME_RED, POWERP_FRAME_TRACK, POWERP_FRAME_YELLOW,
    PowerMeterColor, PowerMeterPaint, power_meter_paint,
};
pub use radar_minimap::{
    RADAR_CONTENT_INSET, RadarMinimapBlip, blit_radar_minimap_into_slot, compose_radar_minimap, land_type_radar_rgba, radar_content_rect,
    radar_fit_xy_to_cell, radar_minimap_fit_rect,
};
pub use render::{
    blit_battle_cameos, blit_battle_hud_chrome, blit_battle_hud_chrome_ex, blit_battle_hud_chrome_with_state, blit_battle_pause_hub_chrome,
    blit_command_bar_track, cameo_ready_flash_on, paint_battle_hud_chrome, paint_cameo_progress_clock,
};
