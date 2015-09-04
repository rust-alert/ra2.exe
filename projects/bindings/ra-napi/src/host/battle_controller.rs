//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::{Duration, Instant}};

use ra_adaptor::RulesSystem;
use ra_assets::{CsfFile, FntFile, Palette, Rgba};
use ra_widgets::{
    fs_source::GameAssetSource,
    battle_hud::{BattleHudChrome, BattleHudHit, decode_battle_hud_chrome, hit_at_with_chrome},
    battle_order_icons::load_battle_order_icons,
    ui_compose::{BattleHudModel, compose_battle_hud_overlay},
    ui_present,
    ui_text::{command_button_csf_tooltip, resolve_csf_text},
};
use ra_engine::{Engine, HudSnapshot, BattleOutcome, Session, SessionPhase};
use ra_layout::{battle_hud_layout_with_metrics, BattleHudChromeMetrics, ui_layout::MapViewport};
use ra_map::{
    MapEntity, MapEntityKind, StructureAnimBank, StructureBuildupClip, Theater, collect_structure_anim_bank, iso_to_screen,
    load_structure_buildup_clip, paint_mobiles_onto_preview_rgba, paint_structure_anims_onto_rgba,
    paint_structure_buildup_onto_rgba, paint_structures_onto_rgba,
};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::{EntityId, PresentFeel};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use super::{
    boot::{remap_owner_palette, BootResult},
    battle_input::{
        edge_scroll_axes, edge_scroll_cursor_for, edge_scroll_screen_delta, EdgeScrollCursor, LeftGesture,
        LeftReleaseAction, ScreenRect, EDGE_SCROLL_MARGIN_PX, EDGE_SCROLL_SPEED_PX_PER_SEC,
        MARQUEE_HIT_HALF_INFANTRY_PX, MARQUEE_HIT_HALF_VEHICLE_PX, MARQUEE_VEHICLE_LIFT_PX,
    },
    local_player::LocalPlayerController,
};

/// 遭遇战开局默认缩放（1 屏幕像素 ≈ 1 预览像素；禁止整图 fit）。
const BATTLE_START_ZOOM: f32 = 1.0;
/// 选中行动线可见时长（仿真 tick，对齐原版约 25 帧窗口）。
const ACTION_LINES_DURATION_TICKS: u64 = 25;

/// 对局控制器向外壳报告的导航意图（外壳改 `AppScreen`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleNav {
    /// 无导航。
    None,
    /// 请求重开（外壳进入 Loading 再装载）。
    Rematch,
    /// 对局已结束，应切到结算页。
    ToResults,
    /// 离开对局/结算，回到遭遇战大厅（保留选图）。
    ToMainMenu,
}

/// 待播的建筑 Buildup（MCV 展开等）。
struct PendingBuildup {
    #[allow(dead_code)]
    entity: EntityId,
    type_id: String,
    owner: String,
    clip: StructureBuildupClip,
    started: Instant,
}

/// 部署已在权威侧完成、等待呈现侧播动画的任务。
struct DeployVisualJob {
    entity: EntityId,
    type_id: String,
    owner: String,
    x: u16,
    y: u16,
}

/// 对局页专用状态（与菜单 / 加载页隔离）。
pub struct BattleController {
    /// 长期引擎。
    pub engine: Option<Engine>,
    /// 当前会话。
    pub session: Option<Session>,
    /// 本地选中与点选指令（非权威）。
    pub local: LocalPlayerController,
    /// 左键点选 / 框选手势（不再用拖拽平移相机）。
    left_gesture: LeftGesture,
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
    /// 暂停后已武装「再按 Esc 回大厅」（避免误触离开）。
    leave_armed: bool,
    /// 标题用版本短名。
    title_base: String,
    /// 测试状态旁路文件。
    status_path: Option<PathBuf>,
    /// 测试场景名（重开用；当前由外壳 `LoadJob` 持有同名副本）。
    #[allow(dead_code)]
    test_scene: Option<String>,
    /// 局内 HUD chrome（按本地阵营缓存；换边或重开时刷新）。
    hud_chrome: Option<BattleHudChrome>,
    /// 是否已尝试装入 `mouse.shp` 命令图标。
    order_icons_loaded: bool,
    /// 命令条悬停槽。
    command_hover: Option<usize>,
    /// 命令条按下槽（高亮）。
    command_pressed: Option<usize>,
    /// 不含建筑活动层的预览底图（可含开局移动单位与已定格建造场）。
    preview_base: Option<RgbaImage>,
    /// 无开局移动单位、可烘焙已定格动态建筑的底图。
    preview_clean: Option<RgbaImage>,
    /// 建筑活动层银行。
    structure_anims: StructureAnimBank,
    /// art.ini 逻辑名。
    art_ini: &'static str,
    /// rules.ini 逻辑名。
    rules_ini: &'static str,
    /// 规则快照（房屋色调）。
    rules: Option<RulesSystem>,
    /// 大厅行色 → house 主色。
    lobby_primaries: HashMap<String, Rgba>,
    /// 正在播放的 Buildup。
    pending_buildups: Vec<PendingBuildup>,
    /// 权威部署完成后待启动的呈现任务。
    deploy_visual_queue: Vec<DeployVisualJob>,
    /// 预览原点。
    preview_origin: (i32, i32),
    /// 活动层呈现时钟起点。
    anim_started: Instant,
    /// 上一帧活动层签名（跳过无变化上传）。
    last_anim_sig: u64,
    /// 开局镜头尚未按战术区对齐（等表面尺寸可用后再 `focus`）。
    start_view_pending: bool,
    /// 等待本 tick 结算的部署实体（`KeyD` 下发后）。
    deploy_watch: Option<ra_types::EntityId>,
    /// 对局短音效事件 id 队列（如 `PlaceBuilding`；由壳层按 `sound.ini` 播放）。
    pending_battle_sfx: Vec<String>,
    /// 当前边缘滚屏光标（整窗边缘；右栏 / 命令条有效）。
    edge_scroll_cursor: EdgeScrollCursor,
    /// 选中行动线计时起点（仿真 tick；`None` 表示未启动）。
    action_lines_start_tick: Option<u64>,
    /// 当前地图剧院（壳层挂载剧院 MIX 用）。
    map_theater: Option<Theater>,
}

