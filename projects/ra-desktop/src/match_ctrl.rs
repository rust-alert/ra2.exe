//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{path::PathBuf, sync::Arc, time::Instant};

use ra_engine::{Engine, HudSnapshot, MatchOutcome, Session, SessionPhase};
use ra_map::MapEntityKind;
use ra_renderer::Renderer;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{boot::BootResult, hud_chrome, local_player::LocalPlayerController};

/// 对局控制器向外壳报告的导航意图（外壳改 `AppScreen`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchNav {
    /// 无导航。
    None,
    /// 请求重开（外壳进入 Loading 再装载）。
    Rematch,
    /// 对局已结束，应切到结算页。
    ToResults,
    /// 离开对局/结算，回到遭遇战大厅（保留选图）。
    ToMainMenu,
}

/// 对局页专用状态（与菜单 / 加载页隔离）。
pub struct MatchController {
    /// 长期引擎。
    pub engine: Option<Engine>,
    /// 当前会话。
    pub session: Option<Session>,
    /// 本地选中与点选指令（非权威）。
    pub local: LocalPlayerController,
    /// 左键拖拽中：上一帧光标位置。
    drag_last: Option<(f64, f64)>,
    /// 已按下左键，等待第一次 CursorMoved 建立起点。
    drag_armed: bool,
    /// 本次左键按下后累计拖拽距离（像素）。
    drag_distance: f32,
    /// 最近光标位置（窗口像素）。
    cursor: (f64, f64),
    /// 上一帧时间，用于固定仿真时钟。
    last_pump: Instant,
    /// 已记录过的胜负文案。
    logged_outcome: Option<String>,
    /// 上一回记入日志的拒绝摘要。
    logged_reject: Option<String>,
    /// Shift 是否按下（多选）。
    shift_down: bool,
    /// Ctrl 是否按下。
    ctrl_down: bool,
    /// 建造放置模式。
    place_mode: Option<&'static str>,
    /// 标题用版本短名。
    title_base: String,
    /// 测试状态旁路文件。
    status_path: Option<PathBuf>,
    /// 测试场景名（重开用；当前由外壳 `LoadJob` 持有同名副本）。
    #[allow(dead_code)]
    test_scene: Option<String>,
}

impl MatchController {
    /// 由装载结果构造；可无会话（装载失败时仍占位）。
    pub fn from_boot(boot: BootResult, status_path: Option<PathBuf>, test_scene: Option<String>) -> Self {
        let edition = boot
            .session
            .as_ref()
            .and_then(|s| s.game())
            .map(|g| g.world.edition.as_str())
            .unwrap_or("—");
        Self {
            engine: boot.engine,
            session: boot.session,
            local: LocalPlayerController::new(),
            drag_last: None,
            drag_armed: false,
            drag_distance: 0.0,
            cursor: (0.0, 0.0),
            last_pump: Instant::now(),
            logged_outcome: None,
            logged_reject: None,
            shift_down: false,
            ctrl_down: false,
            place_mode: None,
            title_base: format!("ra2 ({edition})"),
            status_path,
            test_scene,
        }
    }

    /// 是否已有可玩会话。
    pub fn has_session(&self) -> bool {
        self.session.as_ref().and_then(|s| s.game()).is_some()
    }

    /// 应用新的装载结果（重开）。
    pub fn apply_boot(&mut self, boot: BootResult, renderer: &mut Renderer) {
        if let Some(preview) = boot.preview {
            renderer.set_preview(preview);
        }
        renderer.clear_match_visuals();
        self.engine = boot.engine;
        self.session = boot.session;
        self.local.clear();
        self.logged_outcome = None;
        self.logged_reject = None;
        self.place_mode = None;
        self.last_pump = Instant::now();
        if let Some(game) = self.session.as_ref().and_then(|s| s.game()) {
            self.title_base = format!("ra2 ({})", game.world.edition.as_str());
            tracing::info!("重开完成 · {}", boot.note);
        }
        else {
            tracing::error!("重开失败 · {}", boot.note);
        }
    }

