//! `Shell` 构造。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use ra_renderer::Renderer;
use ra_types::{DisplayMode, PresentFeel};
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::skirmish_setup::SkirmishBootRequest;
use ra_widgets::startup_splash;
use ra_widgets::ui_typewriter::TypewriterText;

use crate::host::battle_controller::BattleController;
use crate::host::boot::BootResult;

use super::Shell;

impl Shell {
    /// 测试 / 已装载路径：直接进入对局页。
    #[cfg_attr(not(feature = "test-harness"), allow(dead_code))]
    pub(super) fn with_match(
        boot: BootResult,
        window_width: f64,
        window_height: f64,
        status_path: Option<PathBuf>,
        test_scene: Option<String>,
    ) -> Self {
        let mut renderer = Renderer::new();
        if let Some(image) = boot.preview.as_ref() {
            renderer.set_map_preview(image.clone());
        }
        let ctrl = BattleController::from_boot(boot, status_path.clone(), test_scene.clone());
        let screen = if ctrl.has_session() { OriginalScreen::Battle } else { OriginalScreen::MainMenu };
        Self {
            window: None,
            screen,
            battle_controller: Some(ctrl),
            renderer,
            banner: String::new(),
            window_width,
            window_height,
            display_mode: DisplayMode::DEFAULT,
            present: PresentFeel::DEFAULT,
            status_path,
            test_scene,
            startup_splash: None,
            splash_min_secs: startup_splash::DEFAULT_MINIMUM_VISIBLE_SECS,
            load_min_secs: 3.0,
            shell_slide_gap_secs: 0.2,
            splash_preload_done: false,
            splash_skip: false,
            pending_after_load: None,
            cursor: (0.0, 0.0),
            load_job: None,
            load_started: None,
            pending_load_boot: None,
            lobby_maps: Vec::new(),
            selected_map: None,
            lobby_preview_for: None,
            lobby_preview: None,
            lobby_preview_job: None,
            skirmish_chrome: None,
            skirmish_chrome_side: None,
            skirmish_pointer_consumed: false,
            menu_assets: None,
            ui_decode_cache: None,
            menu_pressed_entry: None,
            menu_pending_commit: None,
            menu_frame_wave: None,
            menu_slide_gap_until: None,
            menu_hovered_entry: None,
            status_line: TypewriterText::default(),
            menu_font: None,
            menu_font_tried: false,
            menu_csf: None,
            menu_csf_tried: false,
            menu_movie: None,
            menu_movie_clock: None,
            menu_panel_anim_clock: None,
            menu_panel_anim_accum: 0.0,
            menu_panel_anim_frame: 0,
            campaign_side_anim_clock: None,
            campaign_side_anim_accum: 0.0,
            campaign_side_anim_frame: 1,
            campaign_side_sfx: [None, None, None],
            pending_screenshot: None,
            #[cfg(feature = "test-harness")]
            auto_screenshots: super::screenshot::AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
            choose_map_revert: None,
            campaign_side: None,
            campaign_difficulty: 1,
            campaign_dragging: false,
            campaign_pointer_consumed: false,
            audio: crate::host::audio::ShellAudio::try_open(),
            menu_bgm: None,
            menu_bgm_tried: false,
            menu_click: None,
            menu_move_out: None,
            menu_move_out_tried: false,
            menu_move_in: None,
            menu_move_in_tried: false,
            menu_bgm_playing: false,
            audio_bag: None,
            audio_bag_tried: false,
            options_state: None,
            options_volume_baseline: None,
            options_present_baseline: None,
            options_pointer_consumed: false,
            last_shell_title: String::new(),
        }
    }

    /// 正常产品路径：闪屏 → 主菜单；进入对局须经菜单手动操作。
    pub(super) fn with_main_menu(display_mode: DisplayMode) -> Self {
        let (window_width, window_height) = {
            let (w, h) = display_mode.size();
            (w as f64, h as f64)
        };
        Self {
            window: None,
            screen: OriginalScreen::Splash,
            battle_controller: None,
            renderer: Renderer::new(),
            banner: "闪屏 · 预处理中".into(),
            window_width,
            window_height,
            display_mode,
            present: PresentFeel::DEFAULT,
            status_path: None,
            test_scene: None,
            startup_splash: None,
            splash_min_secs: startup_splash::DEFAULT_MINIMUM_VISIBLE_SECS,
            load_min_secs: 3.0,
            shell_slide_gap_secs: 0.2,
            splash_preload_done: false,
            splash_skip: false,
            pending_after_load: None,
            cursor: (0.0, 0.0),
            load_job: None,
            load_started: None,
            pending_load_boot: None,
            lobby_maps: Vec::new(),
            selected_map: None,
            lobby_preview_for: None,
            lobby_preview: None,
            lobby_preview_job: None,
            skirmish_chrome: None,
            skirmish_chrome_side: None,
            skirmish_pointer_consumed: false,
            menu_assets: None,
            ui_decode_cache: None,
            menu_pressed_entry: None,
            menu_pending_commit: None,
            menu_frame_wave: None,
            menu_slide_gap_until: None,
            menu_hovered_entry: None,
            status_line: TypewriterText::default(),
            menu_font: None,
            menu_font_tried: false,
            menu_csf: None,
            menu_csf_tried: false,
            menu_movie: None,
            menu_movie_clock: None,
            menu_panel_anim_clock: None,
            menu_panel_anim_accum: 0.0,
            menu_panel_anim_frame: 0,
            campaign_side_anim_clock: None,
            campaign_side_anim_accum: 0.0,
            campaign_side_anim_frame: 1,
            campaign_side_sfx: [None, None, None],
            pending_screenshot: None,
            #[cfg(feature = "test-harness")]
            auto_screenshots: super::screenshot::AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
            choose_map_revert: None,
            campaign_side: None,
            campaign_difficulty: 1,
            campaign_dragging: false,
            campaign_pointer_consumed: false,
            audio: crate::host::audio::ShellAudio::try_open(),
            menu_bgm: None,
            menu_bgm_tried: false,
            menu_click: None,
            menu_move_out: None,
            menu_move_out_tried: false,
            menu_move_in: None,
            menu_move_in_tried: false,
            menu_bgm_playing: false,
            audio_bag: None,
            audio_bag_tried: false,
            options_state: None,
            options_volume_baseline: None,
            options_present_baseline: None,
            options_pointer_consumed: false,
            last_shell_title: String::new(),
        }
    }
}
