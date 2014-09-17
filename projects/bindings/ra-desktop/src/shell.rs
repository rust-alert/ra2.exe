//! 单窗口应用外壳：页面导航、窗口生命周期；对局逻辑委托 `MatchController`。

use std::{path::PathBuf, sync::Arc, time::Instant};

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

use crate::{
    boot::BootResult,
    load_job::LoadJob,
    match_ctrl::{MatchController, MatchNav},
    menu_action::MenuAction,
    preview_job::PreviewJob,
    screen::OriginalScreen,
    skirmish_setup::{SkirmishBootRequest, side_flag_pcx},
    ui_assets::{MenuUiAssets, load_menu_ui_assets},
    ui_compose::{self, SkirmishChromeSprites},
    ui_decode, ui_hit, ui_layout,
    ui_movie::MenuMoviePlayer,
    ui_page::page_resources_from_slots,
    ui_present,
    ui_resolve, ui_slots,
};

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
    /// 闪屏开始时刻。
    splash_started: Option<Instant>,
    /// 闪屏最短展示秒数。
    splash_min_secs: f64,
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
    /// 遭遇战大厅可选地图。
    lobby_maps: Vec<crate::boot::BootMapCandidate>,
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
    /// 主菜单当前悬停的按钮入口 id（悬停帧合成）。
    menu_hovered_entry: Option<&'static str>,
    /// 菜单字体（`game.fnt`）。
    menu_font: Option<FntFile>,
    /// 菜单文案表（`ra2.csf` / `ra2md.csf`）。
    menu_csf: Option<CsfFile>,
    /// 主菜单 / 单人页循环影片。
    menu_movie: Option<MenuMoviePlayer>,
    /// 影片时钟（`tick` 用）。
    menu_movie_clock: Option<Instant>,
    /// 壳层面板动画时钟（`sdwrnanm` WARNING 屏）。
    menu_panel_anim_clock: Option<Instant>,
    /// 面板动画未消耗的累计秒（跨帧保留，避免每帧 dt 小于步长时永不推进）。
    menu_panel_anim_accum: f64,
    /// `sdwrnanm` 动画帧序号（对多帧 SHP 取模）。
    menu_panel_anim_frame: usize,
    /// 闪屏 PCX 已上传（避免每帧重解）。
    splash_uploaded: bool,
    /// 下一帧回读后落盘的截图短名（`OriginalScreen::as_str`）；F12 手动截图用。
    pending_screenshot: Option<&'static str>,
    /// 自动关键页截图去重（仅 `test-harness`）。
    #[cfg(feature = "test-harness")]
    auto_screenshots: crate::screenshot::AutoScreenshotTracker,
    /// 遭遇战大厅阵营 / 难度（进入装载请求）。
    skirmish: SkirmishBootRequest,
    /// 战役选边：`allied` / `tutorial` / `soviet`。
    campaign_side: Option<&'static str>,
    /// 战役难度档：0 易 / 1 中 / 2 难。
    campaign_difficulty: u8,
    /// 战役难度滑条是否正在拖动。
    campaign_dragging: bool,
    /// 战役左栏按下是否已消费（难度滑条，勿再走点击轮换）。
    campaign_pointer_consumed: bool,
    /// 桌面音频输出（设备不可用则为 `None`）。
    audio: Option<crate::audio::ShellAudio>,
    /// 主菜单 BGM PCM（`theme.ini` `[INTRO]` → `{Sound}.wav`）。
    menu_bgm: Option<PcmAudio>,
    /// 是否已尝试装载菜单 BGM（失败后不再每帧重试）。
    menu_bgm_tried: bool,
    /// 菜单点击音效 PCM（`GUIMainButtonSound` → `sound.ini` → `audio.bag`）。
    menu_click: Option<PcmAudio>,
    /// 当前是否已在播壳层 BGM。
    menu_bgm_playing: bool,
    /// 已解析的 `audio.bag` 索引（惰性）。
    audio_bag: Option<AudioIndex>,
    /// 是否已尝试装载 `audio.bag`（避免反复读盘）。
    audio_bag_tried: bool,
    /// 选项页草稿（进入 Options 时创建，接受/取消后清空）。
    options_state: Option<crate::options_dialog::OptionsDialogState>,
    /// 进入选项页时的音量快照（取消时还原实时预览）。
    options_volume_baseline: Option<(f32, f32)>,
    /// 本轮按下已由左栏控件消费（释放时勿再走右栏命中）。
    options_pointer_consumed: bool,
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
            splash_started: None,
            splash_min_secs: 3.0,
            splash_preload_done: false,
            splash_skip: false,
            pending_after_load: None,
            cursor: (0.0, 0.0),
            load_job: None,
            load_started: None,
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
            menu_hovered_entry: None,
            menu_font: None,
            menu_csf: None,
            menu_movie: None,
            menu_movie_clock: None,
            menu_panel_anim_clock: None,
            menu_panel_anim_accum: 0.0,
            menu_panel_anim_frame: 0,
            splash_uploaded: false,
            pending_screenshot: None,
            #[cfg(feature = "test-harness")]
            auto_screenshots: crate::screenshot::AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
            campaign_side: None,
            campaign_difficulty: 1,
            campaign_dragging: false,
            campaign_pointer_consumed: false,
            audio: crate::audio::ShellAudio::try_open(),
            menu_bgm: None,
            menu_bgm_tried: false,
            menu_click: None,
            menu_bgm_playing: false,
            audio_bag: None,
            audio_bag_tried: false,
            options_state: None,
            options_volume_baseline: None,
            options_pointer_consumed: false,
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
            splash_started: None,
            splash_min_secs: 3.0,
            splash_preload_done: false,
            splash_skip: false,
            pending_after_load: None,
            cursor: (0.0, 0.0),
            load_job: None,
            load_started: None,
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
            menu_hovered_entry: None,
            menu_font: None,
            menu_csf: None,
            menu_movie: None,
            menu_movie_clock: None,
            menu_panel_anim_clock: None,
            menu_panel_anim_accum: 0.0,
            menu_panel_anim_frame: 0,
            splash_uploaded: false,
            pending_screenshot: None,
            #[cfg(feature = "test-harness")]
            auto_screenshots: crate::screenshot::AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
            campaign_side: None,
            campaign_difficulty: 1,
            campaign_dragging: false,
            campaign_pointer_consumed: false,
            audio: crate::audio::ShellAudio::try_open(),
            menu_bgm: None,
            menu_bgm_tried: false,
            menu_click: None,
            menu_bgm_playing: false,
            audio_bag: None,
            audio_bag_tried: false,
            options_state: None,
            options_volume_baseline: None,
            options_pointer_consumed: false,
        }
    }

    /// 按配置应用壳层 BGM / 短音效音量（设备缺失时无操作）。
    pub fn apply_audio_volumes(&mut self, music_volume: f32, sound_volume: f32) {
        if let Some(audio) = self.audio.as_mut() {
            audio.set_music_volume(music_volume);
            audio.set_sfx_volume(sound_volume);
            tracing::info!(
                music_volume = audio.music_volume(),
                sound_volume = audio.sfx_volume(),
                "已应用壳层音量"
            );
        }
    }

    /// 应用壳层质感呈现配置（上传 UI 页前生效）。
    pub fn apply_present_feel(&mut self, present: PresentFeel) {
        self.present = present.sanitized();
        tracing::info!(
            mode = self.present.mode.as_str(),
            quantize = self.present.quantize.as_str(),
            gamma = self.present.gamma,
            highlight_roll_off = self.present.highlight_roll_off,
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

    /// 选项页按下：左栏优先；右栏仍走原有 pressed 精灵。
    fn handle_options_press(&mut self) -> bool {
        let layout = crate::options_dialog::OptionsDialogLayout::new();
        let (x, y) = self.shell_cursor_px();
        let Some(hit) = self
            .options_state
            .as_mut()
            .and_then(|state| state.on_press(&layout, x, y))
        else {
            return false;
        };
        use crate::options_dialog::OptionsHit;
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
                self.refresh_menu_backdrop();
                true
            }
        }
    }

    /// 选项页拖动滑条。
    fn handle_options_drag(&mut self) -> bool {
        let layout = crate::options_dialog::OptionsDialogLayout::new();
        let (x, y) = self.shell_cursor_px();
        let dragged = self
            .options_state
            .as_mut()
            .map(|state| state.dragging.is_some() && state.on_drag(&layout, x, y))
            .unwrap_or(false);
        if !dragged {
            return false;
        }
        self.sync_options_live_volumes();
        self.refresh_menu_backdrop();
        true
    }

    /// 从挂载源解码 PCX → RGBA；品红 `(255,0,255)` 作色键透明（旗标索引未必为 0）。
    fn load_pcx_rgba(source: &crate::fs_source::GameAssetSource, name: &str) -> Option<RgbaImage> {
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
        let side = self.skirmish.side.clone();
        let need_flag = self.skirmish_chrome_side.as_deref() != Some(side.as_str());
        let need_base = self.skirmish_chrome.as_ref().map(|c| c.checkbox_off.is_none()).unwrap_or(true);
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
        }
        if need_flag {
            let flag = Self::load_pcx_rgba(source, side_flag_pcx(&side));
            chrome.ai_flag = flag.clone();
            chrome.flag = flag;
            self.skirmish_chrome_side = Some(side);
        }
        self.skirmish_chrome = Some(chrome);
    }

    /// 遭遇战左栏按下：勾选 / 滑条优先于右栏按钮。
    fn handle_skirmish_press(&mut self) -> bool {
        let layout = ui_layout::skirmish_lobby_layout(0, 0);
        let (x, y) = self.shell_cursor_px();
        if self.skirmish.on_press(&layout, x, y).is_none() {
            return false;
        }
        self.skirmish_pointer_consumed = true;
        self.play_menu_click();
        self.refresh_menu_backdrop();
        true
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

    /// 战役难度滑条按下：按轨坐标落档并开始拖动。
    fn handle_campaign_press(&mut self) -> bool {
        let layout = ui_layout::campaign_layout(0, 0);
        let (x, y) = self.shell_cursor_px();
        if !(layout.difficulty_track.contains(x, y)
            || layout.difficulty_label.contains(x, y)
            || layout.difficulty_value.contains(x, y))
        {
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

    /// 闪屏每帧：推进预处理；条件满足则只切到主菜单。
    fn tick_splash(&mut self) {
        if self.screen != OriginalScreen::Splash {
            return;
        }
        if self.splash_started.is_none() {
            self.splash_started = Some(Instant::now());
        }
        // 先保证标题图在屏，再做菜单资源预热（预热不得切换页面、不得清空 UI 页）。
        if !self.splash_uploaded || !self.renderer.has_ui_page() {
            self.upload_splash_backdrop();
            self.splash_uploaded = true;
        }
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
        let elapsed = self.splash_started.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
        let min_ok = elapsed >= self.splash_min_secs;
        if self.splash_preload_done && (min_ok || self.splash_skip) {
            tracing::info!(elapsed, min = self.splash_min_secs, skip = self.splash_skip, "闪屏结束 → 主菜单");
            self.set_screen(OriginalScreen::MainMenu);
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
        let Some(page) = page_resources_from_slots(OriginalScreen::MainMenu)
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

    /// 闪屏画面：解码槽位中的 `title.pcx`（失败则黑底占位并写明原因）。
    fn upload_splash_backdrop(&mut self) {
        self.ensure_menu_assets();
        let pcx_name = ui_slots::slots_for(OriginalScreen::Splash)
            .and_then(|s| s.background_pcx)
            .unwrap_or("title.pcx");
        let decoded = match self.menu_assets.as_ref().and_then(|a| a.source.as_ref()) {
            None => {
                tracing::warn!(name = pcx_name, "闪屏 PCX 跳过 · 安装资源源未挂载（检查 ra2_dir / edition）");
                None
            }
            Some(src) => match src.read(pcx_name) {
                Ok(bytes) => match ra_assets::parse_pcx(&bytes) {
                    Ok(img) => RgbaImage::from_raw(img.width, img.height, img.rgba),
                    Err(e) => {
                        tracing::warn!(name = pcx_name, "闪屏 PCX 解析失败 · {e}");
                        None
                    }
                },
                Err(e) => {
                    tracing::warn!(name = pcx_name, "闪屏 PCX 不可读 · {e}");
                    None
                }
            },
        };
        if let Some(page) = decoded {
            tracing::info!(name = pcx_name, w = page.width(), h = page.height(), "闪屏 PCX 已上传");
            if !self.banner.contains(pcx_name) {
                self.banner = format!("{} · {pcx_name} {}×{}", self.banner, page.width(), page.height());
                self.refresh_shell_title();
            }
            self.renderer.clear_preview();
            self.upload_ui_page(page);
            return;
        }
        if !self.banner.contains("闪屏缺图") {
            self.banner = format!("{} · 闪屏缺图 {pcx_name}", self.banner);
            self.refresh_shell_title();
        }
        let w = ui_layout::SHELL_BASE_W as u32;
        let h = ui_layout::SHELL_BASE_H as u32;
        let pixels = vec![0u8; (w as usize) * (h as usize) * 4];
        if let Some(page) = RgbaImage::from_raw(w, h, pixels) {
            self.renderer.clear_preview();
            self.upload_ui_page(page);
        }
    }

    fn ensure_lobby_maps(&mut self) {
        if !self.lobby_maps.is_empty() {
            return;
        }
        self.lobby_maps = crate::boot::list_install_boot_maps();
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
        let Some(page) = page_resources_from_slots(self.screen)
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
        let only_movie_gaps = report.missing.iter().all(|m| m.to_ascii_lowercase().ends_with(".bik"));
        if report.named > 0 && only_movie_gaps {
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
            tracing::info!("页面 {} → {}", self.screen.as_str(), next.as_str());
            self.screen = next;
            self.menu_pressed_entry = None;
            self.menu_hovered_entry = None;
            if !matches!(
                next,
                OriginalScreen::MainMenu
                    | OriginalScreen::SinglePlayerMenu
                    | OriginalScreen::Options
                    | OriginalScreen::ExitConfirm
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
        match crate::screenshot::save_screenshot(name, &image) {
            Ok(path) => {
                tracing::info!(%name, path = %path.display(), "关键页截图已保存");
                self.banner = format!("截图已保存 · {}", path.display());
            }
            Err(e) => tracing::error!("截图保存失败 · {e}"),
        }
    }

    fn ensure_menu_text_assets(&mut self) {
        let font_bytes = self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| s.read("game.fnt").ok());
        let csf_bytes = self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| {
            // 资料片优先 `ra2md.csf`，再回退原版 `ra2.csf`。
            for name in ["ra2md.csf", "ra2.csf"] {
                if let Ok(bytes) = s.read(name) {
                    return Some((name, bytes));
                }
            }
            None
        });

        if self.menu_font.is_none() {
            if let Some(bytes) = font_bytes {
                match FntFile::parse(&bytes) {
                    Ok(fnt) => {
                        tracing::info!(glyphs = fnt.glyph_count(), "菜单字体已解析 · game.fnt");
                        self.menu_font = Some(fnt);
                    }
                    Err(e) => tracing::warn!("game.fnt 解析失败 · {e}"),
                }
            }
            else {
                tracing::warn!("game.fnt 不可读");
            }
        }
        if self.menu_csf.is_none() {
            if let Some((name, bytes)) = csf_bytes {
                match CsfFile::parse(&bytes) {
                    Ok(csf) => {
                        tracing::info!(entries = csf.len(), file = name, "菜单文案表已解析");
                        self.menu_csf = Some(csf);
                    }
                    Err(e) => tracing::warn!("{name} 解析失败 · {e}"),
                }
            }
            else {
                tracing::warn!("未找到可读的 ra2.csf / ra2md.csf");
            }
        }
    }

    /// 前置页：主菜单 / 单人 / 选项 / 遭遇战大厅上传合成 chrome；闪屏独立保留 `title.pcx`。
    fn refresh_menu_backdrop(&mut self) {
        if matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
            self.renderer.clear_ui_page();
            return;
        }
        // 闪屏是独立产品页：禁止走菜单合成路径，更不能 clear 掉已上传的 title.pcx。
        if self.screen == OriginalScreen::Splash {
            if !self.splash_uploaded || !self.renderer.has_ui_page() {
                self.upload_splash_backdrop();
                self.splash_uploaded = true;
            }
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
        ) {
            if self.screen == OriginalScreen::SkirmishLobby {
                self.ensure_lobby_maps();
                self.ensure_lobby_preview();
            }
            if matches!(self.screen, OriginalScreen::Campaign | OriginalScreen::SkirmishLobby) {
                self.ensure_skirmish_chrome();
            }
            // 大厅预览并入 UI 页合成，避免与 `set_map_preview` 双通道抢相机。
            self.renderer.clear_preview();
            if let Some(decoded) = self.ui_decode_cache.as_ref() {
                let movie = self.menu_movie.as_ref().and_then(|m| m.frame());
                let page = match self.screen {
                    OriginalScreen::MainMenu => ui_compose::compose_main_menu_page(
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
                    OriginalScreen::SinglePlayerMenu => ui_compose::compose_single_player_page(
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
                    OriginalScreen::Campaign => {
                        let track_thumb = self.skirmish_chrome.as_ref().and_then(|c| c.track_thumb.as_ref());
                        ui_compose::compose_campaign_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            ui_compose::CampaignPaint {
                                selected_side: self.campaign_side,
                                difficulty: self.campaign_difficulty,
                                track_thumb,
                            },
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
                        let map_name = self
                            .selected_map
                            .clone()
                            .or_else(|| self.lobby_maps.first().map(|m| m.file_name.clone()))
                            .unwrap_or_default();
                        let country = self.skirmish.side.clone();
                        let ai_name = self
                            .menu_csf
                            .as_ref()
                            .and_then(|c| c.get("GUI:AIHard").map(|s| s.to_string()))
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "Hard AI".into());
                        let paint = ui_compose::SkirmishLobbyPaint {
                            map_name: map_name.as_str(),
                            player_name: self.skirmish.player_name.as_str(),
                            country_name: country.as_str(),
                            color_rgb: [0, 160, 0],
                            ai_name: ai_name.as_str(),
                            ai_country: country.as_str(),
                            short_game: self.skirmish.short_game,
                            mcv_repacks: self.skirmish.mcv_repacks,
                            crates: self.skirmish.crates,
                            superweapons: self.skirmish.superweapons,
                            build_off_ally: self.skirmish.build_off_ally,
                            game_speed: self.skirmish.game_speed,
                            credits: self.skirmish.credits,
                            unit_count: self.skirmish.unit_count,
                            chrome: self.skirmish_chrome.as_ref(),
                        };
                        ui_compose::compose_skirmish_lobby_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            self.lobby_preview.as_ref(),
                            &paint,
                            self.menu_panel_anim_frame,
                        )
                    }
                    _ => None,
                };
                if let Some(page) = page {
                    tracing::info!(screen = self.screen.as_str(), w = page.width(), h = page.height(), "壳层 chrome 已合成并上传 UI 页通道");
                    self.upload_ui_page(page);
                    if !self.banner.contains("chrome 已上传") {
                        self.banner = format!("{} · chrome 已上传", self.banner);
                        self.refresh_shell_title();
                    }
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
        self.load_job.is_none()
    }

    /// 惰性解析 `audio.idx` / `audio.bag`，结果缓存在壳层。
    fn ensure_audio_bag(&mut self) {
        if self.audio_bag_tried {
            return;
        }
        self.audio_bag_tried = true;
        self.ensure_menu_assets();
        let idx_bytes = self
            .menu_assets
            .as_ref()
            .and_then(|a| a.source.as_ref())
            .and_then(|s| s.read("audio.idx").ok());
        let bag_bytes = self
            .menu_assets
            .as_ref()
            .and_then(|a| a.source.as_ref())
            .and_then(|s| s.read("audio.bag").ok());
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
        self.menu_assets
            .as_ref()
            .and_then(|a| a.source.as_ref())
            .and_then(|s| s.read(name).ok())
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
            .map(crate::audio::theme_sound_stem)
            .map(str::to_string)
            .filter(|s| !s.is_empty());
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("theme.ini")
            .and_then(|b| crate::audio::soft_ini_get(&b, "INTRO", "Sound"))
            .map(|s| crate::audio::theme_sound_stem(&s).to_string())
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
            .and_then(|b| crate::audio::soft_ini_get(&b, "AudioVisual", "GUIMainButtonSound"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "MenuClick".into())
    }

    /// `sound.ini` 事件 → `Sounds=` 采样名列表。
    fn sound_event_sample_names(&self, event_id: &str) -> Vec<String> {
        let line = self
            .read_ini_doc("sound.ini")
            .as_ref()
            .and_then(|d| d.get(event_id, "Sounds"))
            .map(str::to_string)
            .or_else(|| {
                self.read_asset_bytes("sound.ini")
                    .and_then(|b| crate::audio::soft_ini_get(&b, event_id, "Sounds"))
            })
            .unwrap_or_default();
        line.split_whitespace()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
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
        let theme_note = match self
            .read_asset_bytes("theme.mix")
            .or_else(|| self.read_asset_bytes("Theme.mix"))
        {
            Some(bytes) if bytes.as_slice() == b"CLASS" || bytes.len() < 64 => {
                format!(
                    "theme.mix 为占位（{} 字节），无法读取 {stem}.wav",
                    bytes.len()
                )
            }
            Some(bytes) => format!("theme.mix 可读（{} 字节）但未解出 {stem}.wav/.aud", bytes.len()),
            None => format!("无 theme.mix，且未解出 {stem}.wav/.aud"),
        };
        tracing::warn!(%stem, %theme_note, "菜单主题曲不可用，BGM 静音");
    }

    /// 惰性装载菜单 BGM / 点击采样。
    fn ensure_menu_audio_assets(&mut self) {
        if (self.menu_bgm.is_some() || self.menu_bgm_tried) && self.menu_click.is_some() {
            return;
        }
        self.ensure_menu_assets();
        if self.menu_bgm.is_none() && !self.menu_bgm_tried {
            self.menu_bgm_tried = true;
            let stem = self.menu_theme_sound_stem();
            if let Some(pcm) = self.decode_theme_track(&stem) {
                self.menu_bgm = Some(pcm);
            } else {
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
                crate::audio::synthetic_ui_click()
            }));
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
                | OriginalScreen::Network
        );
        if wants_bgm {
            if !self.menu_bgm_playing {
                if let (Some(audio), Some(bgm)) = (self.audio.as_mut(), self.menu_bgm.as_ref()) {
                    audio.play_music_loop(bgm);
                    self.menu_bgm_playing = true;
                }
            }
        } else if self.menu_bgm_playing {
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

    /// 当前光标下的可点按钮入口（逻辑窗口坐标）。
    fn menu_entry_under_cursor(&self) -> Option<&'static str> {
        if self.screen == OriginalScreen::Campaign {
            return ui_hit::campaign_entry_at(self.cursor.0, self.cursor.1, self.window_width, self.window_height);
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
            OriginalScreen::SkirmishLobby => ui_layout::SKIRMISH_LOBBY_BUTTON_IDS.get(idx).copied(),
            _ => None,
        }
    }

    fn apply_menu_action(&mut self, event_loop: &ActiveEventLoop, action: MenuAction) {
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
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
            }
            MenuAction::ChooseMap => {
                // 完整选图模态未接前：右栏选图先切下一张候选图。
                self.cycle_lobby_map(1);
            }
            MenuAction::UseMap => {
                self.set_screen(OriginalScreen::SkirmishLobby);
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
        }
    }

    /// 进入选项页并快照当前显示档 / 音量草稿。
    fn open_options_page(&mut self) {
        let (music, sound) = self
            .audio
            .as_ref()
            .map(|a| (a.music_volume(), a.sfx_volume()))
            .unwrap_or((0.4, 0.7));
        self.options_volume_baseline = Some((music, sound));
        self.options_pointer_consumed = false;
        self.options_state = Some(crate::options_dialog::OptionsDialogState::from_shell(
            self.display_mode,
            music,
            sound,
        ));
        self.set_screen(OriginalScreen::Options);
    }

    /// 丢弃选项草稿并还原进入页前的音量预览。
    fn discard_options_draft(&mut self) {
        if let Some((music, sound)) = self.options_volume_baseline.take() {
            if let Some(audio) = self.audio.as_mut() {
                audio.set_music_volume(music);
                audio.set_sfx_volume(sound);
            }
        }
        self.options_state = None;
        self.options_pointer_consumed = false;
    }

    /// 接受选项草稿：音量立刻生效并落盘，分辨率变更则改窗。
    fn apply_options_accept(&mut self) {
        let Some(state) = self.options_state.take()
        else {
            self.options_volume_baseline = None;
            self.set_screen(OriginalScreen::MainMenu);
            return;
        };
        self.options_volume_baseline = None;
        self.options_pointer_consumed = false;
        let music = state.music_volume_f32();
        let sound = state.sound_volume_f32();
        self.apply_audio_volumes(music, sound);
        match ra_config::DesktopSettings::persist_audio_volumes(music, sound) {
            Ok(()) => tracing::info!(music, sound, "已写入壳层音量"),
            Err(e) => tracing::warn!(error = %e, "写入壳层音量失败"),
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
                format!(
                    "ra2 · 选图 · {} · Esc 回大厅 · F12 截图",
                    self.selected_map.as_deref().unwrap_or("（未选）")
                )
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
                format!(
                    "ra2 · 选项 · {} · 视频循环分辨率 · Esc 返回 · F12 截图",
                    self.banner
                )
            }
            OriginalScreen::ExitConfirm => {
                format!("ra2 · 确认退出 · {} · Enter 退出 · Esc 取消 · F12 截图", self.banner)
            }
            OriginalScreen::Match | OriginalScreen::Results => unreachable!(),
        };
        window.set_title(&title);
    }

    fn begin_skirmish_load(&mut self) {
        if self.load_job.is_some() {
            tracing::warn!("装载已在进行，忽略重复开始");
            return;
        }
        self.ensure_lobby_maps();
        self.banner =
            format!("正在装载 {} · {}/{}…", self.selected_map.as_deref().unwrap_or("默认候选图"), self.skirmish.side, self.skirmish.difficulty);
        self.pending_after_load = Some(OriginalScreen::Match);
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
        if self.load_job.is_none() && self.screen != OriginalScreen::LoadScreen {
            return;
        }
        self.load_job = None;
        self.load_started = None;
        self.pending_after_load = None;
        self.banner = "已取消装载".into();
        tracing::info!("用户取消遭遇战装载");
        self.set_screen(OriginalScreen::SkirmishLobby);
    }

    fn poll_load_job(&mut self) {
        let Some(job) = self.load_job.as_ref()
        else {
            return;
        };
        match job.try_take() {
            Ok(Some(boot)) => {
                self.load_job = None;
                self.load_started = None;
                self.finish_load(boot);
            }
            Ok(None) => {
                if let Some(t0) = self.load_started {
                    let secs = t0.elapsed().as_secs();
                    let pulse = match secs % 3 {
                        0 => ".",
                        1 => "..",
                        _ => "...",
                    };
                    let stage = self.load_job.as_ref().map(|job| job.progress().stage).unwrap_or_else(|| "装载中".into());
                    self.banner = format!("{stage}{pulse} · {secs}s · Esc/点取消");
                }
            }
            Err(()) => {
                self.load_job = None;
                self.load_started = None;
                self.pending_after_load = None;
                self.banner = "装载线程异常断开 · Enter/点重试 · Esc 回大厅".into();
                tracing::error!("遭遇战装载线程异常断开");
                self.set_screen(OriginalScreen::LoadScreen);
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
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
                    self.set_screen(OriginalScreen::SkirmishLobby);
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
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str());
                self.apply_nav(nav);
            }
        }
        else if self.screen.requires_session() {
            if let Some(ctrl) = self.match_ctrl.as_mut() {
                let _ = ctrl.take_pump_clock();
                self.renderer.timings.simulation = None;
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str());
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
            ) {
                let dt = self
                    .menu_movie_clock
                    .replace(Instant::now())
                    .map(|t0| t0.elapsed().as_secs_f64())
                    .unwrap_or(0.0)
                    .min(0.25);
                let movie_advanced = self.menu_movie.as_mut().is_some_and(|m| m.tick(dt));
                // `sdwrnanm.shp`：右上角 WARNING 屏指示条循环（`sdtp` 只作外壳）。
                const PANEL_FRAME_SECS: f64 = 1.0 / 15.0;
                let panel_dt = self
                    .menu_panel_anim_clock
                    .replace(Instant::now())
                    .map(|t0| t0.elapsed().as_secs_f64())
                    .unwrap_or(0.0)
                    .min(0.25);
                self.menu_panel_anim_accum += panel_dt;
                let mut panel_advanced = false;
                while self.menu_panel_anim_accum >= PANEL_FRAME_SECS {
                    self.menu_panel_anim_accum -= PANEL_FRAME_SECS;
                    self.menu_panel_anim_frame = self.menu_panel_anim_frame.wrapping_add(1);
                    panel_advanced = true;
                }
                if movie_advanced || panel_advanced {
                    self.refresh_menu_backdrop();
                } else if let Some(reason) = self.menu_movie.as_ref().and_then(|m| m.stalled_reason()) {
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
            self.splash_started = Some(Instant::now());
            self.upload_splash_backdrop();
            self.splash_uploaded = true;
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
                    } else if self.screen == OriginalScreen::SkirmishLobby && self.handle_skirmish_drag() {
                        // 遭遇战滑条拖动已刷新。
                    } else if self.screen == OriginalScreen::Campaign && self.handle_campaign_drag() {
                        // 战役难度滑条拖动已刷新。
                    } else if matches!(
                        self.screen,
                        OriginalScreen::MainMenu
                            | OriginalScreen::SinglePlayerMenu
                            | OriginalScreen::Campaign
                            | OriginalScreen::Options
                            | OriginalScreen::ExitConfirm
                            | OriginalScreen::SkirmishLobby
                    ) {
                        let next = self.menu_entry_under_cursor();
                        if next != self.menu_hovered_entry {
                            self.menu_hovered_entry = next;
                            self.refresh_menu_backdrop();
                        }
                    }
                }
                WindowEvent::MouseInput { state, button: winit::event::MouseButton::Left, .. } => match state {
                    ElementState::Pressed => {
                        if self.screen == OriginalScreen::Splash {
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
                        ) {
                            let next = self.menu_entry_under_cursor();
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
                        if self.screen == OriginalScreen::Splash {
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
                            } else if let Some(action) = ui_hit::hit_action(
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
                            } else if let Some(action) = ui_hit::hit_action(
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
                            } else if let Some(action) = ui_hit::hit_action(
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
                        self.handle_pre_game_key(event_loop, key_ev.physical_key);
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
fn campaign_difficulty_from_track_x(track: ui_layout::RectPx, mouse_x: i32) -> u8 {
    let travel = (track.w - 12).max(1);
    let rel = (mouse_x - track.x - 6).clamp(0, travel);
    ((rel * 2 + travel / 2) / travel).clamp(0, 2) as u8
}

/// 解析启动参数并进入事件循环。
pub fn run_shell() -> RaResult<()> {
    let (mode, display_mode, music_volume, sound_volume, present, status_path, test_scene) = resolve_launch()?;

    let event_loop = EventLoop::new().map_err(|e| RaError::Msg(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = match mode {
        #[cfg(feature = "test-harness")]
        LaunchMode::DirectMatch(boot) => {
            if let Some(game) = boot.session.as_ref().and_then(|s| s.game()) {
                tracing::info!("preview_origin=({}, {}) entities={}", game.preview_origin_x, game.preview_origin_y, game.world.entities.len());
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

    event_loop.run_app(&mut app).map_err(|e| RaError::Msg(e.to_string()))?;
    tracing::info!("事件循环结束");
    Ok(())
}

enum LaunchMode {
    #[cfg(feature = "test-harness")]
    DirectMatch(BootResult),
    MainMenu,
}

fn resolve_launch() -> RaResult<(LaunchMode, DisplayMode, f32, f32, PresentFeel, Option<PathBuf>, Option<String>)> {
    #[cfg(feature = "test-harness")]
    {
        if let Some(scene) = crate::test_boot::requested_scene() {
            let status_path = crate::test_boot::status_path();
            let window_width = crate::test_boot::TEST_WINDOW_WIDTH;
            let window_height = crate::test_boot::TEST_WINDOW_HEIGHT;
            tracing::info!(
                "test-harness scene={scene} window={}x{} status={}",
                window_width,
                window_height,
                status_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "—".into())
            );
            let t = crate::test_boot::boot_scene(&scene)?;
            tracing::info!("boot: {} · session=ok", t.note);
            return Ok((
                LaunchMode::DirectMatch(BootResult { note: t.note, engine: Some(t.engine), session: Some(t.session), preview: t.preview }),
                DisplayMode::DEFAULT,
                0.4,
                0.7,
                PresentFeel::DEFAULT,
                status_path,
                Some(scene),
            ));
        }
    }

    // 产品路径：主菜单起；对局须手动经菜单进入（自动测试用 DirectMatch 场景）。
    let (settings, diagnostics) = crate::config::load_desktop_config_with_diagnostics();
    for d in &diagnostics {
        tracing::info!(source = %d.source, "{}", d.message);
    }
    let display_mode = settings.display_mode;
    tracing::info!(
        display_mode = display_mode.as_str(),
        music_volume = settings.music_volume,
        sound_volume = settings.sound_volume,
        present_mode = settings.present.mode.as_str(),
        present_gamma = settings.present.gamma,
        ra2_dir = %settings.ra2_dir.display(),
        "desktop launch settings"
    );
    Ok((
        LaunchMode::MainMenu,
        display_mode,
        settings.music_volume,
        settings.sound_volume,
        settings.present,
        None,
        None,
    ))
}

#[cfg(test)]
mod campaign_track_tests {
    use super::campaign_difficulty_from_track_x;
    use crate::ui_layout::campaign_layout;

    #[test]
    fn difficulty_track_maps_left_mid_right() {
        let track = campaign_layout(800, 600).difficulty_track;
        assert_eq!(campaign_difficulty_from_track_x(track, track.x + 2), 0);
        assert_eq!(campaign_difficulty_from_track_x(track, track.x + track.w / 2), 1);
        assert_eq!(campaign_difficulty_from_track_x(track, track.x + track.w - 2), 2);
    }
}
