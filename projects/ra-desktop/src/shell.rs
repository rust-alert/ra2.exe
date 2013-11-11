//! 单窗口应用外壳：页面导航、窗口生命周期；对局逻辑委托 `MatchController`。

use std::{path::PathBuf, sync::Arc, time::Instant};

use ra_renderer::Renderer;
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
    menu_view::{MenuAction, MenuLayout, layout_for, layout_skirmish_lobby},
    screen::OriginalScreen,
    ui_assets::{MenuUiProbe, probe_menu_ui_assets, stamp_top_left, stamp_top_right},
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
    /// 后台遭遇战装载（`LoadScreen` 期间轮询）。
    load_job: Option<LoadJob>,
    /// 当前装载开始时刻（脉搏标题用）。
    load_started: Option<Instant>,
    /// 遭遇战大厅可选地图。
    lobby_maps: Vec<crate::boot::BootMapCandidate>,
    /// 当前选中的地图文件名。
    selected_map: Option<String>,
    /// 主菜单阶段 UI 资源探测（惰性一次）。
    ui_probe: Option<MenuUiProbe>,
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
            load_job: None,
            load_started: None,
            lobby_maps: Vec::new(),
            selected_map: None,
            ui_probe: None,
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
            load_job: None,
            load_started: None,
            lobby_maps: Vec::new(),
            selected_map: None,
            ui_probe: None,
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
        tracing::info!(
            count = self.lobby_maps.len(),
            selected = ?self.selected_map,
            "遭遇战地图列表已刷新"
        );
    }

    fn cycle_lobby_map(&mut self, delta: isize) {
        self.ensure_lobby_maps();
        if self.lobby_maps.is_empty() {
            self.selected_map = None;
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
            self.refresh_menu_backdrop();
            self.refresh_shell_title();
        }
    }

    fn refresh_menu_backdrop(&mut self) {
        let w = self.window_width.max(1.0) as u32;
        let h = self.window_height.max(1.0) as u32;
        if self.screen == OriginalScreen::SkirmishLobby {
            self.ensure_lobby_maps();
            let mut layout = layout_skirmish_lobby(w, h, &self.lobby_maps, self.selected_map.as_deref());
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
        if let Some(mut layout) = layout_for(self.screen, w, h) {
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
        }
        else {
            self.menu = None;
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
            MenuAction::SelectMap(i) => {
                if let Some(map) = self.lobby_maps.get(i) {
                    self.selected_map = Some(map.file_name.clone());
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
            OriginalScreen::MainMenu => format!("ra2 · 主菜单 · {}", self.banner),
            OriginalScreen::SinglePlayerMenu => "ra2 · 单人游戏 · 遭遇战 Enter · Esc 返回".into(),
            OriginalScreen::SkirmishLobby => {
                let map = self.selected_map.as_deref().unwrap_or("（无可用图）");
                format!(
                    "ra2 · 遭遇战大厅 · 地图 {map} · ←/→ 切换 · Enter 开始 · Esc 返回"
                )
            }
            OriginalScreen::Network => "ra2 · 网络（未开放）· Esc 返回".into(),
            OriginalScreen::LoadScreen => format!("ra2 · 加载 · {}", self.banner),
            OriginalScreen::Options => "ra2 · 选项（占位）· Esc 返回".into(),
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
            "正在装载 {}…",
            self.selected_map.as_deref().unwrap_or("默认候选图")
        );
        self.pending_after_load = Some(OriginalScreen::Match);
        self.set_screen(OriginalScreen::LoadScreen);
        self.load_started = Some(Instant::now());
        #[cfg(feature = "test-harness")]
        {
            if let Some(scene) = self.test_scene.clone() {
                self.load_job = Some(LoadJob::start_test_scene(scene));
                return;
            }
        }
        self.load_job = Some(LoadJob::start_install_boot(self.selected_map.clone()));
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
                    self.banner = format!("装载中{pulse} · {secs}s · Esc 取消");
                }
            }
            Err(()) => {
                self.load_job = None;
                self.load_started = None;
                self.banner = "装载线程异常断开 · Enter 重试".into();
                self.set_screen(OriginalScreen::SkirmishLobby);
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
            self.banner = format!("装载失败 · {} · Enter 重试", self.banner);
            self.set_screen(OriginalScreen::SkirmishLobby);
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
                self.banner = "占位色块菜单 · 非 Pre-Alpha 原版 UI".into();
                self.set_screen(OriginalScreen::MainMenu);
            }
        }
    }

    fn handle_pre_game_key(&mut self, event_loop: &ActiveEventLoop, key: PhysicalKey) {
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
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::SinglePlayerMenu),
                _ => {}
            },
            OriginalScreen::Network | OriginalScreen::Options => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.set_screen(OriginalScreen::MainMenu);
                }
            }
            OriginalScreen::LoadScreen => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.cancel_skirmish_load();
                }
            }
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
            }
            if self.menu.is_none() {
                self.refresh_menu_backdrop();
            }
            self.renderer.draw_frame(None);
            self.refresh_shell_title();
        }
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
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
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
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        if let Some(menu) = &self.menu {
                            if let Some(action) = menu.hit(self.cursor.0, self.cursor.1, self.window_width, self.window_height)
                            {
                                self.apply_menu_action(event_loop, action);
                            }
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
