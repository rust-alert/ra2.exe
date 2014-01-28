//! 单窗口应用外壳：页面导航、窗口生命周期；对局逻辑委托 `MatchController`。

use std::{path::PathBuf, sync::Arc, time::Instant};

use ra_renderer::{Renderer, RgbaImage};
use ra_types::{RaError, RaResult};
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
    menu_view::{MenuAction, MenuLayout, layout_for, layout_load_screen, layout_skirmish_lobby},
    preview_job::PreviewJob,
    screen::OriginalScreen,
    screenshot::AutoScreenshotTracker,
    skirmish_setup::SkirmishBootRequest,
    ui_assets::{
        MenuUiProbe, probe_menu_ui_assets, stamp_bottom_right_opaque, stamp_bottom_right_pending,
        stamp_norm_progress_bar, stamp_top_left, stamp_top_right,
    },
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
    status_path: Option<PathBuf>,
    test_scene: Option<String>,
    /// 首次 `resumed` 后自动开一局遭遇战（第一阶段便利；正式配置 UI 到位后可关）。
    auto_start_skirmish: bool,
    /// 装载完成后待切到的目标页。
    pending_after_load: Option<OriginalScreen>,
    /// 当前前置页占位菜单（对局页为 `None`）。
    menu: Option<MenuLayout>,
    /// 光标位置（菜单命中用）。
    cursor: (f64, f64),
    /// 当前悬停的可点命中区下标。
    menu_hover: Option<usize>,
    /// 左键按下时命中的下标（按下态）。
    menu_pressed: Option<usize>,
    /// 后台遭遇战装载（`LoadScreen` 期间轮询）。
    load_job: Option<LoadJob>,
    /// 当前装载开始时刻（脉搏标题用）。
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
    /// 下一帧回读后落盘的截图短名（`OriginalScreen::as_str`）。
    pending_screenshot: Option<&'static str>,
    /// 自动关键页截图去重。
    auto_screenshots: AutoScreenshotTracker,
    /// 遭遇战大厅阵营 / 难度（进入装载请求）。
    skirmish: SkirmishBootRequest,
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
            renderer.set_preview(image.clone());
        }
        let ctrl = MatchController::from_boot(boot, status_path.clone(), test_scene.clone());
        let screen = if ctrl.has_session() {
            OriginalScreen::Match
        }
        else {
            OriginalScreen::MainMenu
        };
        Self {
            window: None,
            screen,
            match_ctrl: Some(ctrl),
            renderer,
            banner: String::new(),
            window_width,
            window_height,
            status_path,
            test_scene,
            auto_start_skirmish: false,
            pending_after_load: None,
            menu: None,
            cursor: (0.0, 0.0),
            menu_hover: None,
            menu_pressed: None,
            load_job: None,
            load_started: None,
            lobby_maps: Vec::new(),
            selected_map: None,
            lobby_preview_for: None,
            lobby_preview: None,
            lobby_preview_job: None,
            ui_probe: None,
            pending_screenshot: None,
            auto_screenshots: AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
        }
    }

    /// 正常产品路径：主菜单起，可自动开遭遇战。
    pub fn with_main_menu(window_width: f64, window_height: f64, auto_start_skirmish: bool) -> Self {
        Self {
            window: None,
            screen: OriginalScreen::MainMenu,
            match_ctrl: None,
            renderer: Renderer::new(),
            banner: "占位色块菜单 · 非 Pre-Alpha 原版 UI".into(),
            window_width,
            window_height,
            status_path: None,
            test_scene: None,
            auto_start_skirmish,
            pending_after_load: None,
            menu: None,
            cursor: (0.0, 0.0),
            menu_hover: None,
            menu_pressed: None,
            load_job: None,
            load_started: None,
            lobby_maps: Vec::new(),
            selected_map: None,
            lobby_preview_for: None,
            lobby_preview: None,
            lobby_preview_job: None,
            ui_probe: None,
            pending_screenshot: None,
            auto_screenshots: AutoScreenshotTracker::default(),
            skirmish: SkirmishBootRequest::default_lobby(),
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
                            w = thumb.width,
                            h = thumb.height,
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
        let cur = self
            .selected_map
            .as_ref()
            .and_then(|name| self.lobby_maps.iter().position(|m| &m.file_name == name))
            .unwrap_or(0);
        let n = self.lobby_maps.len() as isize;
        let next = ((cur as isize + delta).rem_euclid(n)) as usize;
        self.selected_map = Some(self.lobby_maps[next].file_name.clone());
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
    }

    fn set_screen(&mut self, next: OriginalScreen) {
        if self.screen != next {
            tracing::info!("页面 {} → {}", self.screen.as_str(), next.as_str());
            self.screen = next;
            self.menu_hover = None;
            self.menu_pressed = None;
            self.refresh_menu_backdrop();
            self.refresh_shell_title();
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
        let Some(image) = self.renderer.take_capture()
        else {
            tracing::warn!("截图回读为空 · screen={name}");
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

    fn refresh_menu_backdrop(&mut self) {
        // 离开对局后清掉屏上 HUD 色块，避免叠在菜单底图上。
        self.renderer.set_screen_chrome(&[]);
        let w = self.window_width.max(1.0) as u32;
        let h = self.window_height.max(1.0) as u32;
        if self.screen == OriginalScreen::SkirmishLobby {
            self.ensure_lobby_maps();
            let mut layout = layout_skirmish_lobby(
                w,
                h,
                &self.lobby_maps,
                self.selected_map.as_deref(),
                self.skirmish.side.as_str(),
                self.skirmish.difficulty.as_str(),
                self.menu_hover,
                self.menu_pressed,
            );
            self.ensure_lobby_preview();
            let selected = self.selected_map.as_deref();
            let preview_ready =
                self.lobby_preview.is_some() && self.lobby_preview_for.as_deref() == selected;
            if preview_ready {
                if let Some(preview) = self.lobby_preview.as_ref() {
                    stamp_bottom_right_opaque(&mut layout.image, preview, 12);
                }
            }
            else if self.lobby_preview_job.is_some() {
                let pulse = (Instant::now().elapsed().as_millis() / 250) as u8;
                stamp_bottom_right_pending(&mut layout.image, 320, 200, 12, pulse);
            }
            else if let Some(preview) = self.lobby_preview.as_ref() {
                stamp_bottom_right_opaque(&mut layout.image, preview, 12);
            }
            self.ensure_ui_probe();
            if let Some(probe) = self.ui_probe.as_ref() {
                if let Some(frame) = probe.mouse_frame.as_ref() {
                    stamp_top_right(&mut layout.image, frame, 16);
                }
                if let Some(clock) = probe.clock_frame.as_ref() {
                    stamp_top_left(&mut layout.image, clock, 16);
                }
            }
            self.renderer.set_preview(layout.image.clone());
            self.menu = Some(layout);
            return;
        }
        if let Some(mut layout) = if self.screen == OriginalScreen::LoadScreen {
            Some(layout_load_screen(
                w,
                h,
                self.menu_hover,
                self.menu_pressed,
                self.load_job.is_none(),
            ))
        }
        else {
            layout_for(self.screen, w, h, self.menu_hover, self.menu_pressed)
        } {
            self.ensure_ui_probe();
            if let Some(probe) = self.ui_probe.as_ref() {
                if let Some(frame) = probe.mouse_frame.as_ref() {
                    stamp_top_right(&mut layout.image, frame, 16);
                }
                if let Some(clock) = probe.clock_frame.as_ref() {
                    stamp_top_left(&mut layout.image, clock, 16);
                }
            }
            if self.screen == OriginalScreen::LoadScreen {
                // 进度条落在 loading 槽位内，与 ui_slots 命中框对齐。
                let ratio = self
                    .load_job
                    .as_ref()
                    .map(|job| job.progress().ratio.clamp(0.05, 1.0))
                    .unwrap_or(0.05);
                stamp_norm_progress_bar(
                    &mut layout.image,
                    0.32,
                    0.42,
                    0.72,
                    0.46,
                    ratio,
                    [28, 32, 48, 255],
                    [220, 180, 64, 255],
                );
            }
            self.renderer.set_preview(layout.image.clone());
            self.menu = Some(layout);
        }
        else {
            self.menu = None;
        }
    }

    fn update_menu_hover(&mut self) {
        let next = self.menu.as_ref().and_then(|m| {
            m.hover_index(self.cursor.0, self.cursor.1, self.window_width, self.window_height)
        });
        if next != self.menu_hover {
            self.menu_hover = next;
            self.refresh_menu_backdrop();
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

    fn refresh_shell_title(&mut self) {
        let Some(window) = &self.window
        else {
            return;
        };
        if matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
            return;
        }
        let title = match self.screen {
            OriginalScreen::MainMenu => format!("ra2 · 主菜单 · {} · F12 截图", self.banner),
            OriginalScreen::SinglePlayerMenu => "ra2 · 单人游戏 · 遭遇战 Enter · Esc 返回 · F12 截图".into(),
            OriginalScreen::SkirmishLobby => {
                let detail = self
                    .selected_map
                    .as_ref()
                    .and_then(|name| self.lobby_maps.iter().find(|m| &m.file_name == name))
                    .map(|m| {
                        format!(
                            "{} {}x{} {}",
                            m.file_name,
                            m.width,
                            m.height,
                            m.theater.as_str()
                        )
                    })
                    .unwrap_or_else(|| "（无可用图）".into());
                format!(
                    "ra2 · 遭遇战大厅 · {detail} · {}/{} · ←/→ 图 · Q阵营 E难度 · Enter 开始 · Esc 返回 · F12 截图",
                    self.skirmish.side, self.skirmish.difficulty
                )
            }
            OriginalScreen::Network => "ra2 · 网络（占位禁用）· Esc 返回 · F12 截图".into(),
            OriginalScreen::LoadScreen => {
                if self.load_job.is_some() {
                    format!("ra2 · 加载 · {} · Esc/点取消 · F12 截图", self.banner)
                }
                else {
                    format!(
                        "ra2 · 加载 · {} · Enter/点重试 · Esc 回大厅 · F12 截图",
                        self.banner
                    )
                }
            }
            OriginalScreen::Options => "ra2 · 选项（音频/视频占位禁用）· Esc 返回 · F12 截图".into(),
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
        self.banner = format!(
            "正在装载 {} · {}/{}…",
            self.selected_map.as_deref().unwrap_or("默认候选图"),
            self.skirmish.side,
            self.skirmish.difficulty
        );
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
                    let stage = self
                        .load_job
                        .as_ref()
                        .map(|job| job.progress().stage)
                        .unwrap_or_else(|| "装载中".into());
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
            self.renderer.set_preview(preview.clone());
        }
        match self.match_ctrl.as_mut() {
            Some(ctrl) => ctrl.apply_boot(boot, &mut self.renderer),
            None => {
                self.match_ctrl = Some(MatchController::from_boot(
                    boot,
                    self.status_path.clone(),
                    self.test_scene.clone(),
                ));
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
                self.banner = format!(
                    "已返回大厅 · 地图 {}",
                    self.selected_map.as_deref().unwrap_or("（未选）")
                );
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
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
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
            // 主菜单等前置页：占位色块底图 + 标题。原版 SHP 资产未接前不冒充交付完成。
            self.renderer.timings.simulation = None;
            self.renderer.timings.presentation_build = None;
            if self.screen == OriginalScreen::LoadScreen {
                self.poll_load_job();
                // 装载中刷新占位底图，让 loading 条有可见脉动（非原版进度条）。
                if self.load_job.is_some() {
                    self.refresh_menu_backdrop();
                }
            }
            if self.screen == OriginalScreen::SkirmishLobby {
                let ready = self.poll_lobby_preview();
                let pending = self.lobby_preview_job.is_some();
                if ready || pending {
                    self.refresh_menu_backdrop();
                }
                if ready {
                    let map = self.selected_map.as_deref().unwrap_or("?");
                    self.banner = format!("预览就绪 · {map}");
                }
                else if pending {
                    let map = self.selected_map.as_deref().unwrap_or("?");
                    self.banner = format!("预览生成中… {map}");
                }
            }
            if self.menu.is_none() {
                self.refresh_menu_backdrop();
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
                        .with_inner_size(winit::dpi::LogicalSize::new(self.window_width, self.window_height)),
                )
                .expect("创建窗口失败"),
        );
        if let Err(e) = self.renderer.attach_window(window.clone()) {
            tracing::error!("wgpu 附着失败: {e}");
        }
        else {
            tracing::info!(
                "gpu={} preview={} zoom={:.2} screen={}",
                self.renderer.backend_name(),
                if self.renderer.has_preview() { "yes" } else { "no" },
                self.renderer.camera().zoom,
                self.screen.as_str()
            );
        }
        self.window = Some(window);
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
        if self.auto_screenshots.should_capture(self.screen) {
            self.queue_screenshot(self.screen.as_str());
        }
        if self.auto_start_skirmish {
            self.auto_start_skirmish = false;
            self.begin_skirmish_load();
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
                // 命中框与菜单底图按逻辑窗口尺寸算；物理缓冲变化后必须同步，否则缩放后点击错位。
                let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                let logical = size.to_logical::<f64>(scale);
                self.window_width = logical.width.max(1.0);
                self.window_height = logical.height.max(1.0);
                if !matches!(self.screen, OriginalScreen::Match | OriginalScreen::Results) {
                    self.refresh_menu_backdrop();
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                return;
            }
            WindowEvent::KeyboardInput { event: key_ev, .. }
                if key_ev.state == ElementState::Pressed
                    && matches!(key_ev.physical_key, PhysicalKey::Code(KeyCode::F12)) =>
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
            OriginalScreen::MainMenu
            | OriginalScreen::SinglePlayerMenu
            | OriginalScreen::SkirmishLobby
            | OriginalScreen::Network
            | OriginalScreen::Options
            | OriginalScreen::LoadScreen => {
                match &event {
                    WindowEvent::CursorMoved { position, .. } => {
                        self.cursor = (position.x, position.y);
                        self.update_menu_hover();
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        let idx = self.menu.as_ref().and_then(|m| {
                            m.hover_index(self.cursor.0, self.cursor.1, self.window_width, self.window_height)
                        });
                        if idx != self.menu_pressed {
                            self.menu_pressed = idx;
                            self.refresh_menu_backdrop();
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        if self.menu_pressed.is_some() {
                            self.menu_pressed = None;
                            self.refresh_menu_backdrop();
                        }
                        let clicked = self.menu.as_ref().and_then(|menu| {
                            let action =
                                menu.hit(self.cursor.0, self.cursor.1, self.window_width, self.window_height)?;
                            let entry = menu
                                .hover_index(self.cursor.0, self.cursor.1, self.window_width, self.window_height)
                                .and_then(|i| menu.hits.get(i).map(|h| h.entry_id));
                            Some((action, entry))
                        });
                        if let Some((action, entry)) = clicked {
                            tracing::debug!(?entry, ?action, "菜单点击");
                            self.apply_menu_action(event_loop, action);
                        }
                    }
                    WindowEvent::KeyboardInput { event: key_ev, .. } => {
                        if key_ev.state == ElementState::Pressed {
                            self.handle_pre_game_key(event_loop, key_ev.physical_key);
                        }
                    }
                    _ => {}
                }
            }
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
    let (mode, window_width, window_height, status_path, test_scene) = resolve_launch()?;

    let event_loop = EventLoop::new().map_err(|e| RaError::Msg(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = match mode {
        #[cfg(feature = "test-harness")]
        LaunchMode::DirectMatch(boot) => {
            if let Some(game) = boot.session.as_ref().and_then(|s| s.game()) {
                tracing::info!(
                    "preview_origin=({}, {}) entities={}",
                    game.preview_origin_x,
                    game.preview_origin_y,
                    game.world.entities.len()
                );
            }
            AppShell::with_match(boot, window_width, window_height, status_path, test_scene)
        }
        LaunchMode::MainMenu { auto_start } => {
            let _ = (status_path, test_scene);
            AppShell::with_main_menu(window_width, window_height, auto_start)
        }
    };

    event_loop.run_app(&mut app).map_err(|e| RaError::Msg(e.to_string()))?;
    tracing::info!("事件循环结束");
    Ok(())
}

enum LaunchMode {
    #[cfg(feature = "test-harness")]
    DirectMatch(BootResult),
    MainMenu { auto_start: bool },
}

fn resolve_launch() -> RaResult<(LaunchMode, f64, f64, Option<PathBuf>, Option<String>)> {
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
                LaunchMode::DirectMatch(BootResult {
                    note: t.note,
                    engine: Some(t.engine),
                    session: Some(t.session),
                    preview: t.preview,
                }),
                window_width,
                window_height,
                status_path,
                Some(scene),
            ));
        }
    }

    // 产品路径：主菜单起，禁止自动开局（原版主 UI 复刻验收要求）。
    Ok((LaunchMode::MainMenu { auto_start: false }, 1024.0, 768.0, None, None))
}
