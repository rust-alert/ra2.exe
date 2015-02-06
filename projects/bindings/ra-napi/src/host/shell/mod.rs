//! 壳层会话与平台宿主：页面导航、窗口生命周期；对局逻辑委托 `BattleController`。

use std::{path::PathBuf, sync::Arc, time::{Duration, Instant}};

use ra_assets::{AudioIndex, CsfFile, FntFile, IniDocument, PcmAudio, decode_audio_bytes};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::{AssetSource, DisplayMode, PresentFeel, RaError, RaResult};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use super::{
    boot::BootResult,
    load_job::LoadJob,
    battle_controller::{BattleController, BattleNav},
    preview_job::PreviewJob,
};
use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    shell_slide::{
        CAMPAIGN_SLIDE, CHOOSE_MAP_SLIDE, MAIN_MENU_SLIDE, SINGLE_PLAYER_SLIDE, SKIRMISH_SLIDE, ShellFrameWave, ShellSlideSpec,
        WAVE_STOWED_FRAME, WaveDirection,
    },
    skirmish_setup::{SkirmishBootRequest, hover_entry_at, side_flag_pcx},
    startup_splash::{self, StartupSplashPresentation},
    ui_assets::{MenuUiAssets, load_menu_ui_assets},
    ui_compose::{self, SkirmishChromeSprites},
    ui_decode, ui_hit,
    ui_movie::MenuMoviePlayer,
    ui_page::{page_resources_for_load_screen, page_resources_from_slots_with_edition},
    ui_present, ui_resolve,
    ui_text::{campaign_csf_tooltip, main_menu_csf_tooltip, resolve_csf_text, single_player_csf_tooltip, skirmish_lobby_csf_tooltip},
    ui_typewriter::TypewriterText,
};
use ra_layout::ui_layout;

/// 外壳持有的可导航壳层会话状态。
pub struct Shell {
    pub(super) window: Option<Arc<Window>>,
    pub(super) screen: OriginalScreen,
    /// 对局 / 结算页控制器；菜单页可为空。
    pub(super) battle_controller: Option<BattleController>,
    pub(super) renderer: Renderer,
    /// 菜单或装载说明。
    pub(super) banner: String,
    pub(super) window_width: f64,
    pub(super) window_height: f64,
    /// 客户区分辨率档（布局与缓冲基准，非自由拉伸）。
    pub(super) display_mode: DisplayMode,
    /// 壳层质感呈现（来自 `RustAlert.toml` `[present]`）。
    pub(super) present: PresentFeel,
    pub(super) status_path: Option<PathBuf>,
    pub(super) test_scene: Option<String>,
    /// 进程启动闪屏 presentation（独立 owner；非菜单槽）。
    pub(super) startup_splash: Option<StartupSplashPresentation>,
    /// 闪屏最短展示秒数（首次成功 present 后起算；可调，默认 3）。
    pub(super) splash_min_secs: f64,
    /// 遭遇战装载页最短展示秒数（`RustAlert.toml` 的 `load_min_secs`，默认 3；`0` 关闭）。
    pub(super) load_min_secs: f64,
    /// 壳层切页出去→进来之间的停顿秒数（模拟原版重型机械卡顿；`0` 关闭）。
    pub(super) shell_slide_gap_secs: f64,
    /// 闪屏预处理是否完成。
    pub(super) splash_preload_done: bool,
    /// 用户请求跳过闪屏（仍须预处理完成才进主菜单）。
    pub(super) splash_skip: bool,
    /// 装载完成后待切到的目标页。
    pub(super) pending_after_load: Option<OriginalScreen>,
    /// 光标位置（逻辑像素，与 `window_width` / `window_height` 同单位）。
    pub(super) cursor: (f64, f64),
    /// 后台遭遇战装载（`LoadScreen` 期间轮询）。
    pub(super) load_job: Option<LoadJob>,
    /// 当前装载开始时刻。
    pub(super) load_started: Option<Instant>,
    /// 后台已完成、等待最短展示时间后再 `finish_load` 的结果。
    pub(super) pending_load_boot: Option<BootResult>,
    /// 遭遇战大厅可选地图。
    pub(super) lobby_maps: Vec<super::boot::BootMapCandidate>,
    /// 当前选中的地图文件名。
    pub(super) selected_map: Option<String>,
    /// 大厅缩略图对应的地图名（与 `lobby_preview` 配对）。
    pub(super) lobby_preview_for: Option<String>,
    /// 已缩小的选中地图预览。
    pub(super) lobby_preview: Option<RgbaImage>,
    /// 后台地图预览任务。
    pub(super) lobby_preview_job: Option<PreviewJob>,
    /// 遭遇战控件 PCX 缓存（勾选/滑条拇指/旗标）。
    pub(super) skirmish_chrome: Option<SkirmishChromeSprites>,
    /// 旗标缓存对应的阵营名（换边时重载）。
    pub(super) skirmish_chrome_side: Option<String>,
    /// 遭遇战左栏按下是否已消费（勾选/滑条，勿再走右栏按钮命中）。
    pub(super) skirmish_pointer_consumed: bool,
    /// 主菜单阶段已挂载资源（惰性一次）。
    pub(super) menu_assets: Option<MenuUiAssets>,
    /// 当前页 chrome 解码缓存（切换页或重探时刷新）。
    pub(super) ui_decode_cache: Option<ui_decode::PageDecodeReport>,
    /// 主菜单当前按住的按钮入口 id（按下帧合成）。
    pub(super) menu_pressed_entry: Option<&'static str>,
    /// 切页排队：`SlideOut` 完成后提交的 `MenuAction`。
    pub(super) menu_pending_commit: Option<MenuAction>,
    /// 进行中的右栏 `SDBTNANM` 帧波浪（出去 / 进来）。
    pub(super) menu_frame_wave: Option<ShellFrameWave>,
    /// 出去结束后、进来开始前的卡顿截止时刻（无字、钮面收起）。
    pub(super) menu_slide_gap_until: Option<Instant>,
    /// 主菜单当前悬停的按钮入口 id（悬停帧合成）。
    pub(super) menu_hovered_entry: Option<&'static str>,
    /// 底栏状态提示打字机（与按钮 hover 图解耦；亦可复用于局内右上消息）。
    pub(super) status_line: TypewriterText,
    /// 菜单字体（`game.fnt`）。
    pub(super) menu_font: Option<FntFile>,
    /// 是否已尝试装载菜单字体（失败后不再每帧读盘/打日志）。
    pub(super) menu_font_tried: bool,
    /// 菜单文案表（`ra2.csf` / `ra2md.csf`）。
    pub(super) menu_csf: Option<CsfFile>,
    /// 是否已尝试装载菜单文案表。
    pub(super) menu_csf_tried: bool,
    /// 主菜单 / 单人页循环影片。
    pub(super) menu_movie: Option<MenuMoviePlayer>,
    /// 影片时钟（`tick` 用）。
    pub(super) menu_movie_clock: Option<Instant>,
    /// WARNING 窗内 `sdwrnanm` 动画时钟（与侧图箭头分离）。
    pub(super) menu_panel_anim_clock: Option<Instant>,
    /// WARNING 动画未消耗的累计秒。
    pub(super) menu_panel_anim_accum: f64,
    /// `sdwrnanm` 帧序号（对解码帧数取模）。
    pub(super) menu_panel_anim_frame: usize,
    /// 战役侧图箭头动画时钟。
    pub(super) campaign_side_anim_clock: Option<Instant>,
    /// 侧图动画累计秒。
    pub(super) campaign_side_anim_accum: f64,
    /// 侧图箭头帧序号。
    pub(super) campaign_side_anim_frame: usize,
    /// 战役选边悬停语音（`AlliedCampaignSelect` 等，惰性）。
    pub(super) campaign_side_sfx: [Option<PcmAudio>; 3],
    /// 下一帧回读后落盘的截图短名（`OriginalScreen::as_str`）；F12 手动截图用。
    pub(super) pending_screenshot: Option<&'static str>,
    /// 自动关键页截图去重（仅 `test-harness`）。
    #[cfg(feature = "test-harness")]
    pub(super) auto_screenshots: super::screenshot::AutoScreenshotTracker,
    /// 遭遇战大厅阵营 / 难度（进入装载请求）。
    pub(super) skirmish: SkirmishBootRequest,
    /// 进入选图页前的 `preferred_map` 快照（取消时还原）。
    pub(super) choose_map_revert: Option<Option<String>>,
    /// 战役选边：`allied` / `tutorial` / `soviet`。
    pub(super) campaign_side: Option<&'static str>,
    /// 战役难度档：0 易 / 1 中 / 2 难。
    pub(super) campaign_difficulty: u8,
    /// 战役难度滑条是否正在拖动。
    pub(super) campaign_dragging: bool,
    /// 战役左栏按下是否已消费（难度滑条，勿再走点击轮换）。
    pub(super) campaign_pointer_consumed: bool,
    /// 桌面音频输出（设备不可用则为 `None`）。
    pub(super) audio: Option<super::audio::ShellAudio>,
    /// 主菜单 BGM PCM（`theme.ini` `[INTRO]` → `{Sound}.wav`）。
    pub(super) menu_bgm: Option<PcmAudio>,
    /// 是否已尝试装载菜单 BGM（失败后不再每帧重试）。
    pub(super) menu_bgm_tried: bool,
    /// 菜单点击音效 PCM（`GUIMainButtonSound` → `sound.ini` → `audio.bag`）。
    pub(super) menu_click: Option<PcmAudio>,
    /// 壳层出去音效（`GUIMoveOutSound` → 默认 `MenuSlideOut` / `uslide2`）。
    pub(super) menu_move_out: Option<PcmAudio>,
    /// 是否已尝试装载出去音效。
    pub(super) menu_move_out_tried: bool,
    /// 壳层进来音效（`GUIMoveInSound` → 默认 `MenuSlideIn` / `uslide1`）。
    pub(super) menu_move_in: Option<PcmAudio>,
    /// 是否已尝试装载进来音效。
    pub(super) menu_move_in_tried: bool,
    /// 当前是否已在播壳层 BGM。
    pub(super) menu_bgm_playing: bool,
    /// 已解析的 `audio.bag` 索引（惰性）。
    pub(super) audio_bag: Option<AudioIndex>,
    /// 是否已尝试装载 `audio.bag`（避免反复读盘）。
    pub(super) audio_bag_tried: bool,
    /// 选项页草稿（进入 Options 时创建，接受/取消后清空）。
    pub(super) options_state: Option<ra_widgets::options_dialog::OptionsDialogState>,
    /// 进入选项页时的音量快照（取消时还原实时预览）。
    pub(super) options_volume_baseline: Option<(f32, f32)>,
    /// 进入选项页时的质感快照（取消时还原实时预览）。
    pub(super) options_present_baseline: Option<PresentFeel>,
    /// 本轮按下已由左栏控件消费（释放时勿再走右栏命中）。
    pub(super) options_pointer_consumed: bool,
    /// 上次已写入的窗口标题（避免每帧 `set_title` 卡顿）。
    pub(super) last_shell_title: String,
}