impl BattleController {
    /// 由装载结果构造；可无会话（装载失败时仍占位）。
    pub fn from_boot(boot: BootResult, status_path: Option<PathBuf>, test_scene: Option<String>) -> Self {
        let edition = boot.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.edition.as_str()).unwrap_or("—");
        let has_session = boot.session.as_ref().and_then(|s| s.battle()).is_some();
        let map_theater = boot.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.map.theater);
        let mut this = Self {
            engine: boot.engine,
            session: boot.session,
            local: LocalPlayerController::new(),
            left_gesture: LeftGesture::Idle,
            cursor: (0.0, 0.0),
            last_pump: Instant::now(),
            logged_outcome: None,
            logged_reject: None,
            shift_down: false,
            ctrl_down: false,
            place_mode: None,
            leave_armed: false,
            title_base: format!("ra2 ({edition})"),
            status_path,
            test_scene,
            hud_chrome: None,
            order_icons_loaded: false,
            command_hover: None,
            command_pressed: None,
            preview_base: boot.preview_base,
            preview_clean: boot.preview_clean,
            structure_anims: boot.structure_anims,
            art_ini: boot.art_ini,
            rules_ini: boot.rules_ini,
            rules: boot.rules,
            lobby_primaries: boot.lobby_primaries,
            pending_buildups: Vec::new(),
            deploy_visual_queue: Vec::new(),
            preview_origin: boot.preview_origin,
            anim_started: Instant::now(),
            last_anim_sig: u64::MAX,
            start_view_pending: has_session,
            deploy_watch: None,
            pending_battle_sfx: Vec::new(),
            edge_scroll_cursor: EdgeScrollCursor::Default,
            action_lines_start_tick: None,
            map_theater,
        };
        this.bind_local_start();
        this
    }

    /// 当前对局地图剧院（供壳层挂载 `isotemp` 等）。
    pub fn map_theater(&self) -> Option<Theater> {
        self.map_theater
    }

    /// 是否已有可玩会话。
    pub fn has_session(&self) -> bool {
        self.session.as_ref().and_then(|s| s.battle()).is_some()
    }

    /// 选中本地开局单位（优先 MCV）。所有装载路径共用。
    fn bind_local_start(&mut self) {
        let pulse_tick = {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            if let Some(id) = self.local.select_local_start(game) {
                tracing::info!("开局已选中本方单位 #{}", id.0);
                Some(game.world.tick)
            }
            else {
                tracing::warn!("开局未找到可本方选中的移动单位");
                None
            }
        };
        if let Some(tick) = pulse_tick {
            self.pulse_action_lines_at(tick);
        }
    }

    fn pulse_action_lines_at(&mut self, tick: u64) {
        self.action_lines_start_tick = Some(tick);
    }

    fn action_lines_active(&self) -> bool {
        let Some(start) = self.action_lines_start_tick
        else {
            return false;
        };
        let tick = self
            .session
            .as_ref()
            .and_then(|s| s.battle())
            .map(|g| g.world.tick)
            .unwrap_or(start);
        tick.saturating_sub(start) < ACTION_LINES_DURATION_TICKS
    }

    /// 表面尺寸就绪后对齐战术区并聚焦开局单位（可重复调用，只执行一次）。
    pub fn ensure_start_view(&mut self, renderer: &mut Renderer) {
        if !self.start_view_pending {
            return;
        }
        let Some((vw, vh)) = renderer.surface_size_u32()
        else {
            return;
        };
        self.sync_world_view(renderer, vw, vh);
        self.focus_camera_on_local_start(renderer);
        self.start_view_pending = false;
    }

    /// 应用新的装载结果（重开）。
    pub fn apply_boot(&mut self, boot: BootResult, renderer: &mut Renderer) {
        if let Some(preview) = boot.preview {
            renderer.set_map_preview(preview);
        }
        renderer.clear_match_visuals();
        self.engine = boot.engine;
        self.session = boot.session;
        self.local.clear();
        self.logged_outcome = None;
        self.logged_reject = None;
        self.place_mode = None;
        self.leave_armed = false;
        self.last_pump = Instant::now();
        self.hud_chrome = None;
        self.order_icons_loaded = false;
        self.command_hover = None;
        self.command_pressed = None;
        self.preview_base = boot.preview_base;
        self.preview_clean = boot.preview_clean;
        self.structure_anims = boot.structure_anims;
        self.art_ini = boot.art_ini;
        self.rules_ini = boot.rules_ini;
        self.rules = boot.rules;
        self.lobby_primaries = boot.lobby_primaries;
        self.pending_buildups.clear();
        self.deploy_visual_queue.clear();
        self.preview_origin = boot.preview_origin;
        self.anim_started = Instant::now();
        self.last_anim_sig = u64::MAX;
        self.deploy_watch = None;
        self.pending_battle_sfx.clear();
        self.edge_scroll_cursor = EdgeScrollCursor::Default;
        self.action_lines_start_tick = None;
        self.map_theater = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.map.theater);
        self.start_view_pending = self.has_session();
        if self.has_session() {
            let edition = self
                .session
                .as_ref()
                .and_then(|s| s.battle())
                .map(|g| g.world.edition.as_str())
                .unwrap_or("—");
            self.title_base = format!("ra2 ({edition})");
            tracing::info!("重开完成 · {}", boot.note);
            self.bind_local_start();
            self.ensure_start_view(renderer);
        }
        else {
            tracing::error!("重开失败 · {}", boot.note);
        }
    }

    /// 将镜头对准本地玩家开局单位（遭遇战优先 MCV 出生点附近）。
    pub fn focus_camera_on_local_start(&self, renderer: &mut Renderer) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let Some(local_house) = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.clone())
        else {
            return;
        };
        let mut fallback: Option<(u16, u16)> = None;
        let mut mcv: Option<(u16, u16)> = None;
        for id in game.world.entity_ids() {
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            if owner.as_ref() != local_house.as_ref() {
                continue;
            }
            let Some((_, _, dead)) = game.world.ecs_health(id)
            else {
                continue;
            };
            if dead {
                continue;
            }
            let Some((type_id, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some((x, y, _)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            if type_id.to_ascii_uppercase().contains("MCV") {
                mcv = Some((x, y));
                break;
            }
            if fallback.is_none() {
                fallback = Some((x, y));
            }
        }
        let Some((x, y)) = mcv.or(fallback)
        else {
            tracing::warn!("本地阵营 {} 无可用开局单位，镜头保持预览 fit", local_house);
            return;
        };
        let z = game.world.pass_grid.cell_height(x, y);
        let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
        let wx = (sx - game.preview_origin_x) as f32;
        let wy = (sy - game.preview_origin_y) as f32;
        renderer.focus_camera(wx, wy, BATTLE_START_ZOOM);
        tracing::info!("开局镜头对准 {} @({},{}) zoom={}", local_house, x, y, BATTLE_START_ZOOM);
    }

    /// 按当前路径再装载一局（同步；事件循环内请改走 `LoadJob`）。
    #[allow(dead_code)]
    pub fn boot_again(&self) -> BootResult {
        #[cfg(feature = "test-harness")]
        {
            if let Some(scene) = self.test_scene.as_ref() {
                return match super::test_boot::boot_scene(scene) {
                    Ok(t) => BootResult::from_test(t),
                    Err(e) => BootResult::failed(format!("重开失败: {e}")),
                };
            }
        }
        let _ = &self.test_scene;
        super::boot::boot_from_install()
    }

    /// 由窗口尺寸构造当前对局 `MapViewport`（命中 / 投影 / 裁切同一实例）。
    fn map_viewport(&self, window: &Window) -> MapViewport {
        let size = window.inner_size();
        MapViewport::battle(size.width.max(1), size.height.max(1))
    }

    /// 将 renderer 世界 pass 与 `MapViewport` 对齐。
    fn sync_world_view(&self, renderer: &mut Renderer, window_w: u32, window_h: u32) {
        let vp = MapViewport::battle(window_w.max(1), window_h.max(1));
        let (x, y, w, h) = vp.clip_rect_u32();
        renderer.set_world_view_rect(x, y, w, h);
    }

    fn cursor_cell(&self, renderer: &Renderer, window: &Window) -> Option<(u16, u16)> {
        let game = self.session.as_ref()?.battle()?;
        let vp = self.map_viewport(window);
        if !vp.contains_cursor(self.cursor.0 as i32, self.cursor.1 as i32) {
            return None;
        }
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);
        game.image_to_cell(wx, wy)
    }

    fn pan_world(&self, renderer: &mut Renderer, window: &Window, dx: f32, dy: f32) {
        let vp = self.map_viewport(window);
        // 夹紧与投影同口径：战术区宽高（与 `set_world_view_rect` / write_vertices 一致）。
        renderer.pan_clamped_in_viewport(dx, dy, vp.proj_w(), vp.proj_h());
    }

    /// 整窗边缘滚屏（右栏 / 底边命令条同样触发；左键拖拽不再平移相机）。
    pub fn tick_edge_scroll(&mut self, renderer: &mut Renderer, window: &Window, dt: f64, enabled: bool) {
        if !enabled || dt <= 0.0 {
            self.edge_scroll_cursor = EdgeScrollCursor::Default;
            return;
        }
        if self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.paused || g.outcome.is_some()) {
            self.edge_scroll_cursor = EdgeScrollCursor::Default;
            return;
        }
        let size = window.inner_size();
        let sw = size.width.max(1);
        let sh = size.height.max(1);
        let (west, east, north, south) =
            edge_scroll_axes(self.cursor.0, self.cursor.1, sw, sh, EDGE_SCROLL_MARGIN_PX);
        let vp = self.map_viewport(window);
        let (can_west, can_east, can_north, can_south) = self.edge_scroll_can_axes(renderer, vp.proj_w(), vp.proj_h());
        self.edge_scroll_cursor =
            edge_scroll_cursor_for(west, east, north, south, can_west, can_east, can_north, can_south);
        let (mut dx, mut dy) = edge_scroll_screen_delta(
            self.cursor.0,
            self.cursor.1,
            sw,
            sh,
            EDGE_SCROLL_MARGIN_PX,
            EDGE_SCROLL_SPEED_PX_PER_SEC,
            dt,
        );
        if dx > 0.0 && !can_west {
            dx = 0.0;
        }
        if dx < 0.0 && !can_east {
            dx = 0.0;
        }
        if dy > 0.0 && !can_north {
            dy = 0.0;
        }
        if dy < 0.0 && !can_south {
            dy = 0.0;
        }
        if dx.abs() > 0.0 || dy.abs() > 0.0 {
            self.pan_world(renderer, window, dx, dy);
        }
    }

    /// 当前边缘滚屏光标（壳层据此切换系统 / 自定义指针）。
    pub fn edge_scroll_cursor(&self) -> EdgeScrollCursor {
        self.edge_scroll_cursor
    }

    /// 当前选中是否含可部署单位（MCV 等）。
    pub fn selection_has_deployable(&self) -> bool {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return false;
        };
        self.local
            .selected
            .iter()
            .any(|&id| game.deploy_target_of(id).is_some())
    }

    /// 对局指针：边缘滚屏优先，否则按悬停格给出 Select / Move / Attack / Deploy 等。
    pub fn battle_pointer(&self, renderer: &Renderer, window: &Window) -> super::battle_input::BattlePointer {
        use super::battle_input::BattlePointer;
        let context = self.battle_pointer_context(renderer, window);
        BattlePointer::resolve(self.edge_scroll_cursor, context)
    }

    /// 战术区悬停上下文（不含边缘滚屏）。
    ///
    /// 部署光标仅在悬停**已选中的可部署单位本身**时出现；移开即回到移动 / 攻击 / 默认。
    fn battle_pointer_context(&self, renderer: &Renderer, window: &Window) -> super::battle_input::BattlePointer {
        use super::battle_input::BattlePointer;
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return BattlePointer::Default;
        };
        let selected = &self.local.selected;
        let Some(cell) = self.cursor_cell(renderer, window)
        else {
            return BattlePointer::Default;
        };
        let vp = self.map_viewport(window);
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);

        if selected.is_empty() {
            if game.pick_local_mobile_near_image(wx, wy, 72.0).is_some()
                || game
                    .pick_structure_at(cell.0, cell.1)
                    .is_some_and(|id| {
                        let local = game
                            .world
                            .players
                            .iter()
                            .find(|p| p.id == game.world.local_player)
                            .map(|p| p.house.as_ref());
                        local.is_some_and(|h| game.world.ecs_owner(id).is_some_and(|o| o.as_ref() == h))
                    })
            {
                return BattlePointer::Select;
            }
            return BattlePointer::Default;
        }

        // 悬停已选中的可部署单位 → 部署光标（移开则不再是部署）。
        if let Some(id) = game.pick_local_mobile_near_image(wx, wy, 72.0) {
            if selected.contains(&id) && game.deploy_target_of(id).is_some() {
                return BattlePointer::Deploy;
            }
        }

        if let Some(target) = game.pick_entity_at(cell.0, cell.1) {
            let hostile = selected.first().and_then(|&atk| {
                let a_owner = game.world.ecs_owner(atk)?;
                let t_owner = game.world.ecs_owner(target)?;
                Some(a_owner != t_owner)
            });
            if hostile == Some(true) {
                return BattlePointer::Attack;
            }
        }

        let passable = game.world.pass_grid.in_bounds(cell.0, cell.1)
            && game.world.pass_grid.is_passable(cell.0, cell.1);
        if passable {
            BattlePointer::Move
        } else {
            BattlePointer::NoMove
        }
    }

    /// 各轴是否还能平移（`pan_screen`：正 dx 减 `center_x`，正 dy 减 `center_y`）。
    fn edge_scroll_can_axes(&self, renderer: &Renderer, proj_w: f32, proj_h: f32) -> (bool, bool, bool, bool) {
        let Some(bounds) = renderer.camera_bounds_for_viewport(proj_w, proj_h)
        else {
            return (true, true, true, true);
        };
        let cam = renderer.camera();
        const EPS: f32 = 0.5;
        let can_west = cam.center_x > bounds.min_center_x + EPS;
        let can_east = cam.center_x < bounds.max_center_x - EPS;
        let can_north = cam.center_y > bounds.min_center_y + EPS;
        let can_south = cam.center_y < bounds.max_center_y - EPS;
        (can_west, can_east, can_north, can_south)
    }

    /// 可玩对局且未暂停 / 未结算时，壳层应捕获光标以支持边缘滚屏。
    pub fn wants_cursor_capture(&self) -> bool {
        self.session
            .as_ref()
            .and_then(|s| s.battle())
            .is_some_and(|g| !g.paused && g.outcome.is_none())
    }

    fn handle_left_click(&mut self, renderer: &Renderer, window: &Window) {
        let add = self.shift_down;
        let vp = self.map_viewport(window);
        if !vp.contains_cursor(self.cursor.0 as i32, self.cursor.1 as i32) {
            if !add {
                self.local.clear();
            }
            return;
        }
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);
        if let Some(type_id) = self.place_mode {
            let Some(cell) = game.image_to_cell(wx, wy)
            else {
                return;
            };
            if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                tracing::info!("放置建筑 {type_id} @({},{})", cell.0, cell.1);
                game.order_place_building(type_id, cell.0, cell.1);
            }
            return;
        }
        let local_house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string());
        let tick = game.world.tick;
        // 先按屏幕锚点点本方单位（VXL 车身常偏离逻辑格），再回退格点选。
        let picked = game.pick_local_mobile_near_image(wx, wy, 72.0).or_else(|| {
            let cell = game.image_to_cell(wx, wy)?;
            if let Some(house) = local_house.as_deref() {
                game.pick_mobile_at_owned(cell.0, cell.1, Some(house)).or_else(|| {
                    game.pick_structure_at(cell.0, cell.1).filter(|&id| game.world.ecs_owner(id).is_some_and(|o| o.as_ref() == house))
                })
            }
            else {
                game.pick_entity_at(cell.0, cell.1)
            }
        });
        let mut pulse = false;
        if let Some(id) = picked {
            let cell = game.world.ecs_transform(id).map(|(x, y, _)| (x, y)).unwrap_or((0, 0));
            let deployable = game.deploy_target_of(id).is_some();
            // 已选中的可部署单位再点一次 → 部署（非双击）。
            let click_deploy = !add
                && deployable
                && self.local.selected.contains(&id);
            if click_deploy {
                tracing::info!("点击部署 · #{} @({},{})", id.0, cell.0, cell.1);
                self.deploy_selection();
                return;
            }
            if add {
                self.local.select_add(game, id);
                tracing::info!("加选实体 #{} @({},{}) · 选中 {:?}", id.0, cell.0, cell.1, self.local.selected);
            }
            else {
                self.local.select_only(game, id);
                tracing::info!("选中实体 #{} @({},{})", id.0, cell.0, cell.1);
            }
            pulse = true;
        }
        else if !add {
            self.local.clear();
            if let Some(cell) = game.image_to_cell(wx, wy) {
                tracing::debug!("点空地 ({},{})，清空选中", cell.0, cell.1);
            }
        }
        if pulse {
            self.pulse_action_lines_at(tick);
        }
    }

    /// 框选：按实体屏幕包围盒与拖拽矩形相交，选中本方可控移动单位。
    fn handle_marquee_select(&mut self, renderer: &Renderer, window: &Window, rect: ScreenRect) {
        let add = self.shift_down;
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let local_house = game
            .world
            .players
            .iter()
            .find(|p| p.id == game.world.local_player)
            .map(|p| p.house.to_string());
        let Some(house) = local_house.as_deref()
        else {
            return;
        };
        let vp = self.map_viewport(window);
        let cam = renderer.camera();
        let mut hits = Vec::new();
        for id in game.world.entity_ids() {
            if game.world.ecs_health(id).is_none_or(|(_, _, dead)| dead) {
                continue;
            }
            if game.world.ecs_owner(id).is_none_or(|o| o.as_ref() != house) {
                continue;
            }
            let Some((_, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some((x, y, _)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            let z = game.world.pass_grid.cell_height(x, y);
            let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
            // 与标记 / `pick_local_mobile_near_image` 同一脚点锚；载具再上移以覆盖 VXL 车身。
            let wx = (sx - game.preview_origin_x) as f32 + 30.0;
            let wy = (sy - game.preview_origin_y) as f32 + 15.0;
            let (cx, cy) = vp.world_to_screen(cam, wx, wy);
            let (hit_cx, hit_cy, half) = match kind {
                MapEntityKind::Infantry => (cx, cy, MARQUEE_HIT_HALF_INFANTRY_PX),
                MapEntityKind::Unit | MapEntityKind::Aircraft => {
                    (cx, cy - MARQUEE_VEHICLE_LIFT_PX, MARQUEE_HIT_HALF_VEHICLE_PX)
                }
                _ => (cx, cy, MARQUEE_HIT_HALF_INFANTRY_PX),
            };
            let hit = ScreenRect::from_center_half(hit_cx, hit_cy, half);
            if rect.intersects(&hit) {
                hits.push(id);
            }
        }
        if hits.is_empty() {
            if !add {
                self.local.clear();
                tracing::debug!("框选落空，清空选中");
            }
            return;
        }
        self.local.apply_ids(game, &hits, add);
        let tick = game.world.tick;
        tracing::info!("框选命中 {} 个 · {:?}", hits.len(), self.local.selected);
        self.pulse_action_lines_at(tick);
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
        let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut())
        else {
            return;
        };
        if game.selection_has_structure(&selected) {
            tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
            game.order_rally(&selected, cell.0, cell.1);
            return;
        }
        let tick = game.world.tick;
        if let Some(target) = game.pick_entity_at(cell.0, cell.1) {
            let hostile = selected
                .first()
                .and_then(|&atk| {
                    let a_owner = game.world.ecs_owner(atk)?;
                    let t_owner = game.world.ecs_owner(target)?;
                    Some(a_owner != t_owner)
                })
                .unwrap_or(false);
            if hostile {
                tracing::info!("命令攻击 → #{}（选中 {:?}）", target.0, selected);
                game.order_attack(&selected, target);
                self.pulse_action_lines_at(tick);
                return;
            }
        }
        tracing::info!("命令移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
        game.order_move(&selected, cell.0, cell.1);
        self.pulse_action_lines_at(tick);
    }

    /// 对局页输入。`accept_commands=false` 时仅允许相机与重开 / 回菜单。
    pub fn handle_event(&mut self, event: &WindowEvent, renderer: &mut Renderer, window: &Window, accept_commands: bool) -> BattleNav {
        match event {
            WindowEvent::ModifiersChanged(mods) => {
                self.shift_down = mods.state().shift_key();
                self.ctrl_down = mods.state().control_key();
                BattleNav::None
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if accept_commands => {
                match state {
                    ElementState::Pressed => {
                        let x = self.cursor.0 as i32;
                        let y = self.cursor.1 as i32;
                        if let Some(BattleHudHit::CommandButton(slot)) = self.hit_hud_at(window, x, y) {
                            self.command_pressed = Some(slot);
                            self.left_gesture = LeftGesture::Idle;
                        } else {
                            self.command_pressed = None;
                            let vp = self.map_viewport(window);
                            if vp.contains_cursor(x, y) {
                                self.left_gesture = LeftGesture::begin(self.cursor.0, self.cursor.1);
                            } else {
                                self.left_gesture = LeftGesture::Idle;
                            }
                        }
                    }
                    ElementState::Released => {
                        let pressed = self.command_pressed.take();
                        if let Some(slot) = pressed {
                            let x = self.cursor.0 as i32;
                            let y = self.cursor.1 as i32;
                            if matches!(
                                self.hit_hud_at(window, x, y),
                                Some(BattleHudHit::CommandButton(s)) if s == slot
                            ) {
                                self.on_command_button(slot);
                            }
                            self.left_gesture = LeftGesture::Idle;
                        } else {
                            let (idle, action) = self.left_gesture.release();
                            self.left_gesture = idle;
                            match action {
                                LeftReleaseAction::None => {}
                                LeftReleaseAction::Click => self.handle_left_click(renderer, window),
                                LeftReleaseAction::Marquee(rect) => {
                                    self.handle_marquee_select(renderer, window, rect)
                                }
                            }
                        }
                    }
                }
                BattleNav::None
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } if !accept_commands => {
                self.left_gesture = LeftGesture::Idle;
                self.command_pressed = None;
                BattleNav::None
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } if accept_commands => {
                self.left_gesture = LeftGesture::Idle;
                self.command_pressed = None;
                self.handle_right_click(renderer, window);
                BattleNav::None
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                // 建造放置模式只认点选，拖拽不升为框选。
                if accept_commands && self.place_mode.is_none() && self.command_pressed.is_none() {
                    self.left_gesture = self.left_gesture.on_cursor_moved(position.x, position.y);
                }
                self.refresh_command_hover(window);
                BattleNav::None
            }
            WindowEvent::MouseWheel { .. } => {
                // 可玩阶段关闭滚轮缩放，避免越界黑边与选点变换漂移。
                BattleNav::None
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return BattleNav::None;
                }
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyR) => {
                        tracing::info!("重开对局…");
                        BattleNav::Rematch
                    }
                    PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) if !accept_commands => {
                        tracing::info!("重开对局…");
                        BattleNav::Rematch
                    }
                    PhysicalKey::Code(KeyCode::KeyL) if !accept_commands => {
                        tracing::info!("结算 · 返回大厅");
                        BattleNav::ToMainMenu
                    }
                    PhysicalKey::Code(KeyCode::Escape) if !accept_commands => BattleNav::ToMainMenu,
                    PhysicalKey::Code(KeyCode::Escape) if accept_commands => {
                        if self.place_mode.is_some() {
                            self.place_mode = None;
                            self.leave_armed = false;
                            tracing::info!("建造模式 · 已关闭");
                            BattleNav::None
                        }
                        else if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            // 对局中 Esc 先暂停；暂停后再 Esc 武装离开，再按一次确认回大厅。
                            // 空格仍可切换暂停并解除武装。
                            if game.paused {
                                if self.leave_armed {
                                    self.leave_armed = false;
                                    BattleNav::ToMainMenu
                                }
                                else {
                                    self.leave_armed = true;
                                    tracing::info!("再按 Esc 确认返回大厅");
                                    BattleNav::None
                                }
                            }
                            else {
                                self.leave_armed = false;
                                game.toggle_pause();
                                tracing::info!("暂停 · {}", game.pause_reason.as_deref().unwrap_or("已暂停"));
                                BattleNav::None
                            }
                        }
                        else {
                            BattleNav::ToMainMenu
                        }
                    }
                    _ if !accept_commands => BattleNav::None,
                    PhysicalKey::Code(KeyCode::KeyA) if self.ctrl_down => {
                        if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                            let seed = self.local.selected.first().copied().or_else(|| {
                                game.world.entity_ids().into_iter().find(|&eid| {
                                    game.world.ecs_health(eid).is_some_and(|(_, _, dead)| !dead)
                                        && game.world.ecs_identity(eid).is_some_and(|(_, kind)| {
                                            matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
                                        })
                                })
                            });
                            if let Some(id) = seed {
                                self.local.select_all_of_owner(game, id);
                                tracing::info!("全选同阵营 · {} 个", self.local.selected.len());
                            }
                        }
                        BattleNav::None
                    }
                    // 镜头平移只用方向键；原版无 WASD 移动，且 D/X 留给部署/警戒。
                    PhysicalKey::Code(KeyCode::ArrowLeft) => {
                        self.pan_world(renderer, window, 48.0, 0.0);
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) => {
                        self.pan_world(renderer, window, -48.0, 0.0);
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::ArrowUp) => {
                        self.pan_world(renderer, window, 0.0, 48.0);
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown) => {
                        self.pan_world(renderer, window, 0.0, -48.0);
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => BattleNav::None,
                    PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => BattleNav::None,
                    PhysicalKey::Code(KeyCode::Tab) => {
                        let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                            let tick = game.world.tick;
                            self.local.cycle_selection(game);
                            tracing::info!("Tab 循环选中 · {:?}", self.local.selected);
                            tick
                        });
                        if let Some(tick) = pulse_tick {
                            self.pulse_action_lines_at(tick);
                        }
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyT) => {
                        let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                            let tick = game.world.tick;
                            self.local.select_same_type(game);
                            tracing::info!("同类型选中 · {} 个 · {:?}", self.local.selected.len(), self.local.selected);
                            tick
                        });
                        if let Some(tick) = pulse_tick {
                            self.pulse_action_lines_at(tick);
                        }
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyF) => {
                        let selected = self.local.selected.clone();
                        let pulse_tick = {
                            let mut out = None;
                            if let Some(&atk) = selected.first() {
                                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                                    if let Some(tgt) = game.nearest_hostile(atk) {
                                        let tick = game.world.tick;
                                        game.order_attack(&selected, tgt);
                                        out = Some(tick);
                                    }
                                }
                            }
                            out
                        };
                        if let Some(tick) = pulse_tick {
                            self.pulse_action_lines_at(tick);
                        }
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        self.deploy_selection();
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyX) => {
                        // 原版：警戒。引擎命令尚未接线，仅占位避免误绑到部署。
                        tracing::info!("警戒 · 尚未接线 · {:?}", self.local.selected);
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyB) => {
                        self.cycle_place_mode();
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            game.toggle_pause();
                            self.leave_armed = false;
                            if game.paused {
                                tracing::info!("暂停 · {}", game.pause_reason.as_deref().unwrap_or("已暂停"));
                            }
                            else {
                                tracing::info!("继续");
                            }
                        }
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyP) => {
                        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            tracing::info!("生产 · E1");
                            game.order_produce("E1");
                        }
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyO) => {
                        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            tracing::info!("生产 · MTNK");
                            game.order_produce("MTNK");
                        }
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyY) => {
                        if let Some(cell) = self.cursor_cell(renderer, window) {
                            let selected = self.local.selected.clone();
                            if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                                tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                                game.order_rally(&selected, cell.0, cell.1);
                            }
                        }
                        BattleNav::None
                    }
                    _ => BattleNav::None,
                }
            }
            _ => BattleNav::None,
        }
    }

    /// 推进仿真（仅对局页调用）并检测是否应进入结算。返回导航与本段耗时。
    pub fn pump(&mut self, dt: f64) -> (BattleNav, std::time::Duration) {
        let started = Instant::now();
        let nav = if let (Some(engine), Some(session)) = (self.engine.as_ref(), self.session.as_mut()) {
            let _ = session.pump(&engine.runtime(), dt);
            if let Some(game) = session.battle() {
                self.local.prune_dead(game);
            }
            if session.battle().and_then(|g| g.outcome.as_ref()).is_some() {
                session.phase = SessionPhase::Finished;
                self.leave_armed = false;
                self.note_outcome_once();
                BattleNav::ToResults
            }
            else {
                BattleNav::None
            }
        }
        else {
            BattleNav::None
        };
        self.resolve_deploy_watch();
        (nav, started.elapsed())
    }

    /// 对当前选中下发部署命令（`D` 键 / 双击 MCV）。
    fn deploy_selection(&mut self) {
        let selected = self.local.selected.clone();
        let Some(&id) = selected.first()
        else {
            return;
        };
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        if game.deploy_target_of(id).is_none() {
            tracing::info!("部署 · 选中不可部署 · {:?}", selected);
            return;
        }
        self.deploy_watch = Some(id);
        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
            tracing::info!("部署选中 · {:?}", selected);
            game.order_deploy(&selected);
        }
    }

    /// 根据权威世界更新部署中 / 完成 / 拒绝状态。
    fn resolve_deploy_watch(&mut self) {
        let Some(id) = self.deploy_watch
        else {
            return;
        };
        let resolved = {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                self.deploy_watch = None;
                return;
            };
            if game
                .world
                .last_rejects()
                .iter()
                .any(|r| matches!(r.reason, ra_engine::CommandRejectReason::CannotDeploy))
            {
                Some(Err(ra_engine::CommandRejectReason::CannotDeploy.as_hud_label().to_string()))
            }
            else {
                match game.world.ecs_identity(id) {
                    Some((type_id, kind)) if matches!(kind, MapEntityKind::Structure) => {
                        Some(Ok(type_id.to_string()))
                    }
                    None => Some(Err("部署目标已消失".into())),
                    _ => None,
                }
            }
        };
        match resolved {
            Some(Ok(type_id)) => {
                tracing::info!("部署完成 · {type_id} · #{id}", id = id.0);
                self.deploy_watch = None;
                if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                    if let (Some((type_id, kind)), Some(owner), Some((x, y, _))) = (
                        game.world.ecs_identity(id),
                        game.world.ecs_owner(id),
                        game.world.ecs_transform(id),
                    ) {
                        if matches!(kind, MapEntityKind::Structure) {
                            self.deploy_visual_queue.push(DeployVisualJob {
                                entity: id,
                                type_id: type_id.to_string(),
                                owner: owner.to_string(),
                                x,
                                y,
                            });
                        }
                    }
                }
            }
            Some(Err(label)) => {
                tracing::info!("部署失败 · {label}");
                self.deploy_watch = None;
            }
            None => {}
        }
    }

    fn note_outcome_once(&mut self) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let Some(BattleOutcome::Victory { owner }) = game.outcome.as_ref()
        else {
            return;
        };
        if self.logged_outcome.as_deref() == Some(owner.as_str()) {
            return;
        }
        self.logged_outcome = Some(owner.clone());
        let stats = game
            .battle_stats
            .as_ref()
            .map(|s| format!(" · {}tick · 损单位{} · 损建筑{} · 花费{}", s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent))
            .unwrap_or_default();
        tracing::info!("对局结束 · 胜方 {owner} · tick={}{stats} · 按 R 重开", game.world.tick);
    }

    /// 绘制当前对局：首帧或空槽全量同步，其后脏集增量。屏上右侧 HUD 由 `HudSnapshot` 驱动。
    pub fn draw_frame(
        &mut self,
        renderer: &mut Renderer,
        window: Option<&Arc<Window>>,
        screen_label: &str,
        fnt: Option<&FntFile>,
        csf: Option<&CsfFile>,
        assets: Option<&GameAssetSource>,
        present: PresentFeel,
    ) {
        enum PendingDraw {
            Full(ra_engine::RenderSnapshot),
            Incremental { tick: u64, dirty: Vec<ra_types::EntityId>, units: Vec<ra_engine::SnapshotUnit> },
        }

        let selected = self.local.selected.clone();
        let force_full = renderer.render_world().unit_count() == 0;
        let prepared = {
            let Some(session) = self.session.as_mut()
            else {
                renderer.draw_frame(None);
                self.refresh_title(renderer, window, screen_label, None);
                return;
            };
            let Some(game) = session.battle_mut()
            else {
                renderer.draw_frame(None);
                self.refresh_title(renderer, window, screen_label, None);
                return;
            };
            let pres_started = Instant::now();
            if force_full {
                let snap = game.snapshot(&selected);
                renderer.timings.presentation_build = Some(pres_started.elapsed());
                let hud = game.snapshot_hud();
                let _ = game.world.take_presentation_dirty();
                (hud, PendingDraw::Full(snap))
            }
            else {
                let dirty = game.world.take_presentation_dirty();
                let units = game.project_units(&dirty);
                let tick = game.world.tick;
                renderer.timings.presentation_build = Some(pres_started.elapsed());
                let hud = game.snapshot_hud();
                (hud, PendingDraw::Incremental { tick, dirty, units })
            }
        };
        let (hud, pending) = prepared;
        self.ensure_battle_hud_chrome(assets);
        self.ensure_order_icons(renderer, assets);
        self.ensure_start_view(renderer);
        let (vw, vh) = window
            .map(|w| {
                let s = w.inner_size();
                (s.width.max(1), s.height.max(1))
            })
            .unwrap_or((800, 600));
        self.sync_world_view(renderer, vw, vh);
        self.tick_deploy_visuals(assets, renderer);
        if self.pending_buildups.is_empty() {
            // 移动单位烤在预览底图上：ECS 变脏时必须重绘，否则 MCV 等 VXL 会停在开局格。
            let mobiles_moved = match &pending {
                PendingDraw::Incremental { dirty, .. } => self.dirty_includes_mobile(dirty),
                PendingDraw::Full(_) => false,
            };
            if mobiles_moved {
                if let Some(assets) = assets {
                    self.rebuild_preview_base_with_mobiles(assets);
                    self.present_preview_base(renderer);
                }
            }
            else {
                self.refresh_structure_anims(renderer);
            }
        }
        self.upload_battle_hud(renderer, &hud, fnt, csf, vw, vh, present);
        renderer.set_action_lines_active(self.action_lines_active());
        match pending {
            PendingDraw::Full(snap) => renderer.draw_frame(Some(&snap)),
            PendingDraw::Incremental { tick, dirty, units } => renderer.draw_incremental(tick, &dirty, &units, &selected),
        }
        self.refresh_title(renderer, window, screen_label, Some(&hud));
    }

    /// 脏集是否含存活移动单位（步兵 / 载具 / 飞行器）。
    fn dirty_includes_mobile(&self, dirty: &[ra_types::EntityId]) -> bool {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return false;
        };
        dirty.iter().any(|&id| {
            game.world.ecs_health(id).is_some_and(|(_, _, dead)| !dead)
                && game
                    .world
                    .ecs_identity(id)
                    .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
        })
    }

    /// 启动 / 推进部署 Buildup，并在播放期间重绘预览（去掉已烤死的 MCV 像素）。
    fn tick_deploy_visuals(&mut self, assets: Option<&GameAssetSource>, renderer: &mut Renderer) {
        let Some(assets) = assets
        else {
            return;
        };
        let had_queue = !self.deploy_visual_queue.is_empty();
        let jobs: Vec<DeployVisualJob> = self.deploy_visual_queue.drain(..).collect();
        let pending_before = self.pending_buildups.len();
        for job in jobs {
            self.begin_deploy_visual(assets, job);
        }
        let started_new = self.pending_buildups.len() > pending_before;
        if self.pending_buildups.is_empty() {
            if had_queue {
                // 无 Buildup 资源时已定格：必须上传底图（活动层可空）。
                self.present_preview_base(renderer);
            }
            return;
        }
        if started_new {
            self.recompose_preview_with_buildups(assets, renderer);
        }
        let mut still = Vec::new();
        let mut finished = Vec::new();
        for pending in self.pending_buildups.drain(..) {
            let elapsed = pending.started.elapsed().as_millis() as u64;
            if pending.clip.frame_at(elapsed).is_none() {
                finished.push(pending);
            }
            else {
                still.push(pending);
            }
        }
        self.pending_buildups = still;
        for done in &finished {
            tracing::info!(
                "部署动画结束 · {} @({},{})",
                done.type_id,
                done.clip.x,
                done.clip.y
            );
            self.settle_deployed_structure(assets, &done.type_id, &done.owner, done.clip.x, done.clip.y, Some(&done.clip));
        }
        if self.pending_buildups.is_empty() {
            self.present_preview_base(renderer);
        }
        else {
            self.recompose_preview_with_buildups(assets, renderer);
        }
    }

    fn begin_deploy_visual(&mut self, assets: &GameAssetSource, job: DeployVisualJob) {
        if self.rules.is_none() {
            tracing::warn!("部署动画 · 无规则快照，直接定格 {}", job.type_id);
            self.settle_deployed_structure(assets, &job.type_id, &job.owner, job.x, job.y, None);
            return;
        }
        let clip = {
            let rules = self.rules.as_ref().expect("rules checked");
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            let lobby = &self.lobby_primaries;
            load_structure_buildup_clip(
                assets,
                &game.world.map,
                self.art_ini,
                &job.type_id,
                &job.owner,
                job.x,
                job.y,
                &|base, owner| remap_owner_palette(rules, Some(lobby), base, owner),
            )
        };
        match clip {
            Some(clip) => {
                tracing::info!(
                    "部署动画 · {} @({},{}) · {}帧 · {}ms/帧",
                    job.type_id,
                    job.x,
                    job.y,
                    clip.frames.len(),
                    clip.rate_ms
                );
                self.pending_buildups.push(PendingBuildup {
                    entity: job.entity,
                    type_id: job.type_id,
                    owner: job.owner,
                    clip,
                    started: Instant::now(),
                });
            }
            None => {
                tracing::warn!("部署动画 · 无 Buildup 资源 {}，尝试直接定格", job.type_id);
                self.settle_deployed_structure(assets, &job.type_id, &job.owner, job.x, job.y, None);
            }
        }
    }

    /// 把已展开建造场烤进 `preview_clean`。主体 SHP 缺失时用 Buildup 末帧。
    fn settle_deployed_structure(
        &mut self,
        assets: &GameAssetSource,
        type_id: &str,
        owner: &str,
        x: u16,
        y: u16,
        clip: Option<&StructureBuildupClip>,
    ) {
        let art_ini = self.art_ini;
        let origin = self.preview_origin;
        let painted = {
            let Some(rules) = self.rules.as_ref()
            else {
                return;
            };
            let Some(clean) = self.preview_clean.as_mut()
            else {
                return;
            };
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            let mut one = game.world.map.clone();
            one.entities.clear();
            one.entities.push(MapEntity {
                kind: MapEntityKind::Structure,
                owner: owner.to_string(),
                type_id: type_id.to_string(),
                health: 256,
                x,
                y,
                facing: 0,
                sub_cell: 0,
            });
            let lobby = &self.lobby_primaries;
            let mut n = paint_structures_onto_rgba(
                assets,
                &one,
                clean,
                origin.0,
                origin.1,
                art_ini,
                &|base, own| remap_owner_palette(rules, Some(lobby), base, own),
            );
            if n == 0 {
                if let Some(clip) = clip {
                    if let Some(last) = clip.frames.len().checked_sub(1) {
                        if paint_structure_buildup_onto_rgba(clean, origin.0, origin.1, clip, last) {
                            n = 1;
                            tracing::info!("定格 · {} Buildup 末帧 #{}", type_id, last);
                        }
                    }
                }
            }
            else {
                tracing::info!("定格 · {} 主体 SHP", type_id);
            }
            if n == 0 {
                tracing::warn!("定格失败 · {} 无主体也无 Buildup 帧，保留原预览", type_id);
                return;
            }
            let bank = collect_structure_anim_bank(assets, &one, art_ini, &|base, own| {
                remap_owner_palette(rules, Some(lobby), base, own)
            });
            (n, bank)
        };
        let (_n, bank) = painted;
        self.structure_anims.layers.extend(bank.layers);
        self.last_anim_sig = u64::MAX;
        // `rules.ini` `[AudioVisual] BuildingSlam=PlaceBuilding`：建造落位 / MCV 展开定格。
        self.pending_battle_sfx.push("PlaceBuilding".into());
        self.rebuild_preview_base_with_mobiles(assets);
    }

    /// `preview_base` = 已定格底图（含展开后的建造场）+ 当前存活移动单位。
    fn rebuild_preview_base_with_mobiles(&mut self, assets: &GameAssetSource) {
        let Some(rules) = self.rules.as_ref()
        else {
            return;
        };
        let Some(clean) = self.preview_clean.as_ref()
        else {
            return;
        };
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let mut mobile_map = game.world.map.clone();
        mobile_map.entities.clear();
        for id in game.world.entity_ids() {
            if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                continue;
            }
            let Some((type_id, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((x, y, facing)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            mobile_map.entities.push(MapEntity {
                kind,
                owner: owner.to_string(),
                type_id: type_id.to_string(),
                health: 256,
                x,
                y,
                facing,
                sub_cell: 0,
            });
        }
        let mut base = clean.clone();
        let lobby = &self.lobby_primaries;
        paint_mobiles_onto_preview_rgba(
            assets,
            &mobile_map,
            &mut base,
            self.preview_origin.0,
            self.preview_origin.1,
            self.art_ini,
            self.rules_ini,
            &|pal, owner| remap_owner_palette(rules, Some(lobby), pal, owner),
        );
        self.preview_base = Some(base);
        self.last_anim_sig = u64::MAX;
    }

    /// Buildup 播放中：干净底图 + 移动单位 + 当前展开帧 + ActiveAnim。
    fn recompose_preview_with_buildups(&mut self, assets: &GameAssetSource, renderer: &mut Renderer) {
        let Some(rules) = self.rules.as_ref()
        else {
            return;
        };
        let Some(clean) = self.preview_clean.as_ref()
        else {
            return;
        };
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let mut mobile_map = game.world.map.clone();
        mobile_map.entities.clear();
        for id in game.world.entity_ids() {
            if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                continue;
            }
            let Some((type_id, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((x, y, facing)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            mobile_map.entities.push(MapEntity {
                kind,
                owner: owner.to_string(),
                type_id: type_id.to_string(),
                health: 256,
                x,
                y,
                facing,
                sub_cell: 0,
            });
        }
        let lobby = self.lobby_primaries.clone();
        let mut composed = clean.clone();
        paint_mobiles_onto_preview_rgba(
            assets,
            &mobile_map,
            &mut composed,
            self.preview_origin.0,
            self.preview_origin.1,
            self.art_ini,
            self.rules_ini,
            &|pal, owner| remap_owner_palette(rules, Some(&lobby), pal, owner),
        );
        for pending in &self.pending_buildups {
            let elapsed = pending.started.elapsed().as_millis() as u64;
            let frame = pending.clip.frame_at(elapsed).unwrap_or(0);
            paint_structure_buildup_onto_rgba(
                &mut composed,
                self.preview_origin.0,
                self.preview_origin.1,
                &pending.clip,
                frame,
            );
        }
        let clock_ms = self.anim_started.elapsed().as_millis() as u64;
        paint_structure_anims_onto_rgba(
            &mut composed,
            self.preview_origin.0,
            self.preview_origin.1,
            &self.structure_anims,
            clock_ms,
        );
        renderer.update_map_preview(composed);
        self.last_anim_sig = u64::MAX;
    }

    /// 上传当前 `preview_base`（可叠活动层）。定格后即使无 ActiveAnim 也必须调用。
    fn present_preview_base(&mut self, renderer: &mut Renderer) {
        let Some(base) = self.preview_base.as_ref()
        else {
            return;
        };
        let mut composed = base.clone();
        if !self.structure_anims.is_empty() {
            let clock_ms = self.anim_started.elapsed().as_millis() as u64;
            paint_structure_anims_onto_rgba(
                &mut composed,
                self.preview_origin.0,
                self.preview_origin.1,
                &self.structure_anims,
                clock_ms,
            );
            self.last_anim_sig = self.structure_anims.frame_signature(clock_ms);
        }
        else {
            self.last_anim_sig = 0;
        }
        renderer.update_map_preview(composed);
    }

    /// 按呈现时钟刷新建筑 ActiveAnim（旗帜 / 泵机），不重置相机。
    fn refresh_structure_anims(&mut self, renderer: &mut Renderer) {
        if self.structure_anims.is_empty() {
            return;
        }
        let Some(base) = self.preview_base.as_ref()
        else {
            return;
        };
        let clock_ms = self.anim_started.elapsed().as_millis() as u64;
        let sig = self.structure_anims.frame_signature(clock_ms);
        if sig == self.last_anim_sig {
            return;
        }
        let mut composed = base.clone();
        paint_structure_anims_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &self.structure_anims, clock_ms);
        renderer.update_map_preview(composed);
        self.last_anim_sig = sig;
    }

    fn local_house_name(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| s.battle())
            .and_then(|g| g.world.players.iter().find(|p| p.id == g.world.local_player))
            .map(|p| p.house.to_string())
    }

    /// 装入 `mouse.shp` 移动 / 攻击 / 部署命令图标（每局一次）。
    fn ensure_order_icons(&mut self, renderer: &mut Renderer, assets: Option<&GameAssetSource>) {
        if self.order_icons_loaded {
            return;
        }
        self.order_icons_loaded = true;
        let Some(source) = assets
        else {
            return;
        };
        match load_battle_order_icons(source) {
            Some(icons) => {
                tracing::info!(
                    "命令图标 · move#{} attack#{} deploy#{} · {}x{}",
                    icons.move_frames.len(),
                    icons.attack_frames.len(),
                    icons.deploy_frames.len(),
                    icons.canvas_w,
                    icons.canvas_h
                );
                renderer.set_order_icons(icons);
            }
            None => tracing::warn!("命令图标装入失败 · 缺少 mouse.shp / mousepal.pal"),
        }
    }

    /// 按本地阵营解码侧栏/底栏 chrome（仅在缺失或换边时重解）。
    fn ensure_battle_hud_chrome(&mut self, assets: Option<&GameAssetSource>) {
        let Some(source) = assets
        else {
            return;
        };
        let Some(side) = self.local_house_name()
        else {
            return;
        };
        if self.hud_chrome.as_ref().is_some_and(|c| c.side == side) {
            return;
        }
        let chrome = decode_battle_hud_chrome(source, &side);
        if chrome.has_sidebar_body() {
            let pal_origin = chrome
                .side1
                .as_ref()
                .or(chrome.side2.as_ref())
                .or(chrome.credits.as_ref())
                .map(|s| s.origin.as_str())
                .unwrap_or("-");
            tracing::info!(
                side = %chrome.side,
                mix = %chrome.mix,
                pal_origin,
                errors = chrome.errors.len(),
                "对局 HUD chrome 已解码"
            );
        }
        else {
            tracing::warn!(
                side = %side,
                mix = %chrome.mix,
                errors = ?chrome.errors,
                "对局 HUD chrome 未解出侧栏主体，回退占位条"
            );
        }
        self.hud_chrome = Some(chrome);
    }

    fn hud_layout_for_window(&self, window: &Window) -> ra_layout::BattleHudLayout {
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        let metrics = self
            .hud_chrome
            .as_ref()
            .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            .unwrap_or_else(BattleHudChromeMetrics::allied);
        battle_hud_layout_with_metrics(w, h, metrics)
    }

    fn hit_hud_at(&self, window: &Window, x: i32, y: i32) -> Option<BattleHudHit> {
        let layout = self.hud_layout_for_window(window);
        hit_at_with_chrome(layout, self.hud_chrome.as_ref(), x, y)
    }

    fn refresh_command_hover(&mut self, window: &Window) {
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        let next = match self.hit_hud_at(window, x, y) {
            Some(BattleHudHit::CommandButton(slot)) => Some(slot),
            _ => None,
        };
        self.command_hover = next;
    }

    fn on_command_button(&mut self, slot: usize) {
        let tip = command_button_csf_tooltip(slot).unwrap_or("?");
        tracing::info!(slot, tip, "命令条按钮");
        // 语义动作（编队 / 警戒 / 路径点等）随后续对局命令接线补齐；此处先保证按下高亮与可点。
    }

    fn upload_battle_hud(
        &self,
        renderer: &mut Renderer,
        hud: &HudSnapshot,
        fnt: Option<&FntFile>,
        csf: Option<&CsfFile>,
        viewport_w: u32,
        viewport_h: u32,
        present: PresentFeel,
    ) {
        let local_house = self.local_house_name();
        let local = local_house.as_ref().and_then(|house| hud.players.iter().find(|p| p.house.as_ref() == house.as_str()));
        let nsel = self.local.selected.len();
        let game = self.session.as_ref().and_then(|s| s.battle());
        let selected_type = self
            .local
            .selected
            .first()
            .copied()
            .and_then(|id| game.and_then(|g| g.world.ecs_identity(id).map(|(t, _)| t.to_string())));
        let selected_summary = match (self.local.selected.first().copied(), nsel, selected_type.as_deref()) {
            (Some(id), n, Some(ty)) if n > 1 => format!("#{}+{} {ty}", id.0, n - 1),
            (Some(id), _, Some(ty)) => format!("#{} {ty}", id.0),
            (Some(id), n, None) if n > 1 => format!("#{}+{}", id.0, n - 1),
            (Some(id), _, None) => format!("#{}", id.0),
            _ => "—".into(),
        };
        let deploy_hint_owned = self
            .local
            .selected
            .first()
            .copied()
            .and_then(|id| game.and_then(|g| g.deploy_target_of(id).map(|t| format!("D→{t}"))));
        let queue = hud.produce_queues.first().map(|q| format!("队列 {}:{}", q.type_id, q.remaining_ticks));
        let reject = hud.last_rejects.first().map(|r| r.reason.as_hud_label());
        let outcome_owned = hud.outcome.as_ref().map(|o| match o {
            BattleOutcome::Victory { owner } => format!("胜 {owner}"),
        });
        let tip_owned = self
            .command_hover
            .and_then(command_button_csf_tooltip)
            .and_then(|key| resolve_csf_text(csf, key));
        let paint = BattleHudModel {
            tick: hud.tick,
            funds: local.map(|p| p.funds).unwrap_or(0),
            power_output: local.map(|p| p.power_output).unwrap_or(0),
            power_drain: local.map(|p| p.power_drain).unwrap_or(0),
            low_power: local.map(|p| p.low_power).unwrap_or(false),
            selected_summary: selected_summary.as_str(),
            deploy_hint: deploy_hint_owned.as_deref(),
            produce_queue: queue.as_deref(),
            reject,
            paused: hud.paused,
            pause_reason: hud.pause_reason.as_deref(),
            outcome: outcome_owned.as_deref(),
            command_pressed: self.command_pressed,
            command_hovered: self.command_hover,
            command_tip: tip_owned.as_deref(),
        };
        // 与命中 / `world_viewport` 同口径：按窗口像素合成，避免 800×600 letterbox 错位。
        let w = viewport_w.max(1);
        let h = viewport_h.max(1);
        if let Some(mut page) = compose_battle_hud_overlay(w, h, fnt, paint, self.hud_chrome.as_ref()) {
            if let Some(rect) = self.left_gesture.marquee_rect() {
                stroke_marquee_rect(&mut page, rect);
            }
            // 与壳层菜单同走 `[present]`，避免对局侧栏仍以满 8-bit 显得过亮。
            let page = ui_present::present_ui_page(page, present);
            renderer.set_ui_overlay(page);
        }
    }

    fn refresh_title(&mut self, renderer: &Renderer, window: Option<&Arc<Window>>, screen_label: &str, hud: Option<&HudSnapshot>) {
        if let Some(window) = window {
            let zoom = renderer.camera().zoom;
            let title = if let Some(hud) = hud {
                let local_house = self
                    .session
                    .as_ref()
                    .and_then(|s| s.battle())
                    .and_then(|g| g.world.players.iter().find(|p| p.id == g.world.local_player))
                    .map(|p| p.house.clone());
                let local = local_house.and_then(|house| hud.players.iter().find(|p| p.house == house));
                let econ = local
                    .map(|p| {
                        let low = if p.low_power { "!" } else { "" };
                        format!("${} 电{}/{}{low}", p.funds, p.power_output, p.power_drain)
                    })
                    .unwrap_or_else(|| "$-".into());
                let queue =
                    hud.produce_queues.first().map(|q| format!("q:{}:{}", q.type_id, q.remaining_ticks)).unwrap_or_else(|| "q:-".into());
                let reject = hud.last_rejects.first().map(|r| r.reason.as_hud_label()).unwrap_or("-");
                let place = self.place_mode.unwrap_or("-");
                if screen_label == "results" {
                    let outcome = match hud.outcome.as_ref() {
                        Some(BattleOutcome::Victory { owner }) => format!("胜 {owner}"),
                        _ => "结算".into(),
                    };
                    let stats = hud
                        .battle_stats
                        .as_ref()
                        .map(|s| format!(" · {}tick 损{}u/{}b 花${}", s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent))
                        .unwrap_or_default();
                    format!("{} · [results] · t{} · {outcome}{stats} · Enter/R重开 L/Esc大厅", self.title_base, hud.tick)
                }
                else if let Some(BattleOutcome::Victory { owner }) = hud.outcome.as_ref() {
                    let stats = hud
                        .battle_stats
                        .as_ref()
                        .map(|s| format!(" · {}tick 损{}u/{}b 花${}", s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent))
                        .unwrap_or_default();
                    format!("{} · [{screen_label}] · t{} · 胜 {owner}{stats} · Enter/R重开 L/Esc大厅", self.title_base, hud.tick)
                }
                else if hud.paused {
                    let reason = hud.pause_reason.as_deref().unwrap_or("已暂停");
                    if self.leave_armed {
                        format!("{} · [{screen_label}] · t{} · 暂停 · {reason} · 再按 Esc 确认回大厅 · Space继续", self.title_base, hud.tick)
                    }
                    else {
                        format!("{} · [{screen_label}] · t{} · 暂停 · {reason} · Esc离开 Space继续", self.title_base, hud.tick)
                    }
                }
                else if self.place_mode.is_some() {
                    let nsel = self.local.selected.len();
                    let sel = self.local.selected.first().copied();
                    let sel_part = match (sel, nsel) {
                        (Some(id), n) if n > 1 => format!("#{}+{}", id.0, n - 1),
                        (Some(id), _) => format!("#{}", id.0),
                        (None, _) => "#-".into(),
                    };
                    let diff = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.difficulty.as_str()).unwrap_or("Normal");
                    format!(
                        "{} · [{screen_label}] · t{} · {econ} · {queue} · 建:{place} · {reject} · {sel_part} · diff={diff} · Esc取消建造 · z{:.2}",
                        self.title_base, hud.tick, zoom
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
                    let diff = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.difficulty.as_str()).unwrap_or("Normal");
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
        if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
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
            super::test_boot::write_status(path, session, &self.local.selected, screen_label, self.leave_armed);
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

    /// 取出待播对局短音效事件 id（壳层按 `sound.ini` → `audio.bag` 播放）。
    pub fn take_pending_battle_sfx(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_battle_sfx)
    }
}

/// 在 HUD 叠加层上描框选矩形（半透明黄绿边）。
fn stroke_marquee_rect(page: &mut RgbaImage, rect: ScreenRect) {
    let w = page.width() as i32;
    let h = page.height() as i32;
    if w <= 0 || h <= 0 || rect.w < 1.0 || rect.h < 1.0 {
        return;
    }
    let x0 = rect.x.floor() as i32;
    let y0 = rect.y.floor() as i32;
    let x1 = (rect.x + rect.w).ceil() as i32;
    let y1 = (rect.y + rect.h).ceil() as i32;
    let color = [180u8, 255, 60, 220];
    let put = |img: &mut RgbaImage, x: i32, y: i32| {
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        let i = ((y as u32 * img.width() + x as u32) * 4) as usize;
        let px = img.as_mut();
        px[i] = color[0];
        px[i + 1] = color[1];
        px[i + 2] = color[2];
        px[i + 3] = color[3];
    };
    for x in x0..=x1 {
        put(page, x, y0);
        put(page, x, y1);
        if y0 + 1 < y1 {
            put(page, x, y0 + 1);
            put(page, x, y1 - 1);
        }
    }
    for y in y0..=y1 {
        put(page, x0, y);
        put(page, x1, y);
        if x0 + 1 < x1 {
            put(page, x0 + 1, y);
            put(page, x1 - 1, y);
        }
    }
}
