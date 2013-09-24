//! 原生 GUI 入口：二进制名 `ra2`（Windows 上为 `ra2.exe`）。
//!
//! 不是命令行工具——启动配置来自 exe/工作目录旁的 `config.toml`，然后由窗口接管进程。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod fs_source;
#[cfg(feature = "test-harness")]
mod test_boot;

use std::{path::PathBuf, sync::Arc, time::Instant};

use ra_adaptor::{ResourceChain, RulesDb, detect_edition, load_rules_chain};
use ra_engine::{Engine, Session, open_skirmish_session};
use ra_map::{MapEntityKind, MapInfo, compose_boot_preview, find_first_boot_map, mount_theater_mixes};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::{GameEdition, RaError, RaResult};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    config::{DesktopConfig, load_desktop_config_with_diagnostics},
    fs_source::GameAssetSource,
};

struct App {
    window: Option<Arc<Window>>,
    title_base: String,
    /// 长期引擎（与当前会话共享定义生命周期）。
    engine: Option<Engine>,
    session: Option<Session>,
    renderer: Renderer,
    /// 左键拖拽中：上一帧光标位置。
    drag_last: Option<(f64, f64)>,
    /// 已按下左键，等待第一次 CursorMoved 建立起点。
    drag_armed: bool,
    /// 本次左键按下后累计拖拽距离（像素）；用于区分点击与平移。
    drag_distance: f32,
    /// 最近光标位置（窗口像素）。
    cursor: (f64, f64),
    /// 上一帧时间，用于固定仿真时钟。
    last_pump: Instant,
    /// 已记录过的胜负文案，避免每帧刷日志。
    logged_outcome: Option<String>,
    /// 上一回记入标题/日志的拒绝摘要，避免每帧刷日志。
    logged_reject: Option<String>,
    /// Shift 是否按下（多选）。
    shift_down: bool,
    /// Ctrl 是否按下（全选同阵营等）。
    ctrl_down: bool,
    /// 窗口逻辑尺寸（测试构建可固定）。
    window_width: f64,
    window_height: f64,
    /// 测试状态旁路文件（可选）。
    status_path: Option<PathBuf>,
    /// 建造放置模式：待放置的建筑类型 ID；`None` 表示普通点选。
    place_mode: Option<&'static str>,
    /// 测试场景名（test-harness 重开用）。
    test_scene: Option<String>,
}

impl App {
    fn new(
        _boot_note: String,
        engine: Option<Engine>,
        session: Option<Session>,
        preview: Option<RgbaImage>,
        window_width: f64,
        window_height: f64,
        status_path: Option<PathBuf>,
        test_scene: Option<String>,
    ) -> Self {
        let edition = session.as_ref().map(|s| s.world.edition.as_str()).unwrap_or("—");
        let mut renderer = Renderer::new();
        if let Some(image) = preview {
            renderer.set_preview(image);
        }
        Self {
            window: None,
            title_base: format!("ra2 ({edition})"),
            engine,
            session,
            renderer,
            drag_last: None,
            drag_armed: false,
            drag_distance: 0.0,
            cursor: (0.0, 0.0),
            last_pump: Instant::now(),
            logged_outcome: None,
            logged_reject: None,
            shift_down: false,
            ctrl_down: false,
            window_width,
            window_height,
            status_path,
            place_mode: None,
            test_scene,
        }
    }

    fn cursor_cell(&self) -> Option<(u16, u16)> {
        let session = self.session.as_ref()?;
        let window = self.window.as_ref()?;
        let size = window.inner_size();
        let (wx, wy) = self.renderer.camera().screen_to_world(
            self.cursor.0 as f32,
            self.cursor.1 as f32,
            size.width as f32,
            size.height as f32,
        );
        session.image_to_cell(wx, wy)
    }

    fn handle_left_click(&mut self) {
        let add = self.shift_down;
        let Some(cell) = self.cursor_cell()
        else {
            if !add {
                if let Some(session) = self.session.as_mut() {
                    session.selected.clear();
                }
            }
            return;
        };
        if let Some(type_id) = self.place_mode {
            if let Some(session) = self.session.as_mut() {
                tracing::info!("放置建筑 {type_id} @({},{})", cell.0, cell.1);
                session.order_place_building(type_id, cell.0, cell.1);
            }
            return;
        }
        let Some(session) = self.session.as_mut()
        else {
            return;
        };
        if let Some(i) = session.pick_entity_at(cell.0, cell.1) {
            if add {
                session.select_add(i);
                tracing::info!("加选实体 #{i} @({},{}) · 选中 {:?}", cell.0, cell.1, session.selected);
            }
            else {
                session.select_only(i);
                tracing::info!("选中实体 #{i} @({},{})", cell.0, cell.1);
            }
        }
        else if !add {
            session.selected.clear();
            tracing::debug!("点空地 ({},{})，清空选中", cell.0, cell.1);
        }
    }