impl Shell {
    /// 测试 / 已装载路径：直接进入对局页。
    #[cfg_attr(not(feature = "test-harness"), allow(dead_code))]
    pub fn with_match(
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
            audio: super::audio::ShellAudio::try_open(),
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
    pub fn with_main_menu(display_mode: DisplayMode) -> Self {
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
            audio: super::audio::ShellAudio::try_open(),
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

    /// 上传 UI 页：先按 `[present]` 做质感变换再进 GPU。
    fn upload_ui_page(&mut self, page: RgbaImage) {
        let page = ui_present::present_ui_page(page, self.present);
        self.renderer.set_ui_page(page);
    }

    fn shell_cursor_px(&self) -> (i32, i32) {
        ui_layout::window_to_shell_px(self.cursor.0, self.cursor.1, self.window_width, self.window_height)
    }

    fn set_screen(&mut self, next: OriginalScreen) {
        if self.screen != next {
            if self.screen == OriginalScreen::SkirmishLobby {
                self.skirmish.end_name_edit();
                self.skirmish.close_combo();
            }
            tracing::info!("页面 {} → {}", self.screen.as_str(), next.as_str());
            self.screen = next;
            self.menu_pressed_entry = None;
            self.menu_pending_commit = None;
            self.menu_hovered_entry = None;
            self.status_line.clear();
            // `menu_frame_wave` / `menu_slide_gap_until` 由切页状态机显式启停，不在此清空。
            if !matches!(
                next,
                OriginalScreen::MainMenu | OriginalScreen::SinglePlayerMenu | OriginalScreen::Options | OriginalScreen::ExitConfirm
            ) {
                self.menu_movie = None;
                self.menu_movie_clock = None;
            }
            self.sync_shell_audio();
            self.refresh_ui_resolve_note();
            self.refresh_menu_backdrop();
            self.refresh_shell_title();
            #[cfg(feature = "test-harness")]
            if self.auto_screenshots.should_capture(next) {
                self.queue_screenshot(next.as_str());
            }
        }
    }

    /// 前置页：主菜单 / 单人 / 选项 / 遭遇战大厅 / 装载页上传合成 chrome；启动闪屏由独立 owner 保持。
    fn refresh_menu_backdrop(&mut self) {
        if matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
            // 对局 HUD 由 `BattleController::draw_frame` 维护，勿在此清空；仍预热字体。
            self.ensure_menu_assets();
            self.ensure_menu_text_assets();
            return;
        }
        // 启动闪屏禁止走菜单合成路径，更不能 clear 掉已上传的 GLSS/GLSL 画面。
        if self.screen == OriginalScreen::Splash {
            self.ensure_startup_splash_presented();
            return;
        }
        self.ensure_menu_assets();
        self.ensure_menu_text_assets();
        if matches!(
            self.screen,
            OriginalScreen::MainMenu
                | OriginalScreen::SinglePlayerMenu
                | OriginalScreen::Campaign
                | OriginalScreen::Options
                | OriginalScreen::ExitConfirm
                | OriginalScreen::SkirmishLobby
                | OriginalScreen::ChooseMap
                | OriginalScreen::LoadScreen
        ) {
            if matches!(self.screen, OriginalScreen::SkirmishLobby | OriginalScreen::ChooseMap) {
                self.ensure_lobby_maps();
                self.ensure_lobby_preview();
            }
            if matches!(self.screen, OriginalScreen::Campaign | OriginalScreen::SkirmishLobby) {
                self.ensure_skirmish_chrome();
            }
            // 大厅预览并入 UI 页合成，避免与 `set_map_preview` 双通道抢相机。
            self.renderer.clear_preview();
            let load_allow_retry = self.load_allow_retry();
            let load_status = if self.screen == OriginalScreen::LoadScreen { Some(self.banner.clone()) } else { None };
            let load_progress = self.load_screen_progress();
            let wave_owned = self.current_wave_frames();
            let wave = wave_owned.as_ref().map(|(buttons, tiles)| {
                let animate_empty_tiles =
                    self.menu_frame_wave.as_ref().is_some_and(|w| w.direction() == WaveDirection::SlideOut);
                ui_compose::ShellWaveFrames {
                    buttons: buttons.as_slice(),
                    tiles: tiles.as_slice(),
                    animate_empty_tiles,
                }
            });
            if let Some(decoded) = self.ui_decode_cache.as_ref() {
                let movie = self.menu_movie.as_ref().and_then(|m| m.frame());
                let page = match self.screen {
                    OriginalScreen::MainMenu => ui_compose::compose_main_menu_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.status_line_visible(),
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        movie,
                        wave,
                        self.menu_panel_anim_frame,
                    ),
                    OriginalScreen::SinglePlayerMenu => ui_compose::compose_single_player_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.status_line_visible(),
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        movie,
                        wave,
                        self.menu_panel_anim_frame,
                    ),
                    OriginalScreen::Campaign => {
                        let track_thumb = self.skirmish_chrome.as_ref().and_then(|c| c.track_thumb.as_ref());
                        ui_compose::compose_campaign_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.status_line_visible(),
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            ui_compose::CampaignPaint {
                                selected_side: self.campaign_side,
                                difficulty: self.campaign_difficulty,
                                track_thumb,
                                side_anim_frame: self.campaign_side_anim_frame.max(1),
                            },
                            wave,
                            self.menu_panel_anim_frame,
                        )
                    }
                    OriginalScreen::Options => self.options_state.as_ref().and_then(|state| {
                        ui_compose::compose_options_page(
                            decoded,
                            state,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            movie,
                            self.menu_panel_anim_frame,
                        )
                    }),
                    OriginalScreen::ExitConfirm => ui_compose::compose_exit_confirm_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        movie,
                        self.menu_panel_anim_frame,
                    ),
                    OriginalScreen::SkirmishLobby => {
                        let map_name =
                            self.selected_map.clone().or_else(|| self.lobby_maps.first().map(|m| m.file_name.clone())).unwrap_or_default();
                        let country = self.skirmish.side.clone();
                        let ai_csf = ra_widgets::skirmish_setup::SkirmishBootRequest::ai_difficulty_csf_key(&self.skirmish.difficulty);
                        let ai_name = self
                            .menu_csf
                            .as_ref()
                            .and_then(|c| c.get(ai_csf).map(|s| s.to_string()))
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| self.skirmish.difficulty.clone());
                        let ai_rows = self.lobby_ai_rows();
                        let paint = ui_compose::SkirmishLobbyPaint {
                            map_name: map_name.as_str(),
                            player_name: self.skirmish.player_name.as_str(),
                            country_name: country.as_str(),
                            color_rgb: self.skirmish.color_rgb(),
                            ai_name: ai_name.as_str(),
                            ai_country: self.skirmish.row_side(1),
                            ai_difficulty: self.skirmish.difficulty.as_str(),
                            ai_rows,
                            short_game: self.skirmish.short_game,
                            mcv_repacks: self.skirmish.mcv_repacks,
                            crates: self.skirmish.crates,
                            superweapons: self.skirmish.superweapons,
                            build_off_ally: self.skirmish.build_off_ally,
                            game_speed: self.skirmish.game_speed,
                            credits: self.skirmish.credits,
                            unit_count: self.skirmish.unit_count,
                            player_name_editing: self.skirmish.player_name_editing,
                            country_combo_open: self.skirmish.open_combo == Some(ra_widgets::skirmish_setup::SkirmishComboKind::Country),
                            color_combo_open: self.skirmish.open_combo == Some(ra_widgets::skirmish_setup::SkirmishComboKind::Color),
                            ai_combo_open: self.skirmish.open_combo == Some(ra_widgets::skirmish_setup::SkirmishComboKind::Ai),
                            combo_row: self.skirmish.combo_row,
                            row_side_indices: self.skirmish.row_sides,
                            row_color_indices: self.skirmish.row_colors,
                            chrome: self.skirmish_chrome.as_ref(),
                        };
                        ui_compose::compose_skirmish_lobby_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.status_line_visible(),
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            self.lobby_preview.as_ref(),
                            &paint,
                            wave,
                            0,
                        )
                    }
                    OriginalScreen::ChooseMap => {
                        let map_names: Vec<&str> = self.lobby_maps.iter().map(|m| m.file_name.as_str()).collect();
                        let selected_map_index =
                            self.selected_map.as_ref().and_then(|sel| self.lobby_maps.iter().position(|m| &m.file_name == sel));
                        ui_compose::compose_choose_map_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.status_line_visible(),
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            self.lobby_preview.as_ref(),
                            &map_names,
                            selected_map_index,
                            wave,
                            0,
                        )
                    }
                    OriginalScreen::LoadScreen => ui_compose::compose_load_screen_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        ui_compose::LoadScreenPaint {
                            side: self.skirmish.side.as_str(),
                            player_name: self.skirmish.player_name.as_str(),
                            side_flag: self.skirmish_chrome.as_ref().and_then(|c| c.row_flags[0].as_ref()),
                            status: load_status.as_deref().unwrap_or(""),
                            allow_retry: load_allow_retry,
                            progress: load_progress,
                        },
                    ),
                    _ => None,
                };
                if let Some(page) = page {
                    tracing::debug!(screen = self.screen.as_str(), w = page.width(), h = page.height(), "壳层 chrome 已合成并上传 UI 页通道");
                    self.upload_ui_page(page);
                    if self.screen != OriginalScreen::LoadScreen && !self.banner.contains("chrome 已上传") {
                        self.banner = format!("{} · chrome 已上传", self.banner);
                        self.refresh_shell_title();
                    }
                    return;
                }
            }
            // 装载页即使 chrome 未解码也要画出可读状态，禁止纯色空窗。
            if self.screen == OriginalScreen::LoadScreen {
                let empty = ui_decode::PageDecodeReport {
                    background: None,
                    panels: Vec::new(),
                    button_normals: Vec::new(),
                    button_hovers: Vec::new(),
                    button_presseds: Vec::new(),
                    sdbtnanm_frames: Vec::new(),
                    errors: Vec::new(),
                };
                if let Some(page) = ui_compose::compose_load_screen_page(
                    &empty,
                    self.window_width as u32,
                    self.window_height as u32,
                    self.menu_pressed_entry,
                    self.menu_hovered_entry,
                    self.menu_font.as_ref(),
                    self.menu_csf.as_ref(),
                    ui_compose::LoadScreenPaint {
                        side: self.skirmish.side.as_str(),
                        player_name: self.skirmish.player_name.as_str(),
                        side_flag: self.skirmish_chrome.as_ref().and_then(|c| c.row_flags[0].as_ref()),
                        status: load_status.as_deref().unwrap_or(self.banner.as_str()),
                        allow_retry: load_allow_retry,
                        progress: load_progress,
                    },
                ) {
                    self.upload_ui_page(page);
                    return;
                }
            }
            self.renderer.clear_ui_page();
            return;
        }

        self.renderer.clear_ui_page();
        self.renderer.clear_preview();
    }

    fn load_allow_retry(&self) -> bool {
        self.load_job.is_none() && self.pending_load_boot.is_none()
    }

    /// 装载页进度：进行中读任务 ratio；最短展示等待或失败后视为满格。
    fn load_screen_progress(&self) -> f32 {
        if self.pending_load_boot.is_some() {
            return 1.0;
        }
        if let Some(job) = self.load_job.as_ref() {
            return job.progress().ratio.clamp(0.0, 1.0);
        }
        if self.screen == OriginalScreen::LoadScreen && self.load_allow_retry() {
            return 1.0;
        }
        0.0
    }

    /// 当前光标下的可点按钮入口（逻辑窗口坐标）。
    fn menu_entry_under_cursor(&self) -> Option<&'static str> {
        if self.screen == OriginalScreen::Campaign {
            return ui_hit::campaign_entry_at(self.cursor.0, self.cursor.1, self.window_width, self.window_height);
        }
        if self.screen == OriginalScreen::SkirmishLobby {
            if let Some(id) = ui_hit::hover_index(
                self.screen,
                &self.lobby_maps,
                self.selected_map.as_deref(),
                self.cursor,
                self.window_width,
                self.window_height,
                self.load_allow_retry(),
            )
            .and_then(|idx| ui_layout::SKIRMISH_LOBBY_BUTTON_IDS.get(idx).copied())
            {
                return Some(id);
            }
            let layout = ui_layout::skirmish_lobby_layout(0, 0);
            let (x, y) = self.shell_cursor_px();
            return hover_entry_at(&layout, x, y);
        }
        let idx = ui_hit::hover_index(
            self.screen,
            &self.lobby_maps,
            self.selected_map.as_deref(),
            self.cursor,
            self.window_width,
            self.window_height,
            self.load_allow_retry(),
        )?;
        match self.screen {
            OriginalScreen::MainMenu => ui_layout::MAIN_MENU_BUTTON_IDS.get(idx).copied(),
            OriginalScreen::SinglePlayerMenu => ui_layout::SINGLE_PLAYER_BUTTON_IDS.get(idx).copied(),
            OriginalScreen::Options => ui_layout::OPTIONS_BUTTON_IDS.get(idx).copied(),
            OriginalScreen::ExitConfirm => ui_layout::EXIT_CONFIRM_BUTTON_IDS.get(idx).copied(),
            OriginalScreen::ChooseMap => ui_layout::CHOOSE_MAP_BUTTON_IDS.get(idx).copied(),
            _ => None,
        }
    }

    fn apply_menu_action(&mut self, event_loop: &ActiveEventLoop, action: MenuAction) {
        self.request_menu_action(event_loop, action);
    }

    /// 是否为会换壳层页的导航动作（才走出去 / 进来波浪）。
    fn action_uses_shell_slide(action: MenuAction) -> bool {
        matches!(
            action,
            MenuAction::OpenSinglePlayer
                | MenuAction::OpenNetwork
                | MenuAction::OpenOptions
                | MenuAction::Exit
                | MenuAction::OpenSkirmish
                | MenuAction::OpenCampaign
                | MenuAction::Back
                | MenuAction::StartSkirmish
                | MenuAction::OptionsAccept
                | MenuAction::OptionsCancel
                | MenuAction::ChooseMap
                | MenuAction::UseMap
        )
    }

    /// 当前页若在允许列表内，返回波浪规格。
    fn slide_spec_for(screen: OriginalScreen) -> Option<ShellSlideSpec> {
        match screen {
            OriginalScreen::MainMenu => Some(MAIN_MENU_SLIDE),
            OriginalScreen::SinglePlayerMenu => Some(SINGLE_PLAYER_SLIDE),
            OriginalScreen::SkirmishLobby => Some(SKIRMISH_SLIDE),
            OriginalScreen::Campaign => Some(CAMPAIGN_SLIDE),
            OriginalScreen::ChooseMap => Some(CHOOSE_MAP_SLIDE),
            _ => None,
        }
    }

    /// 当前页参与波浪的按钮 id 表。
    fn wave_button_ids(screen: OriginalScreen) -> Option<&'static [&'static str]> {
        match screen {
            OriginalScreen::MainMenu => Some(&ui_layout::MAIN_MENU_BUTTON_IDS),
            OriginalScreen::SinglePlayerMenu => Some(&ui_layout::SINGLE_PLAYER_BUTTON_IDS),
            OriginalScreen::SkirmishLobby => Some(&ui_layout::SKIRMISH_LOBBY_BUTTON_IDS),
            OriginalScreen::Campaign => Some(&ui_layout::CAMPAIGN_BUTTON_IDS),
            OriginalScreen::ChooseMap => Some(&ui_layout::CHOOSE_MAP_BUTTON_IDS),
            _ => None,
        }
    }

    /// 当前页壳层 chrome 布局（波浪按物理平铺格取帧）。
    fn wave_shell_layout(screen: OriginalScreen) -> Option<ui_layout::MainMenuLayout> {
        Some(match screen {
            OriginalScreen::MainMenu => ui_layout::main_menu_layout(0, 0),
            OriginalScreen::SinglePlayerMenu => ui_layout::single_player_layout(0, 0),
            OriginalScreen::SkirmishLobby => ui_layout::skirmish_lobby_layout(0, 0).shell,
            OriginalScreen::Campaign => ui_layout::campaign_layout(0, 0).shell,
            OriginalScreen::ChooseMap => ui_layout::choose_map_layout(0, 0).shell,
            _ => return None,
        })
    }

    /// 按钮格相对 `panel_tile` 的平铺下标（贴底 Exit/返回落在末格）。
    fn panel_tile_index(layout: &ui_layout::MainMenuLayout, cell: ui_layout::RectPx) -> u32 {
        let tile_h = layout.panel_tile.h.max(1);
        ((cell.y - layout.panel_tile.y) / tile_h).max(0) as u32
    }

    /// 切页波浪或出去→进来卡顿进行中。
    fn shell_slide_busy(&self) -> bool {
        self.menu_frame_wave.is_some() || self.menu_slide_gap_until.is_some()
    }

    /// 合成用收起帧（卡顿间隙：钮面停在 `WAVE_STOWED_FRAME`，不叠字）。
    fn stowed_wave_frames(&self) -> Option<(Vec<u16>, Vec<u16>)> {
        let ids = Self::wave_button_ids(self.screen)?;
        let layout = Self::wave_shell_layout(self.screen)?;
        let buttons = vec![WAVE_STOWED_FRAME; ids.len()];
        let tiles = vec![WAVE_STOWED_FRAME; layout.panel_tile_count.max(0) as usize];
        Some((buttons, tiles))
    }

    /// 合成用：按钮帧 + 空格平铺帧；无波浪且无卡顿时为 `None`。
    /// 帧序按物理格自上而下统一交错，末钮与中间无字格同一波浪。
    fn current_wave_frames(&self) -> Option<(Vec<u16>, Vec<u16>)> {
        if self.menu_slide_gap_until.is_some() {
            return self.stowed_wave_frames();
        }
        let wave = self.menu_frame_wave.as_ref()?;
        let ids = Self::wave_button_ids(self.screen)?;
        let layout = Self::wave_shell_layout(self.screen)?;
        let buttons = ids
            .iter()
            .enumerate()
            .map(|(i, _id)| {
                let cell = layout.buttons.get(i).copied().unwrap_or(ui_layout::RectPx::new(0, 0, 0, 0));
                let ti = if cell.w > 0 && cell.h > 0 {
                    Self::panel_tile_index(&layout, cell)
                } else {
                    i as u32
                };
                wave.frame_for_slot(ti)
            })
            .collect::<Vec<_>>();
        let tile_count = layout.panel_tile_count.max(0) as u32;
        let tiles = (0..tile_count).map(|ti| wave.frame_for_slot(ti)).collect::<Vec<_>>();
        Some((buttons, tiles))
    }

    /// 新页进场波浪（仅当目标页有规格且当前无波浪 / 卡顿）。
    fn maybe_start_slide_in(&mut self) {
        if self.shell_slide_busy() {
            return;
        }
        let Some(spec) = Self::slide_spec_for(self.screen)
        else {
            return;
        };
        self.menu_frame_wave = Some(ShellFrameWave::new(spec, WaveDirection::SlideIn, Instant::now()));
        self.play_menu_move_in();
        self.refresh_menu_backdrop();
    }

    /// `SlideOut` 完成后：可选卡顿，再 `SlideIn`（`shell_slide_gap_secs=0` 则立刻进）。
    fn begin_slide_gap_or_in(&mut self) {
        let gap = self.shell_slide_gap_secs.max(0.0);
        if gap > 0.0 {
            self.menu_slide_gap_until = Some(Instant::now() + Duration::from_secs_f64(gap));
            self.refresh_menu_backdrop();
        }
        else {
            self.maybe_start_slide_in();
        }
    }

    /// 菜单导航入口：可切页动作先 SlideOut，完成后再提交，再对目标页 SlideIn。
    fn request_menu_action(&mut self, event_loop: &ActiveEventLoop, action: MenuAction) {
        if self.shell_slide_busy() {
            return;
        }
        if Self::action_uses_shell_slide(action) {
            if let Some(spec) = Self::slide_spec_for(self.screen) {
                self.menu_pressed_entry = None;
                self.menu_pending_commit = Some(action);
                self.menu_frame_wave = Some(ShellFrameWave::new(spec, WaveDirection::SlideOut, Instant::now()));
                self.play_menu_move_out();
                self.refresh_menu_backdrop();
                return;
            }
            let before = self.screen;
            self.commit_menu_action(event_loop, action);
            if self.screen != before {
                self.maybe_start_slide_in();
            }
            return;
        }
        self.commit_menu_action(event_loop, action);
    }

    /// 推进切页波浪 / 卡顿。`SlideOut` 结束后提交排队动作，经间隔再 `SlideIn`。
    fn tick_menu_frame_wave(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.menu_slide_gap_until {
            if Instant::now() >= deadline {
                self.menu_slide_gap_until = None;
                self.maybe_start_slide_in();
            }
            return;
        }
        let Some(wave) = self.menu_frame_wave.as_mut()
        else {
            return;
        };
        let now = Instant::now();
        if !wave.advance(now) {
            return;
        }
        if !wave.is_complete() {
            self.refresh_menu_backdrop();
            return;
        }
        let direction = wave.direction();
        self.menu_frame_wave = None;
        if direction == WaveDirection::SlideOut {
            if let Some(action) = self.menu_pending_commit.take() {
                self.commit_menu_action(event_loop, action);
                self.begin_slide_gap_or_in();
            }
            else {
                self.refresh_menu_backdrop();
            }
        }
        else {
            self.refresh_menu_backdrop();
        }
    }

    fn commit_menu_action(&mut self, event_loop: &ActiveEventLoop, action: MenuAction) {
        // 直接提交路径：取消尚未完成的出去波浪排队（卡顿由 `begin_slide_gap_or_in` 另管）。
        self.menu_pending_commit = None;
        let _ = self.menu_pressed_entry.take();
        match action {
            MenuAction::OpenSinglePlayer => self.set_screen(OriginalScreen::SinglePlayerMenu),
            MenuAction::OpenNetwork => {
                tracing::info!("网络入口未开放（Beta）");
                self.set_screen(OriginalScreen::Network);
            }
            MenuAction::OpenOptions => self.open_options_page(),
            MenuAction::Exit => {
                self.banner = "确认退出？".into();
                self.set_screen(OriginalScreen::ExitConfirm);
            }
            MenuAction::ConfirmExit => {
                tracing::info!("用户确认退出");
                event_loop.exit();
            }
            MenuAction::OpenSkirmish => {
                self.ensure_lobby_maps();
                self.set_screen(OriginalScreen::SkirmishLobby);
            }
            MenuAction::OpenCampaign => {
                self.campaign_side = None;
                self.campaign_difficulty = 1;
                self.campaign_dragging = false;
                self.campaign_pointer_consumed = false;
                self.set_screen(OriginalScreen::Campaign);
                self.banner = "战役选边".into();
                self.refresh_shell_title();
            }
            MenuAction::SelectCampaignAllied => {
                self.campaign_side = Some("allied");
                self.banner = "盟军战役（开局未接线）".into();
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::SelectCampaignTutorial => {
                self.campaign_side = Some("tutorial");
                self.banner = "新兵训练营（开局未接线）".into();
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::SelectCampaignSoviet => {
                self.campaign_side = Some("soviet");
                self.banner = "苏军战役（开局未接线）".into();
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::CycleCampaignDifficulty => {
                self.campaign_difficulty = (self.campaign_difficulty + 1) % 3;
                let label = match self.campaign_difficulty {
                    0 => "易",
                    2 => "难",
                    _ => "中",
                };
                self.banner = format!("战役难度 · {label}");
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::Back => match self.screen {
                OriginalScreen::SinglePlayerMenu | OriginalScreen::Network | OriginalScreen::Options | OriginalScreen::ExitConfirm => {
                    if self.screen == OriginalScreen::Options {
                        self.discard_options_draft();
                    }
                    self.set_screen(OriginalScreen::MainMenu);
                }
                OriginalScreen::SkirmishLobby | OriginalScreen::Campaign => {
                    self.set_screen(OriginalScreen::SinglePlayerMenu);
                }
                OriginalScreen::ChooseMap => self.cancel_choose_map(),
                _ => self.set_screen(OriginalScreen::MainMenu),
            },
            MenuAction::StartSkirmish => self.begin_skirmish_load(),
            MenuAction::CancelLoad => self.cancel_skirmish_load(),
            MenuAction::RetryLoad => {
                if self.load_job.is_some() {
                    tracing::info!("装载进行中，忽略重试点击");
                }
                else {
                    self.begin_skirmish_load();
                }
            }
            MenuAction::Noop => {}
            MenuAction::CycleSide => {
                self.skirmish.cycle_side();
                self.banner = format!("阵营 · {}", self.skirmish.side);
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::CycleDifficulty => {
                self.skirmish.cycle_difficulty();
                self.banner = format!("难度 · {}", self.skirmish.difficulty);
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::OptionsAccept => self.apply_options_accept(),
            MenuAction::OptionsCancel => {
                self.discard_options_draft();
                self.set_screen(OriginalScreen::MainMenu);
                self.banner = "选项已取消".into();
                self.refresh_shell_title();
            }
            MenuAction::SelectMap(i) => {
                if let Some(map) = self.lobby_maps.get(i) {
                    self.selected_map = Some(map.file_name.clone());
                    self.skirmish.preferred_map = Some(map.file_name.clone());
                    self.ensure_lobby_preview();
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
            }
            MenuAction::ChooseMap => self.open_choose_map_page(),
            MenuAction::UseMap => self.confirm_choose_map(),
        }
    }

    fn refresh_shell_title(&mut self) {
        let Some(window) = &self.window
        else {
            return;
        };
        if matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
            return;
        }
        let title = match self.screen {
            OriginalScreen::Splash => {
                format!("ra2 · 闪屏 · {} · Esc/Enter/点击跳过（预处理完成后进主菜单）· F12 截图", self.banner)
            }
            OriginalScreen::MainMenu => {
                format!("ra2 · 主菜单 · {} · Enter 单人 · N 网络 · O 选项 · Esc 确认退出 · F12 截图", self.banner)
            }
            OriginalScreen::SinglePlayerMenu => "ra2 · 单人游戏 · Enter/S 遭遇战 · Esc 返回 · F12 截图".into(),
            OriginalScreen::Campaign => {
                format!("ra2 · 战役 · {} · Esc 返回 · F12 截图", self.banner)
            }
            OriginalScreen::SkirmishLobby => {
                let detail = self
                    .selected_map
                    .as_ref()
                    .and_then(|name| self.lobby_maps.iter().find(|m| &m.file_name == name))
                    .map(|m| format!("{} {}x{} {}", m.file_name, m.width, m.height, m.theater.as_str()))
                    .unwrap_or_else(|| "（无可用图）".into());
                format!(
                    "ra2 · 遭遇战大厅 · {detail} · {}/{} · ←/→ 图 · Home/End · Q阵营 E难度 · Enter 开始 · Esc 返回 · F12 截图",
                    self.skirmish.side, self.skirmish.difficulty
                )
            }
            OriginalScreen::ChooseMap => {
                format!("ra2 · 选图 · {} · Esc 回大厅 · F12 截图", self.selected_map.as_deref().unwrap_or("（未选）"))
            }
            OriginalScreen::Network => "ra2 · 网络（占位禁用）· Esc 返回 · F12 截图".into(),
            OriginalScreen::LoadScreen => {
                if self.load_job.is_some() {
                    format!("ra2 · 加载 · {} · Esc/点取消 · F12 截图", self.banner)
                }
                else {
                    format!("ra2 · 加载 · {} · Enter/点重试 · Esc 回大厅 · F12 截图", self.banner)
                }
            }
            OriginalScreen::Options => {
                format!("ra2 · 选项 · {} · 视频循环分辨率 · Esc 返回 · F12 截图", self.banner)
            }
            OriginalScreen::ExitConfirm => {
                format!("ra2 · 确认退出 · {} · Enter 退出 · Esc 取消 · F12 截图", self.banner)
            }
            OriginalScreen::Battle | OriginalScreen::Results => unreachable!(),
        };
        if title != self.last_shell_title {
            window.set_title(&title);
            self.last_shell_title = title;
        }
    }

    fn begin_skirmish_load(&mut self) {
        if self.load_job.is_some() || self.pending_load_boot.is_some() {
            tracing::warn!("装载已在进行，忽略重复开始");
            return;
        }
        self.ensure_lobby_maps();
        self.banner =
            format!("正在装载 {} · {}/{}…", self.selected_map.as_deref().unwrap_or("默认候选图"), self.skirmish.side, self.skirmish.difficulty);
        self.pending_after_load = Some(OriginalScreen::Battle);
        self.pending_load_boot = None;
        self.set_screen(OriginalScreen::LoadScreen);
        self.load_started = Some(Instant::now());
        #[cfg(feature = "test-harness")]
        {
            if let Some(scene) = self.test_scene.clone() {
                self.load_job = Some(LoadJob::start_test_scene(scene));
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
                return;
            }
        }
        self.load_job = Some(LoadJob::start_install_boot({
            let mut req = self.skirmish.clone();
            req.preferred_map = self.selected_map.clone();
            req
        }));
        // load_job 赋值后刷新：禁用重试并改标题提示。
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    /// 放弃进行中的装载并回到遭遇战大厅（工作线程结果会被丢弃）。
    fn cancel_skirmish_load(&mut self) {
        if self.load_job.is_none() && self.pending_load_boot.is_none() && self.screen != OriginalScreen::LoadScreen {
            return;
        }
        self.load_job = None;
        self.pending_load_boot = None;
        self.load_started = None;
        self.pending_after_load = None;
        self.banner = "已取消装载".into();
        tracing::info!("用户取消遭遇战装载");
        self.set_screen(OriginalScreen::SkirmishLobby);
    }

    fn poll_load_job(&mut self) {
        if let Some(job) = self.load_job.as_ref() {
            match job.try_take() {
                Ok(Some(boot)) => {
                    self.load_job = None;
                    self.pending_load_boot = Some(boot);
                }
                Ok(None) => {}
                Err(()) => {
                    self.load_job = None;
                    self.pending_load_boot = None;
                    self.load_started = None;
                    self.pending_after_load = None;
                    self.banner = "装载线程异常断开 · Enter/点重试 · Esc 回大厅".into();
                    tracing::error!("遭遇战装载线程异常断开");
                    self.set_screen(OriginalScreen::LoadScreen);
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                    return;
                }
            }
        }

        if self.pending_load_boot.is_some() {
            let min = std::time::Duration::from_secs_f64(self.load_min_secs.max(0.0));
            let ready = self.load_started.map(|t0| t0.elapsed() >= min).unwrap_or(true);
            if ready {
                let boot = self.pending_load_boot.take().expect("pending_load_boot");
                self.load_started = None;
                self.finish_load(boot);
                return;
            }
        }

        if self.load_job.is_some() || self.pending_load_boot.is_some() {
            if let Some(t0) = self.load_started {
                let secs = t0.elapsed().as_secs();
                let pulse = match secs % 3 {
                    0 => ".",
                    1 => "..",
                    _ => "...",
                };
                let stage = if self.pending_load_boot.is_some() {
                    "装载完成，准备进入".into()
                }
                else {
                    self.load_job.as_ref().map(|job| job.progress().stage).unwrap_or_else(|| "装载中".into())
                };
                let pct = (self.load_screen_progress() * 100.0).round() as i32;
                let next = format!("{stage}{pulse} · {secs}s · {pct}% · Esc 取消");
                if next != self.banner {
                    self.banner = next;
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
            }
        }
    }

    fn finish_load(&mut self, boot: BootResult) {
        self.banner = boot.note.clone();
        if let Some(preview) = &boot.preview {
            self.renderer.set_map_preview(preview.clone());
        }
        match self.battle_controller.as_mut() {
            Some(ctrl) => ctrl.apply_boot(boot, &mut self.renderer),
            None => {
                self.battle_controller = Some(BattleController::from_boot(boot, self.status_path.clone(), self.test_scene.clone()));
            }
        }
        let ok = self.battle_controller.as_ref().is_some_and(|c| c.has_session());
        let target = self.pending_after_load.take().unwrap_or(OriginalScreen::Battle);
        if ok {
            self.set_screen(target);
        }
        else {
            self.banner = format!("装载失败 · {} · Enter/点重试 · Esc 回大厅", self.banner);
            tracing::warn!("遭遇战装载失败，停留加载页待重试");
            self.set_screen(OriginalScreen::LoadScreen);
            self.refresh_menu_backdrop();
            self.refresh_shell_title();
        }
    }

    fn apply_nav(&mut self, nav: BattleNav) {
        match nav {
            BattleNav::None => {}
            BattleNav::Rematch => {
                self.banner = "重开…".into();
                self.begin_skirmish_load();
            }
            BattleNav::ToResults => self.set_screen(OriginalScreen::Results),
            BattleNav::ToMainMenu => {
                // Pre-Alpha：从对局/结算回到遭遇战大厅，保留已选地图。
                self.ensure_lobby_maps();
                self.banner = format!("已返回大厅 · 地图 {}", self.selected_map.as_deref().unwrap_or("（未选）"));
                self.set_screen(OriginalScreen::SkirmishLobby);
            }
        }
    }

    fn handle_pre_game_key(&mut self, event_loop: &ActiveEventLoop, key: PhysicalKey) {
        if matches!(key, PhysicalKey::Code(KeyCode::F12)) {
            self.queue_screenshot(self.screen.as_str());
            return;
        }
        match self.screen {
            OriginalScreen::Splash => {
                // 只打跳过标；状态机在预处理完成后切主菜单。
                if matches!(
                    key,
                    PhysicalKey::Code(KeyCode::Escape) | PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter)
                ) {
                    self.request_splash_skip();
                }
            }
            OriginalScreen::MainMenu => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    self.set_screen(OriginalScreen::SinglePlayerMenu);
                }
                PhysicalKey::Code(KeyCode::KeyN) => {
                    tracing::info!("网络入口未开放（Beta）");
                    self.set_screen(OriginalScreen::Network);
                }
                PhysicalKey::Code(KeyCode::KeyO) => self.set_screen(OriginalScreen::Options),
                PhysicalKey::Code(KeyCode::Escape) => {
                    self.banner = "确认退出？".into();
                    self.set_screen(OriginalScreen::ExitConfirm);
                }
                _ => {}
            },
            OriginalScreen::SinglePlayerMenu => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) | PhysicalKey::Code(KeyCode::KeyS) => {
                    self.ensure_lobby_maps();
                    self.set_screen(OriginalScreen::SkirmishLobby);
                }
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::MainMenu),
                _ => {}
            },
            OriginalScreen::Campaign => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.set_screen(OriginalScreen::SinglePlayerMenu);
                }
            }
            OriginalScreen::SkirmishLobby => match key {
                PhysicalKey::Code(KeyCode::Escape) if self.skirmish.open_combo.is_some() => {
                    self.skirmish.close_combo();
                    self.refresh_menu_backdrop();
                }
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    self.begin_skirmish_load();
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) => self.cycle_lobby_map(-1),
                PhysicalKey::Code(KeyCode::ArrowRight) => self.cycle_lobby_map(1),
                PhysicalKey::Code(KeyCode::Home) => self.jump_lobby_map_edge(false),
                PhysicalKey::Code(KeyCode::End) => self.jump_lobby_map_edge(true),
                PhysicalKey::Code(KeyCode::KeyQ) => {
                    self.skirmish.cycle_side();
                    self.banner = format!("阵营 · {}", self.skirmish.side);
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
                PhysicalKey::Code(KeyCode::KeyE) => {
                    self.skirmish.cycle_difficulty();
                    self.banner = format!("难度 · {}", self.skirmish.difficulty);
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::SinglePlayerMenu),
                _ => {}
            },
            OriginalScreen::ChooseMap => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.cancel_choose_map();
                }
            }
            OriginalScreen::Network | OriginalScreen::Options => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    if self.screen == OriginalScreen::Options {
                        self.discard_options_draft();
                    }
                    self.set_screen(OriginalScreen::MainMenu);
                }
            }
            OriginalScreen::ExitConfirm => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    self.apply_menu_action(event_loop, MenuAction::ConfirmExit);
                }
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::MainMenu),
                _ => {}
            },
            OriginalScreen::LoadScreen => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    if self.load_job.is_some() {
                        tracing::info!("装载进行中，忽略 Enter 重试");
                    }
                    else {
                        self.begin_skirmish_load();
                    }
                }
                PhysicalKey::Code(KeyCode::Escape) => self.cancel_skirmish_load(),
                _ => {}
            },
            OriginalScreen::Battle | OriginalScreen::Results => {}
        }
    }

    fn redraw(&mut self) {
        if self.screen.pumps_session() {
            if let Some(ctrl) = self.battle_controller.as_mut() {
                let prev = ctrl.take_pump_clock();
                let dt = Instant::now().duration_since(prev).as_secs_f64();
                let (nav, sim_dt) = ctrl.pump(dt);
                self.renderer.timings.simulation = Some(sim_dt);
                let assets = self.menu_assets.as_ref().and_then(|a| a.source.as_ref());
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str(), self.menu_font.as_ref(), assets);
                self.apply_nav(nav);
            }
        }
        else if self.screen.requires_session() {
            if let Some(ctrl) = self.battle_controller.as_mut() {
                let _ = ctrl.take_pump_clock();
                self.renderer.timings.simulation = None;
                let assets = self.menu_assets.as_ref().and_then(|a| a.source.as_ref());
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str(), self.menu_font.as_ref(), assets);
            }
        }
        else {
            // 前置页：无色块菜单。原版 SHP 未接前仅标题 + 可选大厅地图预览。
            self.renderer.timings.simulation = None;
            self.renderer.timings.presentation_build = None;
            if self.screen == OriginalScreen::Splash {
                self.tick_splash();
            }
            if self.screen == OriginalScreen::LoadScreen {
                self.poll_load_job();
            }
            if matches!(
                self.screen,
                OriginalScreen::MainMenu
                    | OriginalScreen::SinglePlayerMenu
                    | OriginalScreen::Campaign
                    | OriginalScreen::Options
                    | OriginalScreen::ExitConfirm
                    | OriginalScreen::SkirmishLobby
                    | OriginalScreen::ChooseMap
            ) {
                let dt = self.menu_movie_clock.replace(Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0).min(0.25);
                let movie_advanced = self.menu_movie.as_mut().is_some_and(|m| m.tick(dt));
                let status_advanced = self.status_line.tick(dt);
                // WARNING 窗内动画：分类器间隔 100ms（10 FPS），勿与全局 15 FPS chrome 时钟混用。
                const WARN_FRAME_SECS: f64 = 0.1;
                let warn_pages = matches!(
                    self.screen,
                    OriginalScreen::MainMenu
                        | OriginalScreen::SinglePlayerMenu
                        | OriginalScreen::Campaign
                        | OriginalScreen::Options
                        | OriginalScreen::ExitConfirm
                );
                let mut panel_advanced = false;
                if warn_pages {
                    let panel_dt =
                        self.menu_panel_anim_clock.replace(Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0).min(0.25);
                    self.menu_panel_anim_accum += panel_dt;
                    while self.menu_panel_anim_accum >= WARN_FRAME_SECS {
                        self.menu_panel_anim_accum -= WARN_FRAME_SECS;
                        self.menu_panel_anim_frame = self.menu_panel_anim_frame.wrapping_add(1);
                        panel_advanced = true;
                    }
                }
                else {
                    self.menu_panel_anim_clock = None;
                }
                let mut side_advanced = false;
                if self.screen == OriginalScreen::Campaign {
                    let side_hot = matches!(self.menu_hovered_entry, Some("allied" | "tutorial" | "soviet")) || self.campaign_side.is_some();
                    if side_hot {
                        const SIDE_FRAME_SECS: f64 = 1.0 / 12.0;
                        let side_dt =
                            self.campaign_side_anim_clock.replace(Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0).min(0.25);
                        self.campaign_side_anim_accum += side_dt;
                        while self.campaign_side_anim_accum >= SIDE_FRAME_SECS {
                            self.campaign_side_anim_accum -= SIDE_FRAME_SECS;
                            self.campaign_side_anim_frame = self.campaign_side_anim_frame.wrapping_add(1).max(1);
                            side_advanced = true;
                        }
                    }
                    else {
                        self.campaign_side_anim_clock = None;
                    }
                }
                if movie_advanced || side_advanced || status_advanced || panel_advanced {
                    self.refresh_menu_backdrop();
                }
                else if let Some(reason) = self.menu_movie.as_ref().and_then(|m| m.stalled_reason()) {
                    if !self.banner.contains("影片失步") {
                        self.banner = format!("{} · 影片失步 · {reason}", self.banner);
                    }
                }
            }
            if self.screen == OriginalScreen::SkirmishLobby {
                let ready = self.poll_lobby_preview();
                if ready {
                    self.refresh_menu_backdrop();
                    let map = self.selected_map.as_deref().unwrap_or("?");
                    self.banner = format!("预览就绪 · {map}");
                }
                else if self.lobby_preview_job.is_some() {
                    let map = self.selected_map.as_deref().unwrap_or("?");
                    self.banner = format!("预览生成中… {map}");
                }
            }
            self.renderer.draw_frame(None);
            self.refresh_shell_title();
        }
        self.flush_pending_screenshot();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("ra2")
                        .with_inner_size(winit::dpi::LogicalSize::new(self.window_width, self.window_height))
                        // 客户区尺寸由 `DisplayMode` 离散档决定，禁止自由拉伸窗口。
                        .with_resizable(false)
                        // GPU / 闪屏首帧完成前保持隐藏，避免 Windows 默认纯白客户区闪一下。
                        .with_visible(false),
                )
                .expect("创建窗口失败"),
        );
        if let Err(e) = self.renderer.attach_window(window.clone()) {
            tracing::error!("wgpu 附着失败: {e}");
            self.banner = format!("GPU 附着失败 · {e}");
            // 无 GPU 表面则禁止空转主循环；窗口也不再保留。
            event_loop.exit();
            return;
        }
        tracing::info!(
            "gpu={} preview={} zoom={:.2} screen={}",
            self.renderer.backend_name(),
            if self.renderer.has_preview() { "yes" } else { "no" },
            self.renderer.camera().zoom,
            self.screen.as_str()
        );
        self.window = Some(window.clone());
        if self.screen == OriginalScreen::Splash {
            self.ensure_startup_splash_presented();
        }
        else {
            self.refresh_menu_backdrop();
        }
        self.refresh_shell_title();
        // 先提交一帧（闪屏或菜单），再显示窗口，消除启动纯白闪屏。
        self.renderer.draw_frame(None);
        window.set_visible(true);
        window.request_redraw();
        #[cfg(feature = "test-harness")]
        if self.auto_screenshots.should_capture(self.screen) {
            self.queue_screenshot(self.screen.as_str());
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match &event {
            WindowEvent::CloseRequested => {
                // 前置壳层页先进入退出确认；已在确认页或对局中则直接退出。
                if self.screen == OriginalScreen::ExitConfirm
                    || matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results | OriginalScreen::Splash)
                {
                    event_loop.exit();
                }
                else if matches!(
                    self.screen,
                    OriginalScreen::MainMenu
                        | OriginalScreen::SinglePlayerMenu
                        | OriginalScreen::Options
                        | OriginalScreen::SkirmishLobby
                        | OriginalScreen::Network
                        | OriginalScreen::LoadScreen
                ) {
                    if self.screen == OriginalScreen::Options {
                        self.discard_options_draft();
                    }
                    self.banner = "确认退出？".into();
                    self.set_screen(OriginalScreen::ExitConfirm);
                }
                else {
                    event_loop.exit();
                }
                return;
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
                // GPU 表面跟物理缓冲；命中/布局锁定在选定 `DisplayMode` 客户区。
                let (w, h) = self.display_mode.size();
                self.window_width = w as f64;
                self.window_height = h as f64;
                if !matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
                    self.refresh_menu_backdrop();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let (w, h) = self.display_mode.size();
                self.window_width = w as f64;
                self.window_height = h as f64;
                if !matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
                    self.refresh_menu_backdrop();
                }
            }
            WindowEvent::RedrawRequested => {
                self.tick_menu_frame_wave(event_loop);
                self.redraw();
                return;
            }
            WindowEvent::KeyboardInput { event: key_ev, .. }
                if key_ev.state == ElementState::Pressed && matches!(key_ev.physical_key, PhysicalKey::Code(KeyCode::F12)) =>
            {
                // 对局页也走同一截图路径（不交给 BattleController）。
                self.queue_screenshot(self.screen.as_str());
                return;
            }
            _ => {}
        }

        match self.screen {
            OriginalScreen::Battle | OriginalScreen::Results => {
                let accept = self.screen.accepts_battle_commands();
                let Some(window) = self.window.clone()
                else {
                    return;
                };
                if let Some(ctrl) = self.battle_controller.as_mut() {
                    let nav = ctrl.handle_event(&event, &mut self.renderer, &window, accept);
                    self.apply_nav(nav);
                }
            }
            OriginalScreen::Splash
            | OriginalScreen::MainMenu
            | OriginalScreen::SinglePlayerMenu
            | OriginalScreen::Campaign
            | OriginalScreen::SkirmishLobby
            | OriginalScreen::ChooseMap
            | OriginalScreen::Network
            | OriginalScreen::Options
            | OriginalScreen::ExitConfirm
            | OriginalScreen::LoadScreen => match &event {
                WindowEvent::CursorMoved { position, .. } => {
                    // 与 window_width/height 同用逻辑像素，避免 HiDPI 下物理光标打偏命中框。
                    let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                    let logical = position.to_logical::<f64>(scale);
                    self.cursor = (logical.x, logical.y);
                    if self.screen == OriginalScreen::Options && self.handle_options_drag() {
                        // 拖动滑条已刷新。
                    }
                    else if self.screen == OriginalScreen::SkirmishLobby && self.handle_skirmish_drag() {
                        // 遭遇战滑条拖动已刷新。
                    }
                    else if self.screen == OriginalScreen::Campaign && self.handle_campaign_drag() {
                        // 战役难度滑条拖动已刷新。
                    }
                    else if matches!(
                        self.screen,
                        OriginalScreen::MainMenu
                            | OriginalScreen::SinglePlayerMenu
                            | OriginalScreen::Campaign
                            | OriginalScreen::Options
                            | OriginalScreen::ExitConfirm
                            | OriginalScreen::SkirmishLobby
                            | OriginalScreen::ChooseMap
                    ) {
                        let next = self.menu_entry_under_cursor();
                        if next != self.menu_hovered_entry {
                            if self.screen == OriginalScreen::Campaign {
                                if matches!(next, Some("allied" | "tutorial" | "soviet")) {
                                    if let Some(side) = next {
                                        self.play_campaign_side_hover(side);
                                    }
                                    self.campaign_side_anim_frame = 1;
                                    self.campaign_side_anim_accum = 0.0;
                                }
                            }
                            self.menu_hovered_entry = next;
                            self.sync_status_line_from_hover();
                            self.refresh_menu_backdrop();
                        }
                    }
                }
                WindowEvent::MouseInput { state, button: winit::event::MouseButton::Left, .. } => match state {
                    ElementState::Pressed => {
                        if self.shell_slide_busy() {
                            // 切页波浪 / 卡顿进行中忽略新按下。
                        }
                        else if self.screen == OriginalScreen::Splash {
                            self.request_splash_skip();
                        }
                        else if self.screen == OriginalScreen::Options && self.handle_options_press() {
                            // 选项左/右栏已处理。
                        }
                        else if self.screen == OriginalScreen::SkirmishLobby && self.handle_skirmish_press() {
                            // 遭遇战左栏勾选/滑条已处理。
                        }
                        else if self.screen == OriginalScreen::Campaign && self.handle_campaign_press() {
                            // 战役难度滑条已处理。
                        }
                        else if matches!(
                            self.screen,
                            OriginalScreen::MainMenu
                                | OriginalScreen::SinglePlayerMenu
                                | OriginalScreen::Campaign
                                | OriginalScreen::Options
                                | OriginalScreen::ExitConfirm
                                | OriginalScreen::SkirmishLobby
                                | OriginalScreen::ChooseMap
                        ) {
                            let next = self.menu_entry_under_cursor();
                            if self.screen == OriginalScreen::SkirmishLobby && next.is_some() {
                                self.skirmish.end_name_edit();
                                self.skirmish.close_combo();
                            }
                            if next != self.menu_pressed_entry {
                                self.menu_pressed_entry = next;
                                if next.is_some() {
                                    self.play_menu_click();
                                }
                                self.refresh_menu_backdrop();
                            }
                        }
                    }
                    ElementState::Released => {
                        if self.shell_slide_busy() {
                            // 切页波浪 / 卡顿进行中忽略释放提交。
                        }
                        else if self.screen == OriginalScreen::Splash {
                            // 闪屏仅接受按下跳过请求；释放不走菜单命中。
                        }
                        else if self.screen == OriginalScreen::Options {
                            let consumed = self.options_pointer_consumed;
                            self.options_pointer_consumed = false;
                            if let Some(state) = self.options_state.as_mut() {
                                state.on_release();
                            }
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            if consumed {
                                self.refresh_menu_backdrop();
                            }
                            else if let Some(action) = ui_hit::hit_action(
                                self.screen,
                                &self.lobby_maps,
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.load_allow_retry(),
                            ) {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                        else if self.screen == OriginalScreen::SkirmishLobby {
                            let consumed = self.skirmish_pointer_consumed;
                            self.skirmish_pointer_consumed = false;
                            self.skirmish.on_release();
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            if consumed {
                                self.refresh_menu_backdrop();
                            }
                            else if let Some(action) = ui_hit::hit_action(
                                self.screen,
                                &self.lobby_maps,
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.load_allow_retry(),
                            ) {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                        else if self.screen == OriginalScreen::Campaign {
                            let consumed = self.campaign_pointer_consumed;
                            self.campaign_pointer_consumed = false;
                            self.campaign_dragging = false;
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            if consumed {
                                // 难度已在按下/拖动时落档，勿再 CycleCampaignDifficulty。
                            }
                            else if let Some(action) = ui_hit::hit_action(
                                self.screen,
                                &self.lobby_maps,
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.load_allow_retry(),
                            ) {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                        else {
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            let action = ui_hit::hit_action(
                                self.screen,
                                &self.lobby_maps,
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.load_allow_retry(),
                            );
                            if let Some(action) = action {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                    }
                },
                WindowEvent::KeyboardInput { event: key_ev, .. } => {
                    if key_ev.state == ElementState::Pressed {
                        if self.handle_skirmish_name_key(key_ev) {
                            // 编辑玩家名时吞掉大厅快捷键。
                        }
                        else {
                            self.handle_pre_game_key(event_loop, key_ev.physical_key);
                        }
                    }
                }
                _ => {}
            },
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

mod construct;
mod host;
mod audio;
mod splash;
mod options;
mod lobby;
mod campaign;
mod assets;
mod diagnostics;
mod navigation;
mod loading;
mod input;
mod redraw;
mod event_loop;
mod launch;

pub use host::Host;
pub use launch::{campaign_difficulty_from_track_x, run_shell};
