//! 遭遇战大厅：模型、阵营 chrome、布局、命中与装载请求。

mod chrome;
mod controls;
mod hit_test;
mod layout;
mod load;
mod model;

pub use chrome::{UiFactionChrome, eva_fallback_sample_names, eva_known_event_index, eva_voice_stem_prefix, pick_side_flag_pcx};
pub use hit_test::hover_entry_at;
pub use load::{
    CAMPAIGN_LOAD_SCREEN_ALLIED_PAL, CAMPAIGN_LOAD_SCREEN_GENERIC_PAL, CAMPAIGN_LOAD_SCREEN_SOVIET_PAL, LOAD_SCREEN_FALLBACK_PAL,
    LOAD_SCREEN_PROGRESS_SHP, campaign_load_screen_palette_from_shp, campaign_load_screen_palette_resolved,
    campaign_score_screen_animation_candidates, campaign_score_screen_background_candidates, campaign_score_screen_background_shp,
    campaign_score_screen_palette, campaign_score_screen_palette_candidates, campaign_score_screen_transition_candidates,
    load_screen_background_shp_resolved, load_screen_brief_csf_key, load_screen_palette_resolved, score_screen_background_candidates,
    score_screen_background_shp, score_screen_palette, score_screen_palette_candidates,
};
pub use model::{
    LOBBY_COLORS, LOBBY_DIFFICULTIES, LOBBY_TEAM_COUNT, PLAYER_NAME_MAX_CHARS, SkirmishBootRequest, SkirmishCheckbox, SkirmishComboKind,
    SkirmishLobbyHit, SkirmishTrackbar,
};
