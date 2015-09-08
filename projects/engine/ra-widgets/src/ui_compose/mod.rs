//! 将已解码壳层精灵合成整页 RGBA（上传 `set_ui_page` 之前）。
//!
//! 合成 ≠ atlas/instance 终态；当前只为验证颜色、原尺寸与粗略位置。

use ra_assets::{CsfFile, FntFile};
use ra_renderer::RgbaImage;

use crate::{
    battle_hud::BattleHudChrome,
    skirmish_setup::{LOBBY_COLORS, LOBBY_DIFFICULTIES},
    ui_decode::{DecodedUiSprite, PageDecodeReport},
    ui_text::{
        MENU_TEXT_ACCENT, MENU_TEXT_DISABLED, MENU_TEXT_ENABLED, MENU_TEXT_SECTION, blit_caption_in_cell,
        blit_caption_top_left_clipped, blit_caption_wrapped, blit_text_colored, campaign_csf_label,
        campaign_difficulty_csf_key, campaign_title_csf_key, choose_map_csf_label, choose_map_static_csf_key,
        choose_map_title_csf_key, exit_confirm_csf_label, exit_confirm_prompt_csf_key, load_screen_brief_csf_key,
        load_screen_loading_csf_key, load_screen_name_csf_key, load_screen_special_unit_csf_key, main_menu_csf_label,
        options_csf_label, options_dialog_csf_key, battle_pause_menu_csf_label, resolve_caption, resolve_csf_text,
        single_player_csf_label, single_player_title_csf_key, skirmish_lobby_csf_label, skirmish_lobby_static_csf_key,
        skirmish_title_csf_key, LOAD_SCREEN_TEXT, LOAD_SCREEN_TEXT_TITLE,
    },
};
use ra_layout::{
    BUTTON_CELL_W, CAMPAIGN_BUTTON_IDS, CHOOSE_MAP_BUTTON_IDS, EXIT_CONFIRM_BUTTON_IDS, MAIN_MENU_BUTTON_IDS,
    MainMenuLayout, OPTIONS_BUTTON_IDS, BATTLE_PAUSE_MENU_BUTTON_IDS, RIGHT_PANEL_W, RectPx, SDWRNANM_OFFSET_X,
    SDWRNANM_OFFSET_Y, SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_CHECK_H, SKIRMISH_CHECK_W, SKIRMISH_COMBO_ARROW_RESERVE,
    SKIRMISH_COMBO_FACE_H, SKIRMISH_LOBBY_BUTTON_IDS, SKIRMISH_TRACK_ACTIVE_PAD, SKIRMISH_TRACK_PLAQUE_W,
    SKIRMISH_TRACK_THUMB_W, SkirmishLobbyLayout, LOAD_SCREEN_BUTTON_IDS, battle_hud_layout,
    battle_hud_layout_with_metrics, battle_pause_menu_layout, BattleHudChromeMetrics,
    campaign_layout, choose_map_layout, exit_confirm_layout, load_screen_layout, main_menu_layout,
    single_player_layout, skirmish_lobby_layout,
};

mod raster;
mod controls;
mod chrome;
mod menu;
mod campaign;
mod options;
mod exit_confirm;
mod skirmish;
mod choose_map;
mod load;
mod battle;

// 子模块通过 `use super::*` 共享 pub(super) 符号。
pub(super) use raster::*;
pub(super) use controls::*;
pub(super) use chrome::*;
pub(super) use menu::*;

pub use chrome::ShellWaveFrames;
pub use menu::{compose_main_menu_page, compose_single_player_page};
pub use campaign::{CampaignPaint, compose_campaign_page};
pub use options::compose_options_page;
pub use exit_confirm::compose_exit_confirm_page;
pub use skirmish::{SkirmishChromeSprites, SkirmishLobbyPaint, compose_skirmish_lobby_page};
pub use choose_map::{
    CHOOSE_MAP_LIST_ROW_H, choose_map_visible_rows, clamp_map_list_scroll, compose_choose_map_page, scroll_map_list_to_reveal,
};
pub use load::{LoadScreenPaint, compose_load_screen_page};
pub use battle::{BattleHudModel, compose_battle_hud_overlay, compose_battle_pause_menu_overlay};
pub use controls::paint_options_dialog_controls;
pub use raster::blit_rgba;
