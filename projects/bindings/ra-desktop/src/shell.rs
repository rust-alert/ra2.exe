//! 单窗口应用外壳：页面导航、窗口生命周期；对局逻辑委托 `MatchController`。

use std::{path::PathBuf, sync::Arc, time::Instant};

use ra_assets::{AudioIndex, CsfFile, FntFile, PcmAudio, decode_wav_pcm};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::{AssetSource, DisplayMode, RaError, RaResult};
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
    skirmish_setup::SkirmishBootRequest,
    ui_assets::{MenuUiProbe, probe_menu_ui_assets},
    ui_compose, ui_decode, ui_hit, ui_layout,
    ui_movie::MenuMoviePlayer,
    ui_page::page_resources_from_slots,
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
    /// 主菜单阶段 UI 资源探测（惰性一次）。
    ui_probe: Option<MenuUiProbe>,
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
    /// 闪屏 PCX 已上传（避免每帧重解）。
    splash_uploaded: bool,
    /// 下一帧回读后落盘的截图短名（`OriginalScreen::as_str`）；F12 手动截图用。
    pending_screenshot: Option<&'static str>,
    /// 自动关键页截图去重（仅 `test-harness`）。
    #[cfg(feature = "test-harness")]
    auto_screenshots: crate::screenshot::AutoScreenshotTracker,
    /// 遭遇战大厅阵营 / 难度（进入装载请求）。
    skirmish: SkirmishBootRequest,
    /// 桌面音频输出（设备不可用则为 `None`）。
    audio: Option<crate::audio::ShellAudio>,
    /// 主菜单 BGM PCM（`intro.wav`）。
    menu_bgm: Option<PcmAudio>,
    /// 菜单点击音效 PCM。
    menu_click: Option<PcmAudio>,
    /// 当前是否已在播壳层 BGM。
    menu_bgm_playing: bool,
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
            ui_probe: None,
            ui_decode_cache: None,
            menu_pressed_entry: None,
            menu_hovered_entry: None,
            menu_font: None,
            menu_csf: None,
            menu_movie: None,
            menu_movie_clock: None,
            splash_uploaded: false,
            pending_screenshot: None,
            #[cfg(feature = "test-harness")]
            auto_screenshots: crate::screenshot::AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
            audio: crate::audio::ShellAudio::try_open(),
            menu_bgm: None,
            menu_click: None,
            menu_bgm_playing: false,
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
            ui_probe: None,
            ui_decode_cache: None,
            menu_pressed_entry: None,
            menu_hovered_entry: None,
            menu_font: None,
            menu_csf: None,
            menu_movie: None,
            menu_movie_clock: None,
            splash_uploaded: false,
            pending_screenshot: None,
            #[cfg(feature = "test-harness")]
            auto_screenshots: crate::screenshot::AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
            audio: crate::audio::ShellAudio::try_open(),
            menu_bgm: None,
            menu_click: None,
            menu_bgm_playing: false,
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
        if !self.splash_preload_done {
            self.ensure_ui_probe();
            self.ensure_menu_text_assets();
            self.ensure_menu_audio_assets();
            // 预热主菜单 chrome（不切入主菜单、不推进影片）。
            let prev = self.screen;
            self.screen = OriginalScreen::MainMenu;
            self.refresh_ui_resolve_note();
            self.screen = prev;
            self.menu_movie = None;
            self.menu_movie_clock = None;
            self.splash_preload_done = true;
            self.banner = "闪屏 · 预处理完成".into();
            self.refresh_shell_title();
        }
        let elapsed = self.splash_started.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
        let min_ok = elapsed >= self.splash_min_secs;
        if self.splash_preload_done && (min_ok || self.splash_skip) {
            tracing::info!(elapsed, min = self.splash_min_secs, skip = self.splash_skip, "闪屏结束 → 主菜单");
            self.set_screen(OriginalScreen::MainMenu);
        }
        else {
            if !self.splash_uploaded {
                self.upload_splash_backdrop();
                self.splash_uploaded = true;
            }
        }
    }

    /// 闪屏画面：解码槽位中的 `title.pcx`（失败则黑底占位）。
    fn upload_splash_backdrop(&mut self) {
        self.ensure_ui_probe();
        let pcx_name = ui_slots::slots_for(OriginalScreen::Splash)
            .and_then(|s| s.background_pcx)
            .unwrap_or("title.pcx");
        let decoded = self
            .ui_probe
            .as_ref()
            .and_then(|p| p.source.as_ref())
            .and_then(|src| match src.read(pcx_name) {
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
                },
            });
        if let Some(page) = decoded {
            tracing::info!(name = pcx_name, w = page.width(), h = page.height(), "闪屏 PCX 已上传");
            if !self.banner.contains("title.pcx") {
                self.banner = format!("{} · {pcx_name} {}×{}", self.banner, page.width(), page.height());
                self.refresh_shell_title();
            }
            self.renderer.clear_preview();
            self.renderer.set_ui_page(page);
            return;
        }
        let w = ui_layout::SHELL_BASE_W as u32;
        let h = ui_layout::SHELL_BASE_H as u32;
        let pixels = vec![0u8; (w as usize) * (h as usize) * 4];
        if let Some(page) = RgbaImage::from_raw(w, h, pixels) {
            self.renderer.clear_preview();
            self.renderer.set_ui_page(page);
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

    fn ensure_ui_probe(&mut self) {
        if self.ui_probe.is_some() {
            return;
        }
        let probe = probe_menu_ui_assets();
        tracing::info!(
            ui_ini = ?probe.ui_ini_name,
            ui_ini_ok = probe.ui_ini_readable,
            ui_sections = probe.ui_ini.as_ref().map(|d| d.sections.len()),
            ui_shp_refs = probe.ui_ini_shp_refs.len(),
            has_source = probe.source.is_some(),
            "{}",
            probe.note
        );
        self.banner = probe.note.clone();
        self.ui_probe = Some(probe);
        self.refresh_ui_resolve_note();
    }

    /// 对当前页已声明资源名做可读性探测，并尝试解码 chrome（不绘制）。
    fn refresh_ui_resolve_note(&mut self) {
        let Some(probe) = self.ui_probe.as_ref()
        else {
            return;
        };
        let Some(source) = probe.source.as_ref()
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
            if probe.note.contains("槽位未填") { probe.note.clone() } else { format!("{} · {}", probe.note, report.banner_note()) }
        }
        else {
            format!("{} · {}", probe.note, report.banner_note())
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
                OriginalScreen::MainMenu | OriginalScreen::SinglePlayerMenu | OriginalScreen::Options
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
        let font_bytes = self.ui_probe.as_ref().and_then(|p| p.source.as_ref()).and_then(|s| s.read("game.fnt").ok());
        let csf_bytes = self.ui_probe.as_ref().and_then(|p| p.source.as_ref()).and_then(|s| {
            for name in ["ra2.csf", "ra2md.csf"] {
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

    /// 前置页：主菜单 / 单人 / 选项 / 遭遇战大厅上传合成 chrome；其余清空 UI/预览。
    fn refresh_menu_backdrop(&mut self) {
        if matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
            self.renderer.clear_ui_page();
            return;
        }
        self.ensure_ui_probe();
        self.ensure_menu_text_assets();
        if matches!(
            self.screen,
            OriginalScreen::MainMenu | OriginalScreen::SinglePlayerMenu | OriginalScreen::Options | OriginalScreen::SkirmishLobby
        ) {
            if self.screen == OriginalScreen::SkirmishLobby {
                self.ensure_lobby_maps();
                self.ensure_lobby_preview();
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
                    ),
                    OriginalScreen::Options => ui_compose::compose_options_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        movie,
                    ),
                    OriginalScreen::SkirmishLobby => {
                        let selected = self.selected_map.as_deref();
                        let map_names: Vec<(String, bool)> = self
                            .lobby_maps
                            .iter()
                            .take(ui_layout::LOBBY_MAP_ROW_MAX as usize)
                            .map(|m| (m.file_name.clone(), selected == Some(m.file_name.as_str())))
                            .collect();
                        ui_compose::compose_skirmish_lobby_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            self.lobby_preview.as_ref(),
                            &map_names,
                        )
                    }
                    _ => None,
                };
                if let Some(page) = page {
                    tracing::info!(screen = self.screen.as_str(), w = page.width(), h = page.height(), "壳层 chrome 已合成并上传 UI 页通道");
                    self.renderer.set_ui_page(page);
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

    /// 惰性装载菜单 BGM / 点击采样。
    fn ensure_menu_audio_assets(&mut self) {
        if self.menu_bgm.is_some() && self.menu_click.is_some() {
            return;
        }
        self.ensure_ui_probe();
        if self.menu_bgm.is_none() {
            let bytes = self
                .ui_probe
                .as_ref()
                .and_then(|p| p.source.as_ref())
                .and_then(|s| s.read("intro.wav").ok());
            if let Some(bytes) = bytes {
                match decode_wav_pcm(&bytes) {
                    Ok(pcm) => {
                        tracing::info!(
                            frames = pcm.samples.len(),
                            rate = pcm.sample_rate,
                            "已加载菜单 BGM · intro.wav"
                        );
                        self.menu_bgm = Some(pcm);
                    }
                    Err(e) => tracing::warn!(error = %e, "intro.wav 解码失败"),
                }
            } else {
                tracing::warn!("intro.wav 不可读");
            }
        }
        if self.menu_click.is_none() {
            let mut loaded = None;
            // 优先 audio.bag（主按钮音效常在此）。
            let idx_bytes = self
                .ui_probe
                .as_ref()
                .and_then(|p| p.source.as_ref())
                .and_then(|s| s.read("audio.idx").ok());
            let bag_bytes = self
                .ui_probe
                .as_ref()
                .and_then(|p| p.source.as_ref())
                .and_then(|s| s.read("audio.bag").ok());
            if let (Some(idx), Some(bag)) = (idx_bytes, bag_bytes) {
                if let Some(index) = AudioIndex::parse(&idx, bag) {
                    for name in ["GUIMainButtonSound", "GUIMAINBUTTONSO", "BUTTON"] {
                        if let Some(pcm) = index.decode(name) {
                            tracing::info!(%name, frames = pcm.samples.len(), "已从 audio.bag 加载点击音效");
                            loaded = Some(pcm);
                            break;
                        }
                    }
                    if loaded.is_none() {
                        tracing::debug!(
                            entries = index.len(),
                            "audio.bag 已解析但未命中主按钮音效名"
                        );
                    }
                } else {
                    tracing::warn!("audio.idx 解析失败");
                }
            }
            if loaded.is_none() {
                for name in ["guimainbuttonsound.wav", "button.wav", "click.wav"] {
                    let bytes = self
                        .ui_probe
                        .as_ref()
                        .and_then(|p| p.source.as_ref())
                        .and_then(|s| s.read(name).ok());
                    let Some(bytes) = bytes
                    else {
                        continue;
                    };
                    match decode_wav_pcm(&bytes) {
                        Ok(pcm) => {
                            tracing::info!(%name, "已加载菜单点击 WAV");
                            loaded = Some(pcm);
                            break;
                        }
                        Err(e) => tracing::debug!(%name, error = %e, "点击音 WAV 解码失败"),
                    }
                }
            }
            self.menu_click = Some(loaded.unwrap_or_else(|| {
                tracing::info!("使用合成点击音效占位");
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
                | OriginalScreen::Options
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
            OriginalScreen::SkirmishLobby => {
                let map_n = self.lobby_maps.len().min(ui_layout::LOBBY_MAP_ROW_MAX as usize);
                idx.checked_sub(map_n).and_then(|i| ui_layout::SKIRMISH_LOBBY_BUTTON_IDS.get(i).copied())
            }
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
            MenuAction::OpenOptions => self.set_screen(OriginalScreen::Options),
            MenuAction::Exit => event_loop.exit(),
            MenuAction::OpenSkirmish => {
                self.ensure_lobby_maps();
                self.set_screen(OriginalScreen::SkirmishLobby);
            }
            MenuAction::Back => match self.screen {
                OriginalScreen::SinglePlayerMenu | OriginalScreen::Network | OriginalScreen::Options => {
                    self.set_screen(OriginalScreen::MainMenu);
                }
                OriginalScreen::SkirmishLobby => self.set_screen(OriginalScreen::SinglePlayerMenu),
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
            MenuAction::CycleDisplayMode => self.cycle_display_mode(),
            MenuAction::SelectMap(i) => {
                if let Some(map) = self.lobby_maps.get(i) {
                    self.selected_map = Some(map.file_name.clone());
                    self.skirmish.preferred_map = Some(map.file_name.clone());
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
            }
        }
    }

    /// 循环离散分辨率：改窗口客户区、落盘配置、刷新 chrome。
    fn cycle_display_mode(&mut self) {
        self.display_mode = self.display_mode.cycle_next();
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
                format!("ra2 · 主菜单 · {} · Enter 单人 · N 网络 · O 选项 · Esc 退出 · F12 截图", self.banner)
            }
            OriginalScreen::SinglePlayerMenu => "ra2 · 单人游戏 · Enter/S 遭遇战 · Esc 返回 · F12 截图".into(),
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
                PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                _ => {}
            },
            OriginalScreen::SinglePlayerMenu => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) | PhysicalKey::Code(KeyCode::KeyS) => {
                    self.set_screen(OriginalScreen::SkirmishLobby);
                }
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::MainMenu),
                _ => {}
            },
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
            OriginalScreen::Network | OriginalScreen::Options => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.set_screen(OriginalScreen::MainMenu);
                }
            }
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
            if matches!(self.screen, OriginalScreen::MainMenu | OriginalScreen::SinglePlayerMenu) {
                let dt = self.menu_movie_clock.replace(Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0);
                let advanced = self.menu_movie.as_mut().is_some_and(|m| m.tick(dt.min(0.25)));
                if advanced {
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
                        .with_resizable(false),
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
        self.window = Some(window);
        if self.screen == OriginalScreen::Splash {
            self.splash_started = Some(Instant::now());
            self.upload_splash_backdrop();
        }
        else {
            self.refresh_menu_backdrop();
        }
        self.refresh_shell_title();
        #[cfg(feature = "test-harness")]
        if self.auto_screenshots.should_capture(self.screen) {
            self.queue_screenshot(self.screen.as_str());
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
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
            | OriginalScreen::SkirmishLobby
            | OriginalScreen::Network
            | OriginalScreen::Options
            | OriginalScreen::LoadScreen => match &event {
                WindowEvent::CursorMoved { position, .. } => {
                    // 与 window_width/height 同用逻辑像素，避免 HiDPI 下物理光标打偏命中框。
                    let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                    let logical = position.to_logical::<f64>(scale);
                    self.cursor = (logical.x, logical.y);
                    if matches!(
                        self.screen,
                        OriginalScreen::MainMenu
                            | OriginalScreen::SinglePlayerMenu
                            | OriginalScreen::Options
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
                        else if matches!(
                            self.screen,
                            OriginalScreen::MainMenu
                                | OriginalScreen::SinglePlayerMenu
                                | OriginalScreen::Options
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

/// 解析启动参数并进入事件循环。
pub fn run_shell() -> RaResult<()> {
    let (mode, display_mode, music_volume, sound_volume, status_path, test_scene) = resolve_launch()?;

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

    event_loop.run_app(&mut app).map_err(|e| RaError::Msg(e.to_string()))?;
    tracing::info!("事件循环结束");
    Ok(())
}

enum LaunchMode {
    #[cfg(feature = "test-harness")]
    DirectMatch(BootResult),
    MainMenu,
}

fn resolve_launch() -> RaResult<(LaunchMode, DisplayMode, f32, f32, Option<PathBuf>, Option<String>)> {
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
        ra2_dir = %settings.ra2_dir.display(),
        "desktop launch settings"
    );
    Ok((
        LaunchMode::MainMenu,
        display_mode,
        settings.music_volume,
        settings.sound_volume,
        None,
        None,
    ))
}