    fn cycle_place_mode(&mut self) {
        const CYCLE: &[Option<&'static str>] = &[None, Some("GAPOWR"), Some("GAPILE"), Some("GAREFN"), Some("GAWEAP")];
        let idx = CYCLE.iter().position(|m| *m == self.place_mode).unwrap_or(0);
        self.place_mode = CYCLE[(idx + 1) % CYCLE.len()];
        match self.place_mode {
            Some(id) => tracing::info!("建造模式 · 放置 {id}（再按 B 切换，Esc 取消）"),
            None => tracing::info!("建造模式 · 已关闭"),
        }
    }

    fn handle_right_click(&mut self) {
        let Some(cell) = self.cursor_cell()
        else {
            return;
        };
        let Some(session) = self.session.as_mut()
        else {
            return;
        };
        if session.selected.is_empty() {
            return;
        }
        if session.selection_has_structure() {
            tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, session.selected);
            session.order_selected_rally(cell.0, cell.1);
            return;
        }
        if let Some(target) = session.pick_entity_at(cell.0, cell.1) {
            let hostile = session
                .selected
                .first()
                .and_then(|&atk| {
                    let a = session.world.entities.get(atk)?;
                    let t = session.world.entities.get(target)?;
                    Some(a.owner != t.owner)
                })
                .unwrap_or(false);
            if hostile {
                tracing::info!("命令攻击 → #{target}（选中 {:?}）", session.selected);
                session.order_selected_attack(target);
                return;
            }
        }
        tracing::info!("命令移动 → ({},{})（选中 {:?}）", cell.0, cell.1, session.selected);
        session.order_selected_move(cell.0, cell.1);
    }

    fn refresh_title(&mut self) {
        if let Some(window) = &self.window {
            let zoom = self.renderer.camera().zoom;
            let title = if let Some(session) = self.session.as_ref() {
                let snap = session.snapshot();
                let local = session
                    .world
                    .players
                    .iter()
                    .find(|p| p.id == session.world.local_player)
                    .and_then(|lp| snap.players.iter().find(|p| p.house == lp.house));
                let econ = local
                    .map(|p| {
                        let low = if p.low_power { "!" } else { "" };
                        format!("${} 电{}/{}{low}", p.funds, p.power_output, p.power_drain)
                    })
                    .unwrap_or_else(|| "$-".into());
                let queue = snap
                    .produce_queues
                    .first()
                    .map(|q| format!("q:{}:{}", q.type_id, q.remaining_ticks))
                    .unwrap_or_else(|| "q:-".into());
                let reject = snap.last_rejects.first().map(|r| r.reason.as_hud_label()).unwrap_or("-");
                let place = self.place_mode.unwrap_or("-");
                if let Some(ra_engine::MatchOutcome::Victory { owner }) = snap.outcome.as_ref() {
                    let stats = snap
                        .match_stats
                        .as_ref()
                        .map(|s| {
                            format!(
                                " · {}tick 损{}u/{}b 花${}",
                                s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent
                            )
                        })
                        .unwrap_or_default();
                    format!("{} · t{} · 胜 {owner}{stats} · R重开", self.title_base, snap.tick)
                }
                else if snap.paused {
                    let reason = snap.pause_reason.as_deref().unwrap_or("已暂停");
                    format!("{} · t{} · 暂停 · {reason}", self.title_base, snap.tick)
                }
                else {
                    let nsel = session.selected.len();
                    let sel = session.selected.first().copied();
                    let sel_part = match (sel, nsel) {
                        (Some(i), n) if n > 1 => format!("#{i}+{}", n - 1),
                        (Some(i), _) => format!("#{i}"),
                        (None, _) => "#-".into(),
                    };
                    format!(
                        "{} · t{} · {econ} · {queue} · 建:{place} · {reject} · {sel_part} · z{:.2}",
                        self.title_base, snap.tick, zoom
                    )
                }
            }
            else {
                format!("{} · z{:.2}", self.title_base, zoom)
            };
            window.set_title(&title);
        }
        if let Some(session) = self.session.as_ref() {
            if let Some(reject) = session.world.last_rejects().first() {
                let label = reject.reason.as_hud_label().to_string();
                if self.logged_reject.as_deref() != Some(label.as_str()) {
                    self.logged_reject = Some(label.clone());
                    tracing::info!("命令拒绝 · {label}");
                }
            }
        }
        if let (Some(path), Some(session)) = (self.status_path.as_ref(), self.session.as_ref()) {
            #[cfg(feature = "test-harness")]
            crate::test_boot::write_status(path, session);
            #[cfg(not(feature = "test-harness"))]
            let _ = (path, session);
        }
    }

