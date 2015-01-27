//! 单窗口应用外壳：页面导航、窗口生命周期；对局逻辑委托 `MatchController`。

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
    match_ctrl::{MatchController, MatchNav},
    preview_job::PreviewJob,
};
use ra_components::{
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

/// 外壳持有的可导航应用状态。
pub struct AppShell {
    window: Option<Arc<Window>>,
    screen: OriginalScreen,
    /// 对局 / 结算页控制器；菜单页可为空。
    match_ctrl: Option<MatchController>,
    renderer: Renderer,
    /// 菜单或装载说明。
    banner: String,
    window_width: f64,
    window_height: f64,
    /// 客户区分辨率档（布局与缓冲基准，非自由拉伸）。
    display_mode: DisplayMode,
    /// 壳层质感呈现（来自 `RustAlert.toml` `[present]`）。
    present: PresentFeel,
    status_path: Option<PathBuf>,
    test_scene: Option<String>,
    /// 进程启动闪屏 presentation（独立 owner；非菜单槽）。
    startup_splash: Option<StartupSplashPresentation>,
    /// 闪屏最短展示秒数（首次成功 present 后起算；可调，默认 3）。
    splash_min_secs: f64,
    /// 遭遇战装载页最短展示秒数（`RustAlert.toml` 的 `load_min_secs`，默认 3；`0` 关闭）。
    load_min_secs: f64,
    /// 壳层切页出去→进来之间的停顿秒数（模拟原版重型机械卡顿；`0` 关闭）。
    shell_slide_gap_secs: f64,
    /// 闪屏预处理是否完成。
    splash_preload_done: bool,
    /// 用户请求跳过闪屏（仍须预处理完成才进主菜单）。
    splash_skip: bool,
    /// 装载完成后待切到的目标页。
    pending_after_load: Option<OriginalScreen>,
    /// 光标位置（逻辑像素，与 `window_width` / `window_height` 同单位）。
    cursor: (f64, f64),
    /// 后台遭遇战装载（`LoadScreen` 期间轮询）。
    load_job: Option<LoadJob>,
    /// 当前装载开始时刻。
    load_started: Option<Instant>,
    /// 后台已完成、等待最短展示时间后再 `finish_load` 的结果。
    pending_load_boot: Option<BootResult>,
    /// 遭遇战大厅可选地图。
    lobby_maps: Vec<super::boot::BootMapCandidate>,
    /// 当前选中的地图文件名。
    selected_map: Option<String>,
    /// 大厅缩略图对应的地图名（与 `lobby_preview` 配对）。
    lobby_preview_for: Option<String>,
    /// 已缩小的选中地图预览。
    lobby_preview: Option<RgbaImage>,
    /// 后台地图预览任务。
    lobby_preview_job: Option<PreviewJob>,
    /// 遭遇战控件 PCX 缓存（勾选/滑条拇指/旗标）。
    skirmish_chrome: Option<SkirmishChromeSprites>,
    /// 旗标缓存对应的阵营名（换边时重载）。
    skirmish_chrome_side: Option<String>,
    /// 遭遇战左栏按下是否已消费（勾选/滑条，勿再走右栏按钮命中）。
    skirmish_pointer_consumed: bool,
    /// 主菜单阶段已挂载资源（惰性一次）。
    menu_assets: Option<MenuUiAssets>,
    /// 当前页 chrome 解码缓存（切换页或重探时刷新）。
    ui_decode_cache: Option<ui_decode::PageDecodeReport>,
    /// 主菜单当前按住的按钮入口 id（按下帧合成）。
    menu_pressed_entry: Option<&'static str>,
    /// 切页排队：`SlideOut` 完成后提交的 `MenuAction`。
    menu_pending_commit: Option<MenuAction>,
    /// 进行中的右栏 `SDBTNANM` 帧波浪（出去 / 进来）。
    menu_frame_wave: Option<ShellFrameWave>,
    /// 出去结束后、进来开始前的卡顿截止时刻（无字、钮面收起）。
    menu_slide_gap_until: Option<Instant>,
    /// 主菜单当前悬停的按钮入口 id（悬停帧合成）。
    menu_hovered_entry: Option<&'static str>,
    /// 底栏状态提示打字机（与按钮 hover 图解耦；亦可复用于局内右上消息）。
    status_line: TypewriterText,
    /// 菜单字体（`game.fnt`）。
    menu_font: Option<FntFile>,
    /// 是否已尝试装载菜单字体（失败后不再每帧读盘/打日志）。
    menu_font_tried: bool,
    /// 菜单文案表（`ra2.csf` / `ra2md.csf`）。
    menu_csf: Option<CsfFile>,
    /// 是否已尝试装载菜单文案表。
    menu_csf_tried: bool,
    /// 主菜单 / 单人页循环影片。
    menu_movie: Option<MenuMoviePlayer>,
    /// 影片时钟（`tick` 用）。
    menu_movie_clock: Option<Instant>,
    /// WARNING 窗内 `sdwrnanm` 动画时钟（与侧图箭头分离）。
    menu_panel_anim_clock: Option<Instant>,
    /// WARNING 动画未消耗的累计秒。
    menu_panel_anim_accum: f64,
    /// `sdwrnanm` 帧序号（对解码帧数取模）。
    menu_panel_anim_frame: usize,
    /// 战役侧图箭头动画时钟。
    campaign_side_anim_clock: Option<Instant>,
    /// 侧图动画累计秒。
    campaign_side_anim_accum: f64,
    /// 侧图箭头帧序号。
    campaign_side_anim_frame: usize,
    /// 战役选边悬停语音（`AlliedCampaignSelect` 等，惰性）。
    campaign_side_sfx: [Option<PcmAudio>; 3],
    /// 下一帧回读后落盘的截图短名（`OriginalScreen::as_str`）；F12 手动截图用。
    pending_screenshot: Option<&'static str>,
    /// 自动关键页截图去重（仅 `test-harness`）。
    #[cfg(feature = "test-harness")]
    auto_screenshots: super::screenshot::AutoScreenshotTracker,
    /// 遭遇战大厅阵营 / 难度（进入装载请求）。
    skirmish: SkirmishBootRequest,
    /// 进入选图页前的 `preferred_map` 快照（取消时还原）。
    choose_map_revert: Option<Option<String>>,
    /// 战役选边：`allied` / `tutorial` / `soviet`。
    campaign_side: Option<&'static str>,
    /// 战役难度档：0 易 / 1 中 / 2 难。
    campaign_difficulty: u8,
    /// 战役难度滑条是否正在拖动。
    campaign_dragging: bool,
    /// 战役左栏按下是否已消费（难度滑条，勿再走点击轮换）。
    campaign_pointer_consumed: bool,
    /// 桌面音频输出（设备不可用则为 `None`）。
    audio: Option<super::audio::ShellAudio>,
    /// 主菜单 BGM PCM（`theme.ini` `[INTRO]` → `{Sound}.wav`）。
    menu_bgm: Option<PcmAudio>,
    /// 是否已尝试装载菜单 BGM（失败后不再每帧重试）。
    menu_bgm_tried: bool,
    /// 菜单点击音效 PCM（`GUIMainButtonSound` → `sound.ini` → `audio.bag`）。
    menu_click: Option<PcmAudio>,
    /// 壳层出去音效（`GUIMoveOutSound` → 默认 `MenuSlideOut` / `uslide2`）。
    menu_move_out: Option<PcmAudio>,
    /// 是否已尝试装载出去音效。
    menu_move_out_tried: bool,
    /// 壳层进来音效（`GUIMoveInSound` → 默认 `MenuSlideIn` / `uslide1`）。
    menu_move_in: Option<PcmAudio>,
    /// 是否已尝试装载进来音效。
    menu_move_in_tried: bool,
    /// 当前是否已在播壳层 BGM。
    menu_bgm_playing: bool,
    /// 已解析的 `audio.bag` 索引（惰性）。
    audio_bag: Option<AudioIndex>,
    /// 是否已尝试装载 `audio.bag`（避免反复读盘）。
    audio_bag_tried: bool,
    /// 选项页草稿（进入 Options 时创建，接受/取消后清空）。
    options_state: Option<ra_components::options_dialog::OptionsDialogState>,
    /// 进入选项页时的音量快照（取消时还原实时预览）。
    options_volume_baseline: Option<(f32, f32)>,
    /// 进入选项页时的质感快照（取消时还原实时预览）。
    options_present_baseline: Option<PresentFeel>,
    /// 本轮按下已由左栏控件消费（释放时勿再走右栏命中）。
    options_pointer_consumed: bool,
    /// 上次已写入的窗口标题（避免每帧 `set_title` 卡顿）。
    last_shell_title: String,
}

impl AppShell {
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
        let ctrl = MatchController::from_boot(boot, status_path.clone(), test_scene.clone());
        let screen = if ctrl.has_session() { OriginalScreen::Match } else { OriginalScreen::MainMenu };
        Self {
            window: None,
            screen,
            match_ctrl: Some(ctrl),
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
            match_ctrl: None,
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

    /// 按配置应用壳层 BGM / 短音效音量（设备缺失时无操作）。
    pub fn apply_audio_volumes(&mut self, music_volume: f32, sound_volume: f32) {
        if let Some(audio) = self.audio.as_mut() {
            audio.set_music_volume(music_volume);
            audio.set_sfx_volume(sound_volume);
            tracing::info!(music_volume = audio.music_volume(), sound_volume = audio.sfx_volume(), "已应用壳层音量");
        }
    }

    /// 应用壳层质感呈现配置（上传 UI 页前生效）。
    pub fn apply_present_feel(&mut self, present: PresentFeel) {
        self.present = present.sanitized();
        tracing::info!(
            mode = self.present.mode.as_str(),
            quantize = self.present.quantize.as_str(),
            dither = self.present.dither,
            "已应用壳层质感呈现"
        );
    }

    /// 上传 UI 页：先按 `[present]` 做质感变换再进 GPU。
    fn upload_ui_page(&mut self, page: RgbaImage) {
        let page = ui_present::present_ui_page(page, self.present);
        self.renderer.set_ui_page(page);
    }

    fn shell_cursor_px(&self) -> (i32, i32) {
        ui_layout::window_to_shell_px(self.cursor.0, self.cursor.1, self.window_width, self.window_height)
    }

    /// 用选项草稿中的音乐/音效滑条即时推到设备（不落盘）。
    fn sync_options_live_volumes(&mut self) {
        let Some(state) = self.options_state.as_ref()
        else {
            return;
        };
        let music = state.music_volume_f32();
        let sound = state.sound_volume_f32();
        if let Some(audio) = self.audio.as_mut() {
            audio.set_music_volume(music);
            audio.set_sfx_volume(sound);
        }
    }

    /// 用选项草稿中的质感即时推到壳层（不落盘，避免拖动时刷日志）。
    fn sync_options_live_present(&mut self) {
        let Some(state) = self.options_state.as_ref()
        else {
            return;
        };
        let next = state.present.sanitized();
        if next != self.present {
            self.present = next;
        }
    }

    /// 选项页按下：左栏优先；右栏仍走原有 pressed 精灵。
    fn handle_options_press(&mut self) -> bool {
        let layout = ra_components::options_dialog::OptionsDialogLayout::new();
        let (x, y) = self.shell_cursor_px();
        let Some(hit) = self.options_state.as_mut().and_then(|state| state.on_press(&layout, x, y))
        else {
            return false;
        };
        use ra_components::options_dialog::OptionsHit;
        match hit {
            OptionsHit::Accept => {
                self.menu_pressed_entry = Some("accept");
                self.play_menu_click();
                self.refresh_menu_backdrop();
                true
            }
            OptionsHit::Cancel => {
                self.menu_pressed_entry = Some("cancel");
                self.play_menu_click();
                self.refresh_menu_backdrop();
                true
            }
            OptionsHit::MainMenu => {
                self.menu_pressed_entry = Some("main_menu");
                self.play_menu_click();
                self.refresh_menu_backdrop();
                true
            }
            OptionsHit::Track(_) | OptionsHit::Toggle(_) | OptionsHit::ResolutionCombo | OptionsHit::ResolutionRow(_) => {
                self.options_pointer_consumed = true;
                self.play_menu_click();
                self.sync_options_live_volumes();
                self.sync_options_live_present();
                self.refresh_menu_backdrop();
                true
            }
        }
    }

    /// 选项页拖动滑条。
    fn handle_options_drag(&mut self) -> bool {
        let layout = ra_components::options_dialog::OptionsDialogLayout::new();
        let (x, y) = self.shell_cursor_px();
        let dragged = self.options_state.as_mut().map(|state| state.dragging.is_some() && state.on_drag(&layout, x, y)).unwrap_or(false);
        if !dragged {
            return false;
        }
        self.sync_options_live_volumes();
        self.sync_options_live_present();
        self.refresh_menu_backdrop();
        true
    }

    /// 从挂载源解码 PCX → RGBA；品红 `(255,0,255)` 作色键透明（旗标索引未必为 0）。
    fn load_pcx_rgba(source: &ra_components::fs_source::GameAssetSource, name: &str) -> Option<RgbaImage> {
        let bytes = source.read(name).ok()?;
        let pcx = ra_assets::parse_pcx(&bytes).ok()?;
        let mut rgba = pcx.rgba;
        for px in rgba.chunks_exact_mut(4) {
            if px[0] == 255 && px[1] == 0 && px[2] == 255 {
                px[3] = 0;
            }
        }
        RgbaImage::from_raw(pcx.width, pcx.height, rgba)
    }

    /// 惰性加载遭遇战勾选 / 滑条拇指 / 旗标 PCX。
    fn ensure_skirmish_chrome(&mut self) {
        self.ensure_menu_assets();
        let flag_key = self
            .skirmish
            .row_sides
            .iter()
            .map(|i| ra_components::skirmish_setup::LOBBY_SIDES[(*i as usize) % ra_components::skirmish_setup::LOBBY_SIDES.len()])
            .collect::<Vec<_>>()
            .join(",");
        let need_flag = self.skirmish_chrome_side.as_deref() != Some(flag_key.as_str());
        let need_base = self
            .skirmish_chrome
            .as_ref()
            .map(|c| c.checkbox_off.is_none() || c.track_cap_l.is_none())
            .unwrap_or(true);
        if !need_base && !need_flag {
            return;
        }
        let Some(source) = self.menu_assets.as_ref().and_then(|a| a.source.as_ref())
        else {
            return;
        };
        let mut chrome = self.skirmish_chrome.take().unwrap_or_default();
        if need_base {
            chrome.checkbox_off = Self::load_pcx_rgba(source, "cue_i.pcx");
            chrome.checkbox_on = Self::load_pcx_rgba(source, "cce_i.pcx");
            chrome.track_thumb = Self::load_pcx_rgba(source, "trakgrip.pcx");
            chrome.track_cap_l = Self::load_pcx_rgba(source, "trofl.pcx");
            chrome.track_cap_m = Self::load_pcx_rgba(source, "trofm.pcx");
            chrome.track_cap_r = Self::load_pcx_rgba(source, "trofr.pcx");
        }
        if need_flag {
            for i in 0..ui_layout::SKIRMISH_ROW_COUNT {
                let side = self.skirmish.row_side(i);
                chrome.row_flags[i] = Self::load_pcx_rgba(source, side_flag_pcx(side));
            }
            chrome.flag = chrome.row_flags[0].clone();
            chrome.ai_flag = chrome.row_flags[1].clone();
            self.skirmish_chrome_side = Some(flag_key);
        }
        self.skirmish_chrome = Some(chrome);
    }

    /// 遭遇战左栏按下：勾选 / 滑条优先于右栏按钮。
    fn handle_skirmish_press(&mut self) -> bool {
        let layout = ui_layout::skirmish_lobby_layout(0, 0);
        let (x, y) = self.shell_cursor_px();
        let ai_rows = self.lobby_ai_rows();
        if self.skirmish.on_press(&layout, x, y, ai_rows).is_none() {
            return false;
        }
        self.skirmish_pointer_consumed = true;
        self.play_menu_click();
        self.refresh_menu_backdrop();
        true
    }

    /// 当前选中地图对应的 AI 行数。
    fn lobby_ai_rows(&self) -> usize {
        let slots = self
            .selected_map
            .as_ref()
            .and_then(|sel| self.lobby_maps.iter().find(|m| &m.file_name == sel))
            .or_else(|| self.lobby_maps.first())
            .map(|m| m.start_slots)
            .unwrap_or(4);
        super::boot::skirmish_ai_row_count(slots)
    }

    /// 遭遇战滑条拖动。
    fn handle_skirmish_drag(&mut self) -> bool {
        if self.skirmish.dragging.is_none() {
            return false;
        }
        let layout = ui_layout::skirmish_lobby_layout(0, 0);
        let (x, y) = self.shell_cursor_px();
        if !self.skirmish.on_drag(&layout, x, y) {
            return false;
        }
        self.refresh_menu_backdrop();
        true
    }

    /// 玩家名编辑中的键盘输入。返回 `true` 表示已消费（勿再走大厅快捷键）。
    fn handle_skirmish_name_key(&mut self, key_ev: &winit::event::KeyEvent) -> bool {
        if self.screen != OriginalScreen::SkirmishLobby || !self.skirmish.player_name_editing {
            return false;
        }
        match key_ev.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => {
                if self.skirmish.backspace_name() {
                    self.refresh_menu_backdrop();
                }
            }
            PhysicalKey::Code(KeyCode::Escape) | PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                self.skirmish.end_name_edit();
                self.refresh_menu_backdrop();
            }
            _ => {
                if let Some(text) = key_ev.text.as_deref() {
                    if self.skirmish.append_name_text(text) {
                        self.refresh_menu_backdrop();
                    }
                }
            }
        }
        true
    }

    /// 战役难度滑条按下：按轨坐标落档并开始拖动。
    fn handle_campaign_press(&mut self) -> bool {
        let layout = ui_layout::campaign_layout(0, 0);
        let (x, y) = self.shell_cursor_px();
        if !(layout.difficulty_track.contains(x, y) || layout.difficulty_label.contains(x, y) || layout.difficulty_value.contains(x, y)) {
            return false;
        }
        self.campaign_dragging = true;
        self.campaign_pointer_consumed = true;
        self.set_campaign_difficulty_from_x(layout.difficulty_track, x);
        self.play_menu_click();
        true
    }

    /// 战役难度滑条拖动。
    fn handle_campaign_drag(&mut self) -> bool {
        if !self.campaign_dragging {
            return false;
        }
        let layout = ui_layout::campaign_layout(0, 0);
        let (x, _) = self.shell_cursor_px();
        self.set_campaign_difficulty_from_x(layout.difficulty_track, x);
        true
    }

    /// 按轨道 X 映射难度 0..=2，档位变化时刷新。
    fn set_campaign_difficulty_from_x(&mut self, track: ui_layout::RectPx, x: i32) {
        let next = campaign_difficulty_from_track_x(track, x);
        if next == self.campaign_difficulty {
            return;
        }
        self.campaign_difficulty = next;
        let label = match next {
            0 => "易",
            2 => "难",
            _ => "中",
        };
        self.banner = format!("战役难度 · {label}");
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    /// 用户请求跳过闪屏；预处理完成后才进主菜单。
    fn request_splash_skip(&mut self) {
        if self.screen != OriginalScreen::Splash {
            return;
        }
        self.splash_skip = true;
        tracing::info!("闪屏跳过已请求");
    }

    /// 闪屏每帧：保证启动画面在屏、推进预处理；期限结束或跳过后切主菜单。
    fn tick_splash(&mut self) {
        if self.screen != OriginalScreen::Splash {
            return;
        }
        // 先保证启动画面在屏，再做菜单资源预热（预热不得切换页面、不得清空 UI 页）。
        self.ensure_startup_splash_presented();
        if !self.splash_preload_done {
            self.ensure_menu_assets();
            self.ensure_menu_text_assets();
            self.ensure_menu_audio_assets();
            // 预热主菜单 chrome：不切入 MainMenu，避免合成/清屏打穿闪屏。
            self.warm_main_menu_chrome();
            self.splash_preload_done = true;
            if !self.banner.contains("预处理完成") {
                self.banner = format!("{} · 预处理完成", self.banner);
            }
            self.refresh_shell_title();
        }
        let now = Instant::now();
        let hold_active = self.startup_splash.as_ref().is_some_and(|splash| splash.is_active(now));
        let min_ok = !hold_active;
        if self.splash_preload_done && (min_ok || self.splash_skip) {
            tracing::info!(hold_active, min = self.splash_min_secs, skip = self.splash_skip, "启动闪屏结束 → 主菜单");
            self.startup_splash = None;
            self.set_screen(OriginalScreen::MainMenu);
            self.maybe_start_slide_in();
        }
    }

    /// 在闪屏状态下预热主菜单资源缓存（不改 `screen`、不上传菜单合成页）。
    fn warm_main_menu_chrome(&mut self) {
        let Some(assets) = self.menu_assets.as_ref()
        else {
            return;
        };
        let Some(source) = assets.source.as_ref()
        else {
            return;
        };
        let edition = assets.edition;
        let Some(page) = page_resources_from_slots_with_edition(OriginalScreen::MainMenu, edition)
        else {
            return;
        };
        let report = ui_resolve::resolve_page(source, &page);
        let only_movie_gaps = report.missing.iter().all(|m| m.to_ascii_lowercase().ends_with(".bik"));
        if report.named > 0 && only_movie_gaps {
            let decoded = ui_decode::decode_page_chrome(source, &page);
            tracing::info!(
                errors = decoded.errors.len(),
                chrome_ready = decoded.chrome_ready_for_enabled_buttons(&page),
                "闪屏预热主菜单 chrome · {}",
                decoded.banner_note()
            );
            self.ui_decode_cache = Some(decoded);
        }
    }

    /// 构造或复用启动闪屏 presentation，并上传到 UI 页；首次成功上传时武装最短展示期限。
    fn ensure_startup_splash_presented(&mut self) {
        if self.startup_splash.is_none() {
            self.ensure_menu_assets();
            self.ensure_menu_text_assets();
            let client_w = self.window_width.round().max(1.0) as u32;
            let client_h = self.window_height.round().max(1.0) as u32;
            let minimum = std::time::Duration::from_secs_f64(self.splash_min_secs.max(0.0));
            let prefer_md = self.menu_assets.as_ref().and_then(|a| a.edition).is_some_and(startup_splash::prefer_md_splash);
            let built = match self.menu_assets.as_ref().and_then(|a| a.source.as_ref()) {
                None => {
                    tracing::warn!("启动闪屏 · 安装资源源未挂载，使用黑底占位");
                    StartupSplashPresentation::placeholder(client_w, client_h, prefer_md, minimum).ok()
                }
                Some(source) => {
                    match StartupSplashPresentation::build(
                        source,
                        self.menu_csf.as_ref(),
                        self.menu_font.as_ref(),
                        client_w,
                        client_h,
                        prefer_md,
                        minimum,
                    ) {
                        Ok(splash) => Some(splash),
                        Err(e) => {
                            tracing::warn!("启动闪屏构造失败 · {e} · 回退黑底占位");
                            StartupSplashPresentation::placeholder(client_w, client_h, prefer_md, minimum).ok()
                        }
                    }
                }
            };
            if let Some(splash) = built {
                let shp = splash.shp_name();
                let w = splash.image().width();
                let h = splash.image().height();
                tracing::info!(shp, pal = splash.pal_name(), prefer_md, w, h, "启动闪屏已合成");
                if !self.banner.contains(shp) {
                    self.banner = format!("{} · {shp} {w}×{h}", self.banner);
                    self.refresh_shell_title();
                }
                self.startup_splash = Some(splash);
            }
            else if !self.banner.contains("闪屏缺图") {
                self.banner = format!("{} · 闪屏缺图", self.banner);
                self.refresh_shell_title();
            }
        }

        let Some(splash) = self.startup_splash.as_ref()
        else {
            return;
        };
        if !self.renderer.has_ui_page() {
            let page = splash.image().clone();
            self.renderer.clear_preview();
            self.upload_ui_page(page);
        }
        if let Some(splash) = self.startup_splash.as_mut() {
            splash.mark_presented(Instant::now());
        }
    }

    fn ensure_lobby_maps(&mut self) {
        if !self.lobby_maps.is_empty() {
            return;
        }
        self.lobby_maps = super::boot::list_install_boot_maps();
        if self.selected_map.is_none() {
            self.selected_map = self.lobby_maps.first().map(|m| m.file_name.clone());
        }
        if self.skirmish.preferred_map.is_none() {
            self.skirmish.preferred_map = self.selected_map.clone();
        }
        tracing::info!(
            count = self.lobby_maps.len(),
            selected = ?self.selected_map,
            "遭遇战地图列表已刷新"
        );
    }

    fn ensure_lobby_preview(&mut self) {
        let Some(name) = self.selected_map.clone()
        else {
            self.lobby_preview = None;
            self.lobby_preview_for = None;
            self.lobby_preview_job = None;
            return;
        };
        if self.lobby_preview_for.as_deref() == Some(name.as_str()) {
            return;
        }
        if self.lobby_preview_job.as_ref().is_some_and(|j| j.map_name() == name) {
            return;
        }
        self.lobby_preview_job = Some(PreviewJob::start(name));
    }

    fn poll_lobby_preview(&mut self) -> bool {
        let Some(job) = self.lobby_preview_job.as_ref()
        else {
            return false;
        };
        match job.try_take() {
            Ok(Some(result)) => {
                self.lobby_preview_job = None;
                let still_selected = self.selected_map.as_deref() == Some(result.map_name.as_str());
                if !still_selected {
                    return false;
                }
                match result.image {
                    Some(thumb) => {
                        tracing::info!(
                            map = %result.map_name,
                            w = thumb.width(),
                            h = thumb.height(),
                            "{}",
                            result.note
                        );
                        self.lobby_preview = Some(thumb);
                        self.lobby_preview_for = Some(result.map_name);
                    }
                    None => {
                        tracing::warn!(map = %result.map_name, "遭遇战大厅地图预览失败");
                        self.lobby_preview = None;
                        self.lobby_preview_for = Some(result.map_name);
                    }
                }
                true
            }
            Ok(None) => false,
            Err(()) => {
                self.lobby_preview_job = None;
                false
            }
        }
    }

    fn cycle_lobby_map(&mut self, delta: isize) {
        self.ensure_lobby_maps();
        if self.lobby_maps.is_empty() {
            self.selected_map = None;
            self.skirmish.preferred_map = None;
            return;
        }
        let cur = self.selected_map.as_ref().and_then(|name| self.lobby_maps.iter().position(|m| &m.file_name == name)).unwrap_or(0);
        let n = self.lobby_maps.len() as isize;
        let next = ((cur as isize + delta).rem_euclid(n)) as usize;
        self.select_lobby_map_index(next);
    }

    /// 跳到大厅地图列表首项或末项（空列表时清空选中）。
    fn jump_lobby_map_edge(&mut self, to_end: bool) {
        self.ensure_lobby_maps();
        if self.lobby_maps.is_empty() {
            self.selected_map = None;
            self.skirmish.preferred_map = None;
            return;
        }
        let idx = if to_end { self.lobby_maps.len() - 1 } else { 0 };
        self.select_lobby_map_index(idx);
    }

    fn select_lobby_map_index(&mut self, index: usize) {
        let Some(map) = self.lobby_maps.get(index)
        else {
            return;
        };
        self.selected_map = Some(map.file_name.clone());
        self.skirmish.preferred_map = self.selected_map.clone();
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    fn ensure_menu_assets(&mut self) {
        if self.menu_assets.is_some() {
            return;
        }
        let assets = load_menu_ui_assets();
        tracing::info!(
            ui_ini = ?assets.ui_ini_name,
            ui_ini_ok = assets.ui_ini_readable,
            ui_sections = assets.ui_ini.as_ref().map(|d| d.sections.len()),
            ui_shp_refs = assets.ui_ini_shp_refs.len(),
            has_source = assets.source.is_some(),
            "{}",
            assets.note
        );
        self.banner = assets.note.clone();
        self.menu_assets = Some(assets);
        self.refresh_ui_resolve_note();
    }

    /// 对当前页已声明资源名做可读性检查，并尝试解码 chrome（不绘制）。
    fn refresh_ui_resolve_note(&mut self) {
        let Some(assets) = self.menu_assets.as_ref()
        else {
            return;
        };
        let Some(source) = assets.source.as_ref()
        else {
            return;
        };
        let edition = assets.edition;
        let page = if self.screen == OriginalScreen::LoadScreen {
            page_resources_for_load_screen(&self.skirmish.side, self.window_width as u32, |name| source.resolve(name).is_some())
        }
        else {
            page_resources_from_slots_with_edition(self.screen, edition)
        };
        let Some(page) = page
        else {
            return;
        };
        let report = ui_resolve::resolve_page(source, &page);
        tracing::info!(
            screen = self.screen.as_str(),
            named = report.named,
            readable = report.readable,
            missing = report.missing.len(),
            "{}",
            report.banner_note()
        );
        let mut banner = if report.named == 0 {
            if assets.note.contains("槽位未填") { assets.note.clone() } else { format!("{} · {}", assets.note, report.banner_note()) }
        }
        else {
            format!("{} · {}", assets.note, report.banner_note())
        };

        // 影片缺失不挡 chrome 解码；仅非 BIK 缺口才清空解码缓存。
        // 装载页：只要国家背景可读就解码（进度条 / 失败钮可缺）。
        let only_movie_gaps = report.missing.iter().all(|m| m.to_ascii_lowercase().ends_with(".bik"));
        let load_bg_name = page.background.as_ref().map(|b| b.name.to_ascii_lowercase());
        let load_bg_ok = self.screen == OriginalScreen::LoadScreen
            && load_bg_name.as_ref().is_some_and(|bg| !report.missing.iter().any(|m| m.eq_ignore_ascii_case(bg)));
        if report.named > 0 && (only_movie_gaps || load_bg_ok) {
            let decoded = ui_decode::decode_page_chrome(source, &page);
            tracing::info!(
                screen = self.screen.as_str(),
                errors = decoded.errors.len(),
                chrome_ready = decoded.chrome_ready_for_enabled_buttons(&page),
                "{}",
                decoded.banner_note()
            );
            for err in &decoded.errors {
                tracing::warn!(screen = self.screen.as_str(), "UI 解码失败 · {err}");
            }
            banner = format!("{banner} · {}", decoded.banner_note());
            self.ui_decode_cache = Some(decoded);
        }
        else {
            self.ui_decode_cache = None;
        }

        if let Some(movie) = page.movie.as_ref() {
            match source.read(&movie.name) {
                Ok(bytes) => match MenuMoviePlayer::open(&movie.name, bytes) {
                    Ok(player) => {
                        tracing::info!(
                            name = %player.name(),
                            "主菜单影片播放器已就绪（自研 Bink）"
                        );
                        banner = format!("{banner} · {} 已解首帧", movie.name);
                        self.menu_movie = Some(player);
                        self.menu_movie_clock = Some(Instant::now());
                    }
                    Err(e) => {
                        tracing::warn!(name = %movie.name, "影片播放器启动失败 · {e}");
                        banner = format!("{banner} · {} 解码失败", movie.name);
                        self.menu_movie = None;
                        self.menu_movie_clock = None;
                    }
                },
                Err(_) => {
                    tracing::warn!(name = %movie.name, "影片不可读");
                    self.menu_movie = None;
                    self.menu_movie_clock = None;
                }
            }
        }
        else {
            self.menu_movie = None;
            self.menu_movie_clock = None;
        }

        self.banner = banner;
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

    /// 请求下一帧 GPU 回读并落盘为 `{name}_*.png`。
    fn queue_screenshot(&mut self, name: &'static str) {
        self.renderer.request_capture();
        self.pending_screenshot = Some(name);
    }

    fn flush_pending_screenshot(&mut self) {
        let Some(name) = self.pending_screenshot.take()
        else {
            return;
        };
        if let Some(err) = self.renderer.take_capture_error() {
            tracing::error!("截图回读失败 · screen={name} · {err}");
            self.banner = format!("截图失败 · {err}");
            return;
        }
        let Some(image) = self.renderer.take_capture()
        else {
            tracing::warn!("截图尚未就绪 · screen={name}");
            return;
        };
        match super::screenshot::save_screenshot(name, &image) {
            Ok(path) => {
                tracing::info!(%name, path = %path.display(), "关键页截图已保存");
                self.banner = format!("截图已保存 · {}", path.display());
            }
            Err(e) => tracing::error!("截图保存失败 · {e}"),
        }
    }

    fn ensure_menu_text_assets(&mut self) {
        if (self.menu_font.is_some() || self.menu_font_tried) && (self.menu_csf.is_some() || self.menu_csf_tried) {
            return;
        }
        let source = self.menu_assets.as_ref().and_then(|a| a.source.as_ref());

        if self.menu_font.is_none() && !self.menu_font_tried {
            self.menu_font_tried = true;
            match source.and_then(|s| s.read("game.fnt").ok()) {
                Some(bytes) => match FntFile::parse(&bytes) {
                    Ok(fnt) => {
                        tracing::info!(glyphs = fnt.glyph_count(), "菜单字体已解析 · game.fnt");
                        self.menu_font = Some(fnt);
                    }
                    Err(e) => tracing::warn!("game.fnt 解析失败 · {e}"),
                },
                None => tracing::warn!("game.fnt 不可读"),
            }
        }
        if self.menu_csf.is_none() && !self.menu_csf_tried {
            self.menu_csf_tried = true;
            let csf_bytes = source.and_then(|s| {
                // 资料片优先 `ra2md.csf`，再回退原版 `ra2.csf`。
                for name in ["ra2md.csf", "ra2.csf"] {
                    if let Ok(bytes) = s.read(name) {
                        return Some((name, bytes));
                    }
                }
                None
            });
            match csf_bytes {
                Some((name, bytes)) => match CsfFile::parse(&bytes) {
                    Ok(csf) => {
                        tracing::info!(entries = csf.len(), file = name, "菜单文案表已解析");
                        self.menu_csf = Some(csf);
                    }
                    Err(e) => tracing::warn!("{name} 解析失败 · {e}"),
                },
                None => tracing::warn!("未找到可读的 ra2.csf / ra2md.csf"),
            }
        }
    }

    /// 当前底栏可见切片；空串或切页进出/卡顿中视为无提示。
    fn status_line_visible(&self) -> Option<&str> {
        if self.shell_slide_busy() {
            return None;
        }
        let text = self.status_line.visible();
        if text.is_empty() { None } else { Some(text) }
    }

    /// 按当前页面与悬停入口解析 CSF 提示，提交给打字机。
    fn sync_status_line_from_hover(&mut self) {
        let Some(entry) = self.menu_hovered_entry
        else {
            self.status_line.clear();
            return;
        };
        let key = match self.screen {
            OriginalScreen::MainMenu => main_menu_csf_tooltip(entry),
            OriginalScreen::SinglePlayerMenu => single_player_csf_tooltip(entry),
            OriginalScreen::Campaign => campaign_csf_tooltip(entry),
            OriginalScreen::SkirmishLobby => skirmish_lobby_csf_tooltip(entry),
            _ => None,
        };
        let text = key.and_then(|k| resolve_csf_text(self.menu_csf.as_ref(), k)).unwrap_or_default();
        if text.is_empty() {
            self.status_line.clear();
        }
        else {
            self.status_line.set_text(text);
        }
    }

    /// 前置页：主菜单 / 单人 / 选项 / 遭遇战大厅 / 装载页上传合成 chrome；启动闪屏由独立 owner 保持。
    fn refresh_menu_backdrop(&mut self) {
        if matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
            // 对局 HUD 由 `MatchController::draw_frame` 维护，勿在此清空；仍预热字体。
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
                        let ai_csf = ra_components::skirmish_setup::SkirmishBootRequest::ai_difficulty_csf_key(&self.skirmish.difficulty);
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
                            country_combo_open: self.skirmish.open_combo == Some(ra_components::skirmish_setup::SkirmishComboKind::Country),
                            color_combo_open: self.skirmish.open_combo == Some(ra_components::skirmish_setup::SkirmishComboKind::Color),
                            ai_combo_open: self.skirmish.open_combo == Some(ra_components::skirmish_setup::SkirmishComboKind::Ai),
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

    /// 惰性解析 `audio.idx` / `audio.bag`，结果缓存在壳层。
    fn ensure_audio_bag(&mut self) {
        if self.audio_bag_tried {
            return;
        }
        self.audio_bag_tried = true;
        self.ensure_menu_assets();
        let idx_bytes = self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| s.read("audio.idx").ok());
        let bag_bytes = self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| s.read("audio.bag").ok());
        let Some((idx, bag)) = idx_bytes.zip(bag_bytes)
        else {
            tracing::debug!("audio.idx/audio.bag 不可读");
            return;
        };
        match AudioIndex::parse(&idx, bag) {
            Some(index) => {
                tracing::info!(entries = index.len(), "已缓存 audio.bag 索引");
                self.audio_bag = Some(index);
            }
            None => tracing::warn!("audio.idx 解析失败"),
        }
    }

    /// 从缓存的 bag 按候选名解码首个命中采样。
    fn decode_bag_named(&mut self, names: &[&str]) -> Option<PcmAudio> {
        self.ensure_audio_bag();
        let index = self.audio_bag.as_ref()?;
        for name in names {
            if let Some(pcm) = index.decode(name) {
                tracing::info!(%name, frames = pcm.samples.len(), "已从 audio.bag 解码采样");
                return Some(pcm);
            }
        }
        tracing::debug!(entries = index.len(), "audio.bag 未命中候选名");
        None
    }

    /// 从挂载源读逻辑文件名。
    fn read_asset_bytes(&self, name: &str) -> Option<Vec<u8>> {
        self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| s.read(name).ok())
    }

    /// 解析 INI；失败时仍可用 `soft_ini_get`。
    fn read_ini_doc(&self, name: &str) -> Option<IniDocument> {
        let bytes = self.read_asset_bytes(name)?;
        match IniDocument::parse(&bytes) {
            Ok(doc) => Some(doc),
            Err(e) => {
                tracing::debug!(%name, error = %e, "INI 严格解析失败，改用宽松扫描");
                None
            }
        }
    }

    /// `theme.ini` `[INTRO]` 的 `Sound=` 词干（缺省 `Grinder`）。
    fn menu_theme_sound_stem(&self) -> String {
        let from_doc = self
            .read_ini_doc("theme.ini")
            .as_ref()
            .and_then(|d| d.get("INTRO", "Sound"))
            .map(super::audio::theme_sound_stem)
            .map(str::to_string)
            .filter(|s| !s.is_empty());
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("theme.ini")
            .and_then(|b| super::audio::soft_ini_get(&b, "INTRO", "Sound"))
            .map(|s| super::audio::theme_sound_stem(&s).to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Grinder".into())
    }

    /// 规则里的主菜单点击事件 id（缺省 `MenuClick`）。
    fn menu_click_sound_id(&self) -> String {
        let from_doc = self
            .read_ini_doc("rules.ini")
            .as_ref()
            .and_then(|d| d.get("AudioVisual", "GUIMainButtonSound"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("rules.ini")
            .and_then(|b| super::audio::soft_ini_get(&b, "AudioVisual", "GUIMainButtonSound"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "MenuClick".into())
    }

    /// 规则里的壳层出去事件 id（缺省 `MenuSlideOut`）。
    fn menu_move_out_sound_id(&self) -> String {
        let from_doc = self
            .read_ini_doc("rules.ini")
            .as_ref()
            .and_then(|d| d.get("AudioVisual", "GUIMoveOutSound"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("rules.ini")
            .and_then(|b| super::audio::soft_ini_get(&b, "AudioVisual", "GUIMoveOutSound"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "MenuSlideOut".into())
    }

    /// 规则里的壳层进来事件 id（缺省 `MenuSlideIn`）。
    fn menu_move_in_sound_id(&self) -> String {
        let from_doc = self
            .read_ini_doc("rules.ini")
            .as_ref()
            .and_then(|d| d.get("AudioVisual", "GUIMoveInSound"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("rules.ini")
            .and_then(|b| super::audio::soft_ini_get(&b, "AudioVisual", "GUIMoveInSound"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "MenuSlideIn".into())
    }

    /// `sound.ini` 事件 → `Sounds=` 采样名列表。
    fn sound_event_sample_names(&self, event_id: &str) -> Vec<String> {
        let line = self
            .read_ini_doc("sound.ini")
            .as_ref()
            .and_then(|d| d.get(event_id, "Sounds"))
            .map(str::to_string)
            .or_else(|| self.read_asset_bytes("sound.ini").and_then(|b| super::audio::soft_ini_get(&b, event_id, "Sounds")))
            .unwrap_or_default();
        line.split_whitespace().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
    }

    /// 按 `theme.ini` 词干尝试 `{stem}.wav` / `{stem}.aud`（通常来自 `THEME.MIX`）。
    ///
    /// 不回退 `intro.aud`：那是别的曲目，不是主菜单 `Grinder`。
    fn decode_theme_track(&self, stem: &str) -> Option<PcmAudio> {
        let mut names: Vec<String> = Vec::new();
        for ext in ["wav", "aud"] {
            names.push(format!("{stem}.{ext}"));
        }
        for name in &names {
            let Some(bytes) = self.read_asset_bytes(name)
            else {
                continue;
            };
            let ext = name.rsplit_once('.').map(|(_, e)| e);
            match decode_audio_bytes(&bytes, ext) {
                Ok(pcm) => {
                    tracing::info!(
                        %name,
                        frames = pcm.samples.len() / pcm.channels.max(1) as usize,
                        rate = pcm.sample_rate,
                        "已加载菜单 BGM"
                    );
                    return Some(pcm);
                }
                Err(e) => {
                    tracing::warn!(%name, error = %e, "主题曲解码失败");
                }
            }
        }
        None
    }

    /// 主题曲缺失诊断（仅当前 `--path` 安装根）。
    fn warn_theme_unavailable(&self, stem: &str) {
        let theme_note = match self.read_asset_bytes("theme.mix").or_else(|| self.read_asset_bytes("Theme.mix")) {
            Some(bytes) if bytes.as_slice() == b"CLASS" || bytes.len() < 64 => {
                format!("theme.mix 为占位（{} 字节），无法读取 {stem}.wav", bytes.len())
            }
            Some(bytes) => format!("theme.mix 可读（{} 字节）但未解出 {stem}.wav/.aud", bytes.len()),
            None => format!("无 theme.mix，且未解出 {stem}.wav/.aud"),
        };
        tracing::warn!(%stem, %theme_note, "菜单主题曲不可用，BGM 静音");
    }

    /// 惰性装载菜单 BGM / 点击 / 切页进出采样。
    fn ensure_menu_audio_assets(&mut self) {
        if (self.menu_bgm.is_some() || self.menu_bgm_tried) && self.menu_click.is_some() && self.menu_move_out_tried && self.menu_move_in_tried
        {
            return;
        }
        self.ensure_menu_assets();
        if self.menu_bgm.is_none() && !self.menu_bgm_tried {
            self.menu_bgm_tried = true;
            let stem = self.menu_theme_sound_stem();
            if let Some(pcm) = self.decode_theme_track(&stem) {
                self.menu_bgm = Some(pcm);
            }
            else {
                self.warn_theme_unavailable(&stem);
            }
        }
        if self.menu_click.is_none() {
            let event_id = self.menu_click_sound_id();
            let mut candidates: Vec<String> = self.sound_event_sample_names(&event_id);
            // 零售 `[MenuClick] Sounds=umenucl1`；sound.ini 解析失败时仍走 bag 名。
            for fallback in ["umenucl1", "UMENUCL1", "MenuClick"] {
                if !candidates.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    candidates.push(fallback.into());
                }
            }
            let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
            let loaded = self.decode_bag_named(&refs);
            self.menu_click = Some(loaded.unwrap_or_else(|| {
                tracing::warn!(%event_id, "菜单点击采样未命中，使用合成占位");
                super::audio::synthetic_ui_click()
            }));
        }
        if !self.menu_move_out_tried {
            self.menu_move_out_tried = true;
            let event_id = self.menu_move_out_sound_id();
            let mut candidates: Vec<String> = self.sound_event_sample_names(&event_id);
            for fallback in ["uslide2", "USLIDE2", "MenuSlideOut"] {
                if !candidates.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    candidates.push(fallback.into());
                }
            }
            let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
            self.menu_move_out = self.decode_bag_named(&refs);
            if self.menu_move_out.is_none() {
                tracing::warn!(%event_id, "壳层出去采样未命中");
            }
        }
        if !self.menu_move_in_tried {
            self.menu_move_in_tried = true;
            let event_id = self.menu_move_in_sound_id();
            let mut candidates: Vec<String> = self.sound_event_sample_names(&event_id);
            for fallback in ["uslide1", "USLIDE1", "MenuSlideIn"] {
                if !candidates.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    candidates.push(fallback.into());
                }
            }
            let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
            self.menu_move_in = self.decode_bag_named(&refs);
            if self.menu_move_in.is_none() {
                tracing::warn!(%event_id, "壳层进来采样未命中");
            }
        }
    }

    /// 前置壳层页播 BGM；离开壳层则停。
    fn sync_shell_audio(&mut self) {
        self.ensure_menu_audio_assets();
        let wants_bgm = matches!(
            self.screen,
            OriginalScreen::MainMenu
                | OriginalScreen::SinglePlayerMenu
                | OriginalScreen::Campaign
                | OriginalScreen::Options
                | OriginalScreen::ExitConfirm
                | OriginalScreen::SkirmishLobby
                | OriginalScreen::ChooseMap
                | OriginalScreen::Network
        );
        if wants_bgm {
            if !self.menu_bgm_playing {
                if let (Some(audio), Some(bgm)) = (self.audio.as_mut(), self.menu_bgm.as_ref()) {
                    audio.play_music_loop(bgm);
                    self.menu_bgm_playing = true;
                }
            }
        }
        else if self.menu_bgm_playing {
            if let Some(audio) = self.audio.as_mut() {
                audio.stop_music();
            }
            self.menu_bgm_playing = false;
        }
    }

    /// 菜单按钮按下时播一次点击音。
    fn play_menu_click(&mut self) {
        self.ensure_menu_audio_assets();
        if let (Some(audio), Some(click)) = (self.audio.as_mut(), self.menu_click.as_ref()) {
            audio.play_sfx(click);
        }
    }

    /// 壳层出去波浪开始时播一次（`GUIMoveOutSound`）。
    fn play_menu_move_out(&mut self) {
        self.ensure_menu_audio_assets();
        if let (Some(audio), Some(sfx)) = (self.audio.as_mut(), self.menu_move_out.as_ref()) {
            audio.play_sfx(sfx);
        }
    }

    /// 壳层进来波浪开始时播一次（`GUIMoveInSound`）。
    fn play_menu_move_in(&mut self) {
        self.ensure_menu_audio_assets();
        if let (Some(audio), Some(sfx)) = (self.audio.as_mut(), self.menu_move_in.as_ref()) {
            audio.play_sfx(sfx);
        }
    }

    /// 战役选边悬停切入语音（`sound.ini` Allied/BootCamp/SovietCampaignSelect）。
    fn play_campaign_side_hover(&mut self, side: &str) {
        let slot = match side {
            "allied" => 0,
            "tutorial" => 1,
            "soviet" => 2,
            _ => return,
        };
        let event_id = match side {
            "allied" => "AlliedCampaignSelect",
            "tutorial" => "BootCampSelect",
            "soviet" => "SovietCampaignSelect",
            _ => return,
        };
        if self.campaign_side_sfx[slot].is_none() {
            self.ensure_menu_assets();
            let mut names: Vec<String> = self
                .sound_event_sample_names(event_id)
                .into_iter()
                .map(|s| s.trim().trim_start_matches(['$', '#']).to_string())
                .filter(|s| !s.is_empty())
                .collect();
            // 零售缺省采样名（sound.ini 解析失败时仍可从 bag 取）。
            for fallback in match side {
                "allied" => ["itanatc", "ITANATC"],
                "tutorial" => ["igisea", "IGISEA"],
                _ => ["vgrsatc", "VGRSATC"],
            } {
                if !names.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    names.push(fallback.into());
                }
            }
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            self.campaign_side_sfx[slot] = self.decode_bag_named(&refs);
            if self.campaign_side_sfx[slot].is_none() {
                tracing::warn!(%event_id, "战役选边悬停采样未命中");
            }
        }
        if let (Some(audio), Some(pcm)) = (self.audio.as_mut(), self.campaign_side_sfx[slot].as_ref()) {
            audio.play_sfx(pcm);
        }
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

    /// 进入选图页并快照当前优选地图（取消时还原）。
    fn open_choose_map_page(&mut self) {
        self.ensure_lobby_maps();
        self.choose_map_revert = Some(self.skirmish.preferred_map.clone());
        if self.selected_map.is_none() {
            self.selected_map = self.skirmish.preferred_map.clone().or_else(|| self.lobby_maps.first().map(|m| m.file_name.clone()));
        }
        self.set_screen(OriginalScreen::ChooseMap);
        self.banner = "选图".into();
        self.refresh_shell_title();
    }

    /// 使用当前选中地图并返回遭遇战大厅。
    fn confirm_choose_map(&mut self) {
        self.choose_map_revert = None;
        if let Some(name) = self.selected_map.clone() {
            self.skirmish.preferred_map = Some(name);
        }
        self.set_screen(OriginalScreen::SkirmishLobby);
        self.banner = "已选用地图".into();
        self.refresh_shell_title();
    }

    /// 取消选图：还原进入页前的优选地图并回大厅。
    fn cancel_choose_map(&mut self) {
        if let Some(prev) = self.choose_map_revert.take() {
            self.skirmish.preferred_map = prev.clone();
            self.selected_map = prev.or_else(|| self.lobby_maps.first().map(|m| m.file_name.clone()));
            self.ensure_lobby_preview();
        }
        self.set_screen(OriginalScreen::SkirmishLobby);
        self.banner = "已取消选图".into();
        self.refresh_shell_title();
    }

    /// 进入选项页并快照当前显示档 / 音量 / 质感草稿。
    fn open_options_page(&mut self) {
        let (music, sound) = self.audio.as_ref().map(|a| (a.music_volume(), a.sfx_volume())).unwrap_or((0.4, 0.7));
        self.options_volume_baseline = Some((music, sound));
        self.options_present_baseline = Some(self.present);
        self.options_pointer_consumed = false;
        self.options_state = Some(ra_components::options_dialog::OptionsDialogState::from_shell(self.display_mode, music, sound, self.present));
        self.set_screen(OriginalScreen::Options);
    }

    /// 丢弃选项草稿并还原进入页前的音量 / 质感预览。
    fn discard_options_draft(&mut self) {
        if let Some((music, sound)) = self.options_volume_baseline.take() {
            if let Some(audio) = self.audio.as_mut() {
                audio.set_music_volume(music);
                audio.set_sfx_volume(sound);
            }
        }
        if let Some(present) = self.options_present_baseline.take() {
            self.present = present.sanitized();
        }
        self.options_state = None;
        self.options_pointer_consumed = false;
    }

    /// 接受选项草稿：音量与质感立刻生效并落盘，分辨率变更则改窗。
    fn apply_options_accept(&mut self) {
        let Some(state) = self.options_state.take()
        else {
            self.options_volume_baseline = None;
            self.options_present_baseline = None;
            self.set_screen(OriginalScreen::MainMenu);
            return;
        };
        self.options_volume_baseline = None;
        self.options_present_baseline = None;
        self.options_pointer_consumed = false;
        let music = state.music_volume_f32();
        let sound = state.sound_volume_f32();
        self.apply_audio_volumes(music, sound);
        match ra_config::DesktopSettings::persist_audio_volumes(music, sound) {
            Ok(()) => tracing::info!(music, sound, "已写入壳层音量"),
            Err(e) => tracing::warn!(error = %e, "写入壳层音量失败"),
        }
        self.apply_present_feel(state.present);
        match ra_config::DesktopSettings::persist_present_feel(self.present) {
            Ok(()) => tracing::info!(mode = self.present.mode.as_str(), dither = self.present.dither, "已写入 [present]"),
            Err(e) => tracing::warn!(error = %e, "写入 [present] 失败"),
        }
        if state.display_mode != self.display_mode {
            self.apply_display_mode(state.display_mode);
        }
        self.banner = "选项已保存".into();
        self.set_screen(OriginalScreen::MainMenu);
        self.refresh_shell_title();
    }

    /// 应用指定 `DisplayMode`（改窗、落盘、刷新）。
    fn apply_display_mode(&mut self, mode: DisplayMode) {
        self.display_mode = mode;
        let (w, h) = self.display_mode.size();
        self.window_width = w as f64;
        self.window_height = h as f64;
        if let Some(window) = self.window.as_ref() {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(self.window_width, self.window_height));
        }
        match ra_config::DesktopSettings::persist_display_mode(self.display_mode) {
            Ok(()) => {
                tracing::info!(display_mode = self.display_mode.as_str(), "已写入 display_mode");
                self.banner = format!("分辨率 · {}", self.display_mode.as_str());
            }
            Err(e) => {
                tracing::warn!(error = %e, "写入 display_mode 失败");
                self.banner = format!("分辨率 · {} · 写入失败", self.display_mode.as_str());
            }
        }
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    fn refresh_shell_title(&mut self) {
        let Some(window) = &self.window
        else {
            return;
        };
        if matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
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
            OriginalScreen::Match | OriginalScreen::Results => unreachable!(),
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
        self.pending_after_load = Some(OriginalScreen::Match);
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
        match self.match_ctrl.as_mut() {
            Some(ctrl) => ctrl.apply_boot(boot, &mut self.renderer),
            None => {
                self.match_ctrl = Some(MatchController::from_boot(boot, self.status_path.clone(), self.test_scene.clone()));
            }
        }
        let ok = self.match_ctrl.as_ref().is_some_and(|c| c.has_session());
        let target = self.pending_after_load.take().unwrap_or(OriginalScreen::Match);
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

    fn apply_nav(&mut self, nav: MatchNav) {
        match nav {
            MatchNav::None => {}
            MatchNav::Rematch => {
                self.banner = "重开…".into();
                self.begin_skirmish_load();
            }
            MatchNav::ToResults => self.set_screen(OriginalScreen::Results),
            MatchNav::ToMainMenu => {
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
            OriginalScreen::Match | OriginalScreen::Results => {}
        }
    }

    fn redraw(&mut self) {
        if self.screen.pumps_session() {
            if let Some(ctrl) = self.match_ctrl.as_mut() {
                let prev = ctrl.take_pump_clock();
                let dt = Instant::now().duration_since(prev).as_secs_f64();
                let (nav, sim_dt) = ctrl.pump(dt);
                self.renderer.timings.simulation = Some(sim_dt);
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str(), self.menu_font.as_ref());
                self.apply_nav(nav);
            }
        }
        else if self.screen.requires_session() {
            if let Some(ctrl) = self.match_ctrl.as_mut() {
                let _ = ctrl.take_pump_clock();
                self.renderer.timings.simulation = None;
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str(), self.menu_font.as_ref());
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

impl ApplicationHandler for AppShell {
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
                    || matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results | OriginalScreen::Splash)
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
                if !matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
                    self.refresh_menu_backdrop();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let (w, h) = self.display_mode.size();
                self.window_width = w as f64;
                self.window_height = h as f64;
                if !matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
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
                // 对局页也走同一截图路径（不交给 MatchController）。
                self.queue_screenshot(self.screen.as_str());
                return;
            }
            _ => {}
        }

        match self.screen {
            OriginalScreen::Match | OriginalScreen::Results => {
                let accept = self.screen.accepts_match_commands();
                let Some(window) = self.window.clone()
                else {
                    return;
                };
                if let Some(ctrl) = self.match_ctrl.as_mut() {
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

/// 战役难度轨鼠标 X → 档位 0..=2（与遭遇战滑条同一套整数映射）。
pub fn campaign_difficulty_from_track_x(track: ui_layout::RectPx, mouse_x: i32) -> u8 {
    let travel = (track.w - 12).max(1);
    let rel = (mouse_x - track.x - 6).clamp(0, travel);
    ((rel * 2 + travel / 2) / travel).clamp(0, 2) as u8
}

/// 解析启动参数并进入事件循环。
pub fn run_shell() -> RaResult<()> {
    let (mode, display_mode, music_volume, sound_volume, present, load_min_secs, shell_slide_gap_secs, status_path, test_scene) =
        resolve_launch()?;

    let event_loop = EventLoop::new().map_err(|e| RaError::Msg(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = match mode {
        #[cfg(feature = "test-harness")]
        LaunchMode::DirectMatch(boot) => {
            if let Some(game) = boot.session.as_ref().and_then(|s| s.game()) {
                tracing::info!("preview_origin=({}, {}) entities={}", game.preview_origin_x, game.preview_origin_y, game.world.entity_count());
            }
            AppShell::with_match(boot, display_mode.size().0 as f64, display_mode.size().1 as f64, status_path, test_scene)
        }
        LaunchMode::MainMenu => {
            let _ = (status_path, test_scene);
            AppShell::with_main_menu(display_mode)
        }
    };
    app.apply_audio_volumes(music_volume, sound_volume);
    app.apply_present_feel(present);
    app.load_min_secs = load_min_secs;
    app.shell_slide_gap_secs = shell_slide_gap_secs;

    event_loop.run_app(&mut app).map_err(|e| RaError::Msg(e.to_string()))?;
    tracing::info!("事件循环结束");
    Ok(())
}

enum LaunchMode {
    #[cfg(feature = "test-harness")]
    DirectMatch(BootResult),
    MainMenu,
}

fn resolve_launch() -> RaResult<(LaunchMode, DisplayMode, f32, f32, PresentFeel, f64, f64, Option<PathBuf>, Option<String>)> {
    #[cfg(feature = "test-harness")]
    {
        if let Some(scene) = super::test_boot::requested_scene() {
            let status_path = super::test_boot::status_path();
            let window_width = super::test_boot::TEST_WINDOW_WIDTH;
            let window_height = super::test_boot::TEST_WINDOW_HEIGHT;
            tracing::info!(
                "test-harness scene={scene} window={}x{} status={}",
                window_width,
                window_height,
                status_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "—".into())
            );
            let t = super::test_boot::boot_scene(&scene)?;
            tracing::info!("boot: {} · session=ok", t.note);
            return Ok((
                LaunchMode::DirectMatch(BootResult { note: t.note, engine: Some(t.engine), session: Some(t.session), preview: t.preview }),
                DisplayMode::DEFAULT,
                0.4,
                0.7,
                PresentFeel::DEFAULT,
                0.0,
                0.0,
                status_path,
                Some(scene),
            ));
        }
    }

    // 产品路径：主菜单起；对局须手动经菜单进入（自动测试用 DirectMatch 场景）。
    let (settings, diagnostics) = super::config::load_desktop_config_with_diagnostics();
    for d in &diagnostics {
        tracing::info!(source = %d.source, "{}", d.message);
    }
    let display_mode = settings.display_mode;
    tracing::info!(
        display_mode = display_mode.as_str(),
        music_volume = settings.music_volume,
        sound_volume = settings.sound_volume,
        load_min_secs = settings.load_min_secs,
        shell_slide_gap_secs = settings.shell_slide_gap_secs,
        present_mode = settings.present.mode.as_str(),
        ra2_dir = %settings.ra2_dir.display(),
        "desktop launch settings"
    );
    Ok((
        LaunchMode::MainMenu,
        display_mode,
        settings.music_volume,
        settings.sound_volume,
        settings.present,
        settings.load_min_secs,
        settings.shell_slide_gap_secs,
        None,
        None,
    ))
}