    /// 按当前路径再装载一局（同步；事件循环内请改走 `LoadJob`）。
    #[allow(dead_code)]
    pub fn boot_again(&self) -> BootResult {
        #[cfg(feature = "test-harness")]
        {
            if let Some(scene) = self.test_scene.as_ref() {
                return match crate::test_boot::boot_scene(scene) {
                    Ok(t) => BootResult {
                        note: t.note,
                        engine: Some(t.engine),
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
        let _ = &self.test_scene;
        crate::boot::boot_from_install()
    }

    fn cursor_cell(&self, renderer: &Renderer, window: &Window) -> Option<(u16, u16)> {
        let game = self.session.as_ref()?.game()?;
        let size = window.inner_size();
        let (wx, wy) = renderer.camera().screen_to_world(
            self.cursor.0 as f32,
            self.cursor.1 as f32,
            size.width as f32,
            size.height as f32,
        );
        game.image_to_cell(wx, wy)
    }

    fn handle_left_click(&mut self, renderer: &Renderer, window: &Window) {
        let add = self.shift_down;
        let Some(cell) = self.cursor_cell(renderer, window)
        else {
            if !add {
                self.local.clear();
            }
            return;
        };
        if let Some(type_id) = self.place_mode {
            if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                tracing::info!("放置建筑 {type_id} @({},{})", cell.0, cell.1);
                game.order_place_building(type_id, cell.0, cell.1);
            }
            return;
        }
        let Some(game) = self.session.as_ref().and_then(|s| s.game())
        else {
            return;
        };
        if let Some(id) = game.pick_entity_at(cell.0, cell.1) {
            if add {
                self.local.select_add(game, id);
                tracing::info!("加选实体 #{} @({},{}) · 选中 {:?}", id.0, cell.0, cell.1, self.local.selected);
            }
            else {
                self.local.select_only(game, id);
                tracing::info!("选中实体 #{} @({},{})", id.0, cell.0, cell.1);
            }
        }
        else if !add {
            self.local.clear();
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

    fn handle_right_click(&mut self, renderer: &Renderer, window: &Window) {
        let Some(cell) = self.cursor_cell(renderer, window)
        else {
            return;
        };
        if self.local.selected.is_empty() {
            return;
        }
        let selected = self.local.selected.clone();
        let Some(game) = self.session.as_mut().and_then(|s| s.game_mut())
        else {
            return;
        };
        if game.selection_has_structure(&selected) {
            tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
            game.order_rally(&selected, cell.0, cell.1);
            return;
        }
        if let Some(target) = game.pick_entity_at(cell.0, cell.1) {
            let hostile = selected
                .first()
                .and_then(|&atk| {
                    let ai = game.world.entity_index(atk)?;
                    let ti = game.world.entity_index(target)?;
                    let a = game.world.entities.get(ai)?;
                    let t = game.world.entities.get(ti)?;
                    Some(a.owner != t.owner)
                })
                .unwrap_or(false);
            if hostile {
                tracing::info!("命令攻击 → #{}（选中 {:?}）", target.0, selected);
                game.order_attack(&selected, target);
                return;
            }
        }
        tracing::info!("命令移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
        game.order_move(&selected, cell.0, cell.1);
    }

    /// 对局页输入。`accept_commands=false` 时仅允许相机与重开 / 回菜单。
    pub fn handle_event(
        &mut self,
        event: &WindowEvent,
        renderer: &mut Renderer,
        window: &Window,
        accept_commands: bool,
    ) -> MatchNav {
        match event {
            WindowEvent::ModifiersChanged(mods) => {
                self.shift_down = mods.state().shift_key();
                self.ctrl_down = mods.state().control_key();
                MatchNav::None
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if accept_commands => {
                match state {
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
                            self.handle_left_click(renderer, window);
                        }
                    }
                }
                MatchNav::None
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } if !accept_commands =>
            {
                // 结算页：占位色块可点重开 / 回大厅（非原版按钮）。
                let size = window.inner_size();
                match hud_chrome::hit_results(self.cursor.0, self.cursor.1, size.width as f64, size.height as f64) {
                    Some(hud_chrome::ResultsHit::Rematch) => {
                        tracing::info!("结算 · 点击重开");
                        MatchNav::Rematch
                    }
                    Some(hud_chrome::ResultsHit::ToLobby) => {
                        tracing::info!("结算 · 点击返回大厅");
                        MatchNav::ToMainMenu
                    }
                    None => MatchNav::None,
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. }
                if accept_commands =>
            {
                self.handle_right_click(renderer, window);
                MatchNav::None
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                if accept_commands && self.drag_armed {
                    if let Some((lx, ly)) = self.drag_last {
                        let dx = (position.x - lx) as f32;
                        let dy = (position.y - ly) as f32;
                        self.drag_distance += (dx * dx + dy * dy).sqrt();
                        renderer.pan_screen(dx, dy);
                    }
                    self.drag_last = Some((position.x, position.y));
                }
                MatchNav::None
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if steps != 0.0 {
                    let factor = if steps > 0.0 { 1.1_f32 } else { 1.0 / 1.1 };
                    renderer.zoom_by(factor.powf(steps.abs()));
                }
                MatchNav::None
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return MatchNav::None;
                }
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyR) => {
                        tracing::info!("重开对局…");
                        MatchNav::Rematch
                    }
                    PhysicalKey::Code(KeyCode::Escape) if !accept_commands => MatchNav::ToMainMenu,
                    PhysicalKey::Code(KeyCode::Escape) if accept_commands => {
                        if self.place_mode.is_some() {
                            self.place_mode = None;
                            tracing::info!("建造模式 · 已关闭");
                            MatchNav::None
                        }
                        else if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                            // 对局中 Esc 先暂停；暂停后再 Esc 回大厅（空格仍可切换暂停）。
                            if game.paused {
                                MatchNav::ToMainMenu
                            }
                            else {
                                game.toggle_pause();
                                tracing::info!(
                                    "暂停 · {}",
                                    game.pause_reason.as_deref().unwrap_or("已暂停")
                                );
                                MatchNav::None
                            }
                        }
                        else {
                            MatchNav::ToMainMenu
                        }
                    }
                    _ if !accept_commands => MatchNav::None,
                    PhysicalKey::Code(KeyCode::KeyA) if self.ctrl_down => {
                        if let Some(game) = self.session.as_ref().and_then(|s| s.game()) {
                            let seed = self.local.selected.first().copied().or_else(|| {
                                game.world
                                    .entities
                                    .iter()
                                    .find(|e| {
                                        !e.dead
                                            && matches!(
                                                e.kind,
                                                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                                            )
                                    })
                                    .map(|e| e.id)
                            });
                            if let Some(id) = seed {
                                self.local.select_all_of_owner(game, id);
                                tracing::info!("全选同阵营 · {} 个", self.local.selected.len());
                            }
                        }
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyA) => {
                        renderer.pan_screen(48.0, 0.0);
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyD) => {
                        renderer.pan_screen(-48.0, 0.0);
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::KeyW) => {
                        renderer.pan_screen(0.0, 48.0);
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::KeyS) => {
                        renderer.pan_screen(0.0, -48.0);
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                        renderer.zoom_by(1.1);
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                        renderer.zoom_by(1.0 / 1.1);
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::Tab) => {
                        if let Some(game) = self.session.as_ref().and_then(|s| s.game()) {
                            self.local.cycle_selection(game);
                        }
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyF) => {
                        let selected = self.local.selected.clone();
                        if let Some(&atk) = selected.first() {
                            if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                                if let Some(tgt) = game.nearest_hostile(atk) {
                                    game.order_attack(&selected, tgt);
                                }
                            }
                        }
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyX) => {
                        let selected = self.local.selected.clone();
                        if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                            tracing::info!("部署选中 · {:?}", selected);
                            game.order_deploy(&selected);
                        }
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyB) => {
                        self.cycle_place_mode();
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                            game.toggle_pause();
                            if game.paused {
                                tracing::info!("暂停 · {}", game.pause_reason.as_deref().unwrap_or("已暂停"));
                            }
                            else {
                                tracing::info!("继续");
                            }
                        }
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyP) => {
                        if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                            tracing::info!("生产 · E1");
                            game.order_produce("E1");
                        }
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyO) => {
                        if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                            tracing::info!("生产 · MTNK");
                            game.order_produce("MTNK");
                        }
                        MatchNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyY) => {
                        if let Some(cell) = self.cursor_cell(renderer, window) {
                            let selected = self.local.selected.clone();
                            if let Some(game) = self.session.as_mut().and_then(|s| s.game_mut()) {
                                tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                                game.order_rally(&selected, cell.0, cell.1);
                            }
                        }
                        MatchNav::None
                    }
                    _ => MatchNav::None,
                }
            }
            _ => MatchNav::None,
        }
    }

    /// 推进仿真（仅对局页调用）并检测是否应进入结算。返回导航与本段耗时。
    pub fn pump(&mut self, dt: f64) -> (MatchNav, std::time::Duration) {
        let started = Instant::now();
        let nav = if let (Some(engine), Some(session)) = (self.engine.as_ref(), self.session.as_mut()) {
            let _ = session.pump(&engine.runtime(), dt);
            if let Some(game) = session.game() {
                self.local.prune_dead(game);
            }
            if session.game().and_then(|g| g.outcome.as_ref()).is_some() {
                session.phase = SessionPhase::Finished;
                self.note_outcome_once();
                MatchNav::ToResults
            }
            else {
                MatchNav::None
            }
        }
        else {
            MatchNav::None
        };
        (nav, started.elapsed())
    }

    fn note_outcome_once(&mut self) {
        let Some(game) = self.session.as_ref().and_then(|s| s.game())
        else {
            return;
        };
        let Some(MatchOutcome::Victory { owner }) = game.outcome.as_ref()
        else {
            return;
        };
        if self.logged_outcome.as_deref() == Some(owner.as_str()) {
            return;
        }
        self.logged_outcome = Some(owner.clone());
        let stats = game
            .match_stats
            .as_ref()
            .map(|s| {
                format!(
                    " · {}tick · 损单位{} · 损建筑{} · 花费{}",
                    s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent
                )
            })
            .unwrap_or_default();
        tracing::info!("对局结束 · 胜方 {owner} · tick={}{stats} · 按 R 重开", game.world.tick);
    }

    /// 绘制当前对局：首帧或空槽全量同步，其后脏集增量。标题走 `HudSnapshot`。
    /// 屏上色块为占位 HUD，不是原版侧栏交付。
    pub fn draw_frame(&mut self, renderer: &mut Renderer, window: Option<&Arc<Window>>, screen_label: &str) {
        let Some(session) = self.session.as_mut()
        else {
            renderer.set_screen_chrome(&[]);
            renderer.draw_frame(None);
            self.refresh_title(renderer, window, screen_label, None);
            return;
        };
        let Some(game) = session.game_mut()
        else {
            renderer.set_screen_chrome(&[]);
            renderer.draw_frame(None);
            self.refresh_title(renderer, window, screen_label, None);
            return;
        };

        let selected = self.local.selected.clone();
        let local_house = game
            .world
            .players
            .iter()
            .find(|p| p.id == game.world.local_player)
            .map(|p| p.house.clone());
        let (win_w, win_h) = window
            .map(|w| {
                let s = w.inner_size();
                (s.width as f64, s.height as f64)
            })
            .unwrap_or((1.0, 1.0));
        let cursor = self.cursor;
        let force_full = renderer.render_world().unit_count() == 0;
        let pres_started = Instant::now();
        let hud = if force_full {
            let snap = game.snapshot(&selected);
            renderer.timings.presentation_build = Some(pres_started.elapsed());
            let hud = game.snapshot_hud();
            Self::apply_screen_chrome(
                renderer,
                &hud,
                local_house.as_deref(),
                screen_label,
                cursor,
                win_w,
                win_h,
            );
            renderer.draw_frame(Some(&snap));
            // 全量同步已消费脏集语义：清空以免下一帧重复投影。
            let _ = game.world.take_presentation_dirty();
            hud
        }
        else {
            let dirty = game.world.take_presentation_dirty();
            let units = game.project_units(&dirty);
            let tick = game.world.tick;
            renderer.timings.presentation_build = Some(pres_started.elapsed());
            let hud = game.snapshot_hud();
            Self::apply_screen_chrome(
                renderer,
                &hud,
                local_house.as_deref(),
                screen_label,
                cursor,
                win_w,
                win_h,
            );
            renderer.draw_incremental(tick, &dirty, &units, &selected);
            hud
        };
        self.refresh_title(renderer, window, screen_label, Some(&hud));
    }

    fn apply_screen_chrome(
        renderer: &mut Renderer,
        hud: &HudSnapshot,
        local_house: Option<&str>,
        screen_label: &str,
        cursor: (f64, f64),
        win_w: f64,
        win_h: f64,
    ) {
        let mut quads = hud_chrome::match_hud_chrome(hud, local_house);
        if screen_label == "results" {
            let hover = hud_chrome::hit_results(cursor.0, cursor.1, win_w, win_h);
            quads.extend(hud_chrome::results_chrome(hover));
        }
        renderer.set_screen_chrome(&quads);
    }

    fn refresh_title(
        &mut self,
        renderer: &Renderer,
        window: Option<&Arc<Window>>,
        screen_label: &str,
        hud: Option<&HudSnapshot>,
    ) {
        if let Some(window) = window {
            let zoom = renderer.camera().zoom;
            let title = if let Some(hud) = hud {
                let local_house = self
                    .session
                    .as_ref()
                    .and_then(|s| s.game())
                    .and_then(|g| g.world.players.iter().find(|p| p.id == g.world.local_player))
                    .map(|p| p.house.clone());
                let local = local_house.and_then(|house| hud.players.iter().find(|p| p.house == house));
                let econ = local
                    .map(|p| {
                        let low = if p.low_power { "!" } else { "" };
                        format!("${} 电{}/{}{low}", p.funds, p.power_output, p.power_drain)
                    })
                    .unwrap_or_else(|| "$-".into());
                let queue = hud
                    .produce_queues
                    .first()
                    .map(|q| format!("q:{}:{}", q.type_id, q.remaining_ticks))
                    .unwrap_or_else(|| "q:-".into());
                let reject = hud.last_rejects.first().map(|r| r.reason.as_hud_label()).unwrap_or("-");
                let place = self.place_mode.unwrap_or("-");
                if screen_label == "results" {
                    let outcome = match hud.outcome.as_ref() {
                        Some(MatchOutcome::Victory { owner }) => format!("胜 {owner}"),
                        _ => "结算".into(),
                    };
                    let stats = hud
                        .match_stats
                        .as_ref()
                        .map(|s| {
                            format!(
                                " · {}tick 损{}u/{}b 花${}",
                                s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        "{} · [results] · t{} · {outcome}{stats} · 点重开/回大厅 · R重开 Esc大厅",
                        self.title_base, hud.tick
                    )
                }
                else if let Some(MatchOutcome::Victory { owner }) = hud.outcome.as_ref() {
                    let stats = hud
                        .match_stats
                        .as_ref()
                        .map(|s| {
                            format!(
                                " · {}tick 损{}u/{}b 花${}",
                                s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        "{} · [{screen_label}] · t{} · 胜 {owner}{stats} · R重开 Esc大厅",
                        self.title_base, hud.tick
                    )
                }
                else if hud.paused {
                    let reason = hud.pause_reason.as_deref().unwrap_or("已暂停");
                    format!(
                        "{} · [{screen_label}] · t{} · 暂停 · {reason} · Esc大厅 Space继续",
                        self.title_base, hud.tick
                    )
                }
                else {
                    let nsel = self.local.selected.len();
                    let sel = self.local.selected.first().copied();
                    let sel_part = match (sel, nsel) {
                        (Some(id), n) if n > 1 => format!("#{}+{}", id.0, n - 1),
                        (Some(id), _) => format!("#{}", id.0),
                        (None, _) => "#-".into(),
                    };
                    let diff = self
                        .session
                        .as_ref()
                        .and_then(|s| s.game())
                        .map(|g| g.difficulty.as_str())
                        .unwrap_or("Normal");
                    format!(
                        "{} · [{screen_label}] · t{} · {econ} · {queue} · 建:{place} · {reject} · {sel_part} · diff={diff} · Esc暂停 · z{:.2}",
                        self.title_base, hud.tick, zoom
                    )
                }
            }
            else {
                format!("{} · [{screen_label}] · z{:.2}", self.title_base, zoom)
            };
            window.set_title(&title);
        }
        if let Some(game) = self.session.as_ref().and_then(|s| s.game()) {
            if let Some(reject) = game.world.last_rejects().first() {
                let label = reject.reason.as_hud_label().to_string();
                if self.logged_reject.as_deref() != Some(label.as_str()) {
                    self.logged_reject = Some(label.clone());
                    tracing::info!("命令拒绝 · {label}");
                }
            }
        }
        if let (Some(path), Some(session)) = (self.status_path.as_ref(), self.session.as_ref()) {
            #[cfg(feature = "test-harness")]
            crate::test_boot::write_status(path, session, &self.local.selected);
            #[cfg(not(feature = "test-harness"))]
            let _ = (path, session);
        }
    }

    /// 结算页标题刷新（不推进）。
    pub fn take_pump_clock(&mut self) -> Instant {
        let now = Instant::now();
        let prev = self.last_pump;
        self.last_pump = now;
        prev
    }
}