    fn note_outcome_once(&mut self) {
        let Some(session) = self.session.as_ref()
        else {
            return;
        };
        let Some(ra_engine::MatchOutcome::Victory { owner }) = session.outcome.as_ref()
        else {
            return;
        };
        if self.logged_outcome.as_deref() == Some(owner.as_str()) {
            return;
        }
        self.logged_outcome = Some(owner.clone());
        let stats = session
            .match_stats
            .as_ref()
            .map(|s| {
                format!(
                    " · {}tick · 损单位{} · 损建筑{} · 花费{}",
                    s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent
                )
            })
            .unwrap_or_default();
        tracing::info!("对局结束 · 胜方 {owner} · tick={}{stats} · 按 R 重开", session.world.tick);
    }

    /// 按当前启动路径重新装载一局（结算后或任意时刻）。
    fn rematch(&mut self) {
        tracing::info!("重开对局…");
        let boot = self.boot_again();
        if let Some(preview) = boot.preview {
            self.renderer.set_preview(preview);
        }
        self.engine = boot.engine;
        self.session = boot.session;
        self.logged_outcome = None;
        self.logged_reject = None;
        self.place_mode = None;
        self.last_pump = Instant::now();
        if let Some(session) = self.session.as_ref() {
            let edition = session.world.edition.as_str();
            self.title_base = format!("ra2 ({edition})");
            tracing::info!("重开完成 · {}", boot.note);
        }
        else {
            tracing::error!("重开失败 · {}", boot.note);
        }
        self.refresh_title();
    }

    fn boot_again(&self) -> BootResult {
        let _ = self.test_scene.as_ref();
        #[cfg(feature = "test-harness")]
        {
            if let Some(scene) = self.test_scene.as_ref() {
                return match crate::test_boot::boot_scene(scene) {
                    Ok(t) => BootResult {
                        note: t.note,
                        engine: None,
                        session: Some(t.session),
                        preview: t.preview,
                    },
                    Err(e) => BootResult {
                        note: format!("重开失败: {e}"),
                        engine: None,
                        session: None,
                        preview: None,
                    },
                };
            }
        }
        boot_from_install()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(self.title_base.clone())
                        .with_inner_size(winit::dpi::LogicalSize::new(self.window_width, self.window_height)),
                )
                .expect("创建窗口失败"),
        );
        if let Err(e) = self.renderer.attach_window(window.clone()) {
            tracing::error!("wgpu 附着失败: {e}");
        }
        else {
            tracing::info!(
                "gpu={} preview={} zoom={:.2}",
                self.renderer.backend_name(),
                if self.renderer.has_preview() { "yes" } else { "no" },
                self.renderer.camera().zoom
            );
        }
        self.window = Some(window);
        self.refresh_title();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(mods) => {
                self.shift_down = mods.state().shift_key();
                self.ctrl_down = mods.state().control_key();
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => match state {
                ElementState::Pressed => {
                    self.drag_armed = true;
                    self.drag_last = None;
                    self.drag_distance = 0.0;
                }
                ElementState::Released => {
                    let was_click = self.drag_armed && self.drag_distance < 6.0;
                    self.drag_armed = false;
                    self.drag_last = None;
                    if was_click {
                        self.handle_left_click();
                    }
                }
            },
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => {
                self.handle_right_click();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                if self.drag_armed {
                    if let Some((lx, ly)) = self.drag_last {
                        let dx = (position.x - lx) as f32;
                        let dy = (position.y - ly) as f32;
                        self.drag_distance += (dx * dx + dy * dy).sqrt();
                        self.renderer.pan_screen(dx, dy);
                    }
                    self.drag_last = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if steps != 0.0 {
                    let factor = if steps > 0.0 { 1.1_f32 } else { 1.0 / 1.1 };
                    self.renderer.zoom_by(factor.powf(steps.abs()));
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let step = 48.0_f32;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) if self.ctrl_down => {
                        if let Some(session) = self.session.as_mut() {
                            let seed = session.selected.first().copied().or_else(|| {
                                session.world.entities.iter().position(|e| {
                                    !e.dead
                                        && matches!(
                                            e.kind,
                                            MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                                        )
                                })
                            });
                            if let Some(i) = seed {
                                session.select_all_of_owner(i);
                                tracing::info!("全选同阵营 · {} 个", session.selected.len());
                            }
                        }
                    }
                    PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyA) => {
                        self.renderer.pan_screen(step, 0.0);
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyD) => {
                        self.renderer.pan_screen(-step, 0.0);
                    }
                    PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::KeyW) => {
                        self.renderer.pan_screen(0.0, step);
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::KeyS) => {
                        self.renderer.pan_screen(0.0, -step);
                    }
                    PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                        self.renderer.zoom_by(1.1);
                    }
                    PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                        self.renderer.zoom_by(1.0 / 1.1);
                    }
                    PhysicalKey::Code(KeyCode::Tab) => {
                        if let Some(session) = self.session.as_mut() {
                            session.cycle_selection();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyF) => {
                        if let Some(session) = self.session.as_mut() {
                            if let Some(&atk) = session.selected.first() {
                                if let Some(tgt) = session.nearest_hostile(atk) {
                                    session.order_selected_attack(tgt);
                                }
                            }
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyX) => {
                        if let Some(session) = self.session.as_mut() {
                            tracing::info!("部署选中 · {:?}", session.selected);
                            session.order_selected_deploy();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyB) => {
                        self.cycle_place_mode();
                    }
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.place_mode.is_some() {
                            self.place_mode = None;
                            tracing::info!("建造模式 · 已关闭");
                        }
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        if let Some(session) = self.session.as_mut() {
                            session.toggle_pause();
                            if session.paused {
                                tracing::info!("暂停 · {}", session.pause_reason.as_deref().unwrap_or("已暂停"));
                            }
                            else {
                                tracing::info!("继续");
                            }
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyR) => {
                        self.rematch();
                    }
                    PhysicalKey::Code(KeyCode::KeyP) => {
                        if let Some(session) = self.session.as_mut() {
                            tracing::info!("生产 · E1");
                            session.order_produce("E1");
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyO) => {
                        if let Some(session) = self.session.as_mut() {
                            tracing::info!("生产 · MTNK");
                            session.order_produce("MTNK");
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyY) => {
                        if let Some(cell) = self.cursor_cell() {
                            if let Some(session) = self.session.as_mut() {
                                tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, session.selected);
                                session.order_selected_rally(cell.0, cell.1);
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_pump).as_secs_f64();
                self.last_pump = now;
                if let Some(session) = self.session.as_mut() {
                    let _advanced = session.pump(dt);
                }
                self.note_outcome_once();
                let snap = self.session.as_ref().map(|s| s.snapshot());
                self.renderer.draw_frame(snap.as_ref());
                self.refresh_title();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

struct BootResult {
    note: String,
    engine: Option<Engine>,
    session: Option<Session>,
    preview: Option<RgbaImage>,
}

fn load_map_terrain_preview(
    source: &GameAssetSource,
    map: &MapInfo,
    chain: &ResourceChain,
    rules: &RulesDb,
) -> Option<(String, RgbaImage, i32, i32)> {
    let preview = compose_boot_preview(
        source,
        map,
        chain.art_ini,
        &|id| rules.overlay_types.name(id).map(str::to_owned),
        &|base, owner| rules.color_schemes.palette_for_house(&rules.rules, base, owner),
    )?;
    let rgba = RgbaImage::new(preview.image.width, preview.image.height, preview.image.pixels)?;
    Some((preview.note, rgba, preview.origin_x, preview.origin_y))
}

fn load_boot_map(source: &mut GameAssetSource, edition: GameEdition, note: &mut String) -> MapInfo {
    let loaded = find_first_boot_map(edition, source);
    let theater_mounted = mount_theater_mixes(loaded.map.theater, &mut |mix| matches!(source.vfs.mount_nested(mix), Ok(true)));
    *note = format!("{note} · {} · 剧院mix {}", loaded.note, theater_mounted);
    loaded.map
}

fn boot_world(cfg: &DesktopConfig) -> RaResult<BootResult> {
    let root = cfg.ra2_dir.clone();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&root, explicit)?;
    for report in &manifest.stack.unsupported {
        tracing::warn!("适配能力缺口 [{}] {}", report.code, report.message);
    }
    if !manifest.stack.extensions.is_empty() {
        let ids: Vec<_> = manifest.stack.extensions.iter().map(|e| e.as_str()).collect();
        tracing::info!("适配扩展探测: {}", ids.join("+"));
    }
    let chain = &manifest.chain;

    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, skipped_root) = source.mount_present_roots(&manifest.present_mixes);
    let mounted_nested = source.mount_nested_names(chain.nested_mix_files);

    let mut note = format!(
        "{} · 根mix {} · 嵌套 {} · 跳过 {} · 缺盘 {}",
        chain.edition.as_str(),
        mounted_root,
        mounted_nested,
        skipped_root,
        manifest.missing_mixes.len()
    );

    let map = load_boot_map(&mut source, chain.edition, &mut note);

    let mut preview_origin = (0i32, 0i32);
    let rules = match load_rules_chain(&source, chain) {
        Ok(db) => Some(db),
        Err(e) => {
            note = format!("{note} · 规则待加载（{e}）");
            None
        }
    };

    let preview = match rules.as_ref().and_then(|rules| load_map_terrain_preview(&source, &map, chain, rules)) {
        Some((name, image, ox, oy)) => {
            note = format!("{note} · preview:{name}");
            preview_origin = (ox, oy);
            Some(image)
        }
        None => {
            note = format!("{note} · preview:无");
            None
        }
    };

    let (engine, session) =
        match rules.as_ref().map(|rules| open_skirmish_session(&source, chain, rules, map, note.clone(), preview_origin)) {
            Some(Ok(opened)) => {
                note = opened.note;
                tracing::info!(
                    "fingerprint edition={} map={} rules_hash={:#x}",
                    opened.session.fingerprint.edition,
                    opened.session.fingerprint.map,
                    opened.session.fingerprint.rules_hash
                );
                (Some(opened.engine), Some(opened.session))
            }
            Some(Err(e)) => {
                note = format!("{note} · 会话未打开（{e}）");
                (None, None)
            }
            None => (None, None),
        };

    Ok(BootResult { note, engine, session, preview })
}

/// 初始化终端 + `logs/ra2.log` 双输出。返回的 guard 必须持有到进程结束。
fn init_tracing() -> WorkerGuard {
    let _ = std::fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::never("logs", "ra2.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr_layer = fmt::layer().with_writer(std::io::stderr);
    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);
    tracing_subscriber::registry().with(filter).with(stderr_layer).with(file_layer).init();
    guard
}

fn main() {
    if let Err(e) = run() {
        // init 可能失败，仍尽量打 stderr。
        eprintln!("ra2 错误: {e}");
        tracing::error!("致命错误: {e}");
        std::process::exit(1);
    }
}

fn run() -> RaResult<()> {
    let _log_guard = init_tracing();
    tracing::info!("ra2 启动");

    let (boot, window_width, window_height, status_path, test_scene) = resolve_boot()?;

    if let Some(session) = boot.session.as_ref() {
        tracing::info!(
            "preview_origin=({}, {}) entities={}",
            session.preview_origin_x,
            session.preview_origin_y,
            session.world.entities.len()
        );
    }

    let event_loop = EventLoop::new().map_err(|e| RaError::Msg(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(
        boot.note,
        boot.engine,
        boot.session,
        boot.preview,
        window_width,
        window_height,
        status_path,
        test_scene,
    );
    event_loop.run_app(&mut app).map_err(|e| RaError::Msg(e.to_string()))?;
    tracing::info!("事件循环结束");
    Ok(())
}

fn resolve_boot() -> RaResult<(BootResult, f64, f64, Option<PathBuf>, Option<String>)> {
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
                status_path.as_ref().map(|p| p.display().to_string().unwrap_or_else(|| "—".into()))
            );
            let t = crate::test_boot::boot_scene(&scene)?;
            tracing::info!("boot: {} · session=ok", t.note);
            return Ok((
                BootResult {
                    note: t.note,
                    engine: None,
                    session: Some(t.session),
                    preview: t.preview,
                },
                window_width,
                window_height,
                status_path,
                Some(scene),
            ));
        }
    }

    Ok((boot_from_install(), 1024.0, 768.0, None, None))
}

fn boot_from_install() -> BootResult {
    let (cfg, cfg_diags) = load_desktop_config_with_diagnostics();
    for d in &cfg_diags {
        tracing::warn!("配置诊断 {} · {}", d.source, d.message);
    }
    match (&cfg.net_url, &cfg.net_room) {
        (Some(url), room) => {
            tracing::info!("联机配置预留 url={} room={}（协议未定点，不接 socket）", url, room.as_deref().unwrap_or("—"))
        }
        (None, _) => tracing::info!("联机配置：未设 net_url"),
    }
    let boot = match boot_world(&cfg) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("启动失败: {e}");
            BootResult {
                note: format!("启动失败: {e}"),
                engine: None,
                session: None,
                preview: None,
            }
        }
    };
    tracing::info!("boot: {} · session={}", boot.note, if boot.session.is_some() { "ok" } else { "none" });
    boot
}
