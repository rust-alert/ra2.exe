//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use ra_adaptor::RulesSystem;
use ra_assets::{CsfFile, FntFile, IniDocument, Rgba};
use ra_widgets::{
    battle_hud::{
        BattleCameoPaint, BattleHudChrome, BattleHudHit, decode_battle_hud_chrome_with, decode_cameo_sprite,
        hit_at_with_chrome,
    },
    battle_order_icons::load_battle_order_icons,
    battle_pause_menu::{self, BattlePauseChrome, BattlePauseMenuHit},
    compose::{
        blit_rgba, BattleHudModel, compose_battle_hud_overlay, compose_battle_pause_menu_overlay,
    },
    fs_source::GameAssetSource,
    render::present,
    skin::decode::DecodedUiSprite,
    skin::text::{command_button_csf_tooltip, resolve_csf_text},
};
use ra_engine::{
    BattleCapabilitiesSnapshot, CapabilityItem, Engine, HudSnapshot, BattleOutcome, Session, SessionPhase,
    CELL_MOVE_COST, terrain_spawner_frame_signature,
};
use ra_layout::{
    cameo_visible_slot_count, rect_px_from_snapshot, solve_battle_hud_with_metrics,
    BattleHudChromeMetrics, MapViewport, SIDEBAR_TAB_COUNT,
};
use ra_map::{
    MapEntity, MapEntityKind, MobilePaintPose, OverlayLayerFilter, StructureAnimBank, StructureBuildupClip, TerrainAnimBank,
    Theater, WeatherParticleField, collect_structure_anim_bank, iso_to_screen, load_structure_buildup_clip,
    local_size_preview_rect, paint_mobiles_onto_preview_rgba, paint_ore_tree_frames_onto_rgba,
    paint_overlays_onto_preview_rgba, paint_structure_anims_onto_rgba, paint_structure_buildup_onto_rgba,
    paint_structures_onto_rgba, paint_terrain_anims_onto_rgba,
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
        edge_scroll_axes, edge_scroll_cursor_for, edge_scroll_screen_delta, keyboard_pan_screen_delta,
        CameraPanKeys, EdgeScrollCursor, LeftGesture, LeftReleaseAction, ScreenRect,
        EDGE_SCROLL_MARGIN_PX, EDGE_SCROLL_SPEED_PX_PER_SEC, KEYBOARD_PAN_SPEED_PX_PER_SEC,
        MARQUEE_HIT_HALF_INFANTRY_PX, MARQUEE_HIT_HALF_VEHICLE_PX, MARQUEE_VEHICLE_LIFT_PX,
    },
    local_player::LocalPlayerController,
};

/// 遭遇战开局默认缩放（1 屏幕像素 ≈ 1 预览像素；禁止整图 fit）。
const BATTLE_START_ZOOM: f32 = 1.0;
/// 选中行动线可见时长（仿真 tick，对齐原版约 25 帧窗口）。
const ACTION_LINES_DURATION_TICKS: u64 = 25;

/// 由权威移动状态计算步兵/载具烤图姿态（含格内滑移偏移）。
///
/// `tick_fraction` 为距下一逻辑 tick 的进度，用于在渲染帧之间继续滑移，避免整格瞬移。
fn mobile_paint_pose_for(
    game: &ra_engine::BattleSession,
    id: EntityId,
    cell_x: u16,
    cell_y: u16,
    tick_fraction: f64,
) -> MobilePaintPose {
    let anim_frame = game.world.ecs_animation(id).map(|(f, _)| f).unwrap_or(0);
    let moving = game
        .world
        .ecs_move_destination(id)
        .is_some_and(|(dx, _)| dx.is_some())
        || game.world.ecs_path(id).is_some_and(|p| !p.is_empty());
    let (offset_x, offset_y) = if moving {
        let accum = game.world.ecs_move_accum(id).unwrap_or(0);
        let speed = game.world.ecs_speed(id).unwrap_or(0);
        let path = game.world.ecs_path(id).unwrap_or_default();
        slide_offset_along_path(
            cell_x,
            cell_y,
            &path,
            accum,
            speed,
            tick_fraction,
            CELL_MOVE_COST,
            |x, y| game.world.pass_grid.cell_height(x, y),
        )
    } else {
        (0, 0)
    };
    MobilePaintPose {
        anim_frame,
        moving,
        offset_x,
        offset_y,
    }
}

/// 沿路径用 `move_accum + speed * tick_fraction` 计算相对当前逻辑格的屏幕像素偏移。
fn slide_offset_along_path(
    cell_x: u16,
    cell_y: u16,
    path: &[(u16, u16)],
    move_accum: u32,
    speed: u32,
    tick_fraction: f64,
    cell_cost: u32,
    cell_z: impl Fn(u16, u16) -> u8,
) -> (i32, i32) {
    if cell_cost == 0 || path.is_empty() {
        return (0, 0);
    }
    let cost = cell_cost as f32;
    let mut visual = move_accum as f32 + speed as f32 * (tick_fraction as f32).clamp(0.0, 1.0);
    let mut from_x = cell_x;
    let mut from_y = cell_y;
    let mut path_i = 0usize;
    // 预测跨越的整格：逻辑格仍停在 from，呈现滑到后续路点。
    while visual >= cost && path_i + 1 < path.len() {
        visual -= cost;
        let (nx, ny) = path[path_i];
        from_x = nx;
        from_y = ny;
        path_i += 1;
    }
    let Some(&(nx, ny)) = path.get(path_i)
    else {
        return (0, 0);
    };
    let t = (visual / cost).clamp(0.0, 1.0);
    let z0 = cell_z(from_x, from_y);
    let z1 = cell_z(nx, ny);
    let (sx0, sy0) = iso_to_screen(i32::from(from_x), i32::from(from_y), z0);
    let (sx1, sy1) = iso_to_screen(i32::from(nx), i32::from(ny), z1);
    // 偏移相对实体逻辑格（cell_x/y）的屏幕原点，而非预测 from。
    let z_logic = cell_z(cell_x, cell_y);
    let (sx_logic, sy_logic) = iso_to_screen(i32::from(cell_x), i32::from(cell_y), z_logic);
    let sx = sx0 as f32 + (sx1 - sx0) as f32 * t;
    let sy = sy0 as f32 + (sy1 - sy0) as f32 * t;
    ((sx - sx_logic as f32).round() as i32, (sy - sy_logic as f32).round() as i32)
}

#[cfg(test)]
mod slide_offset_tests {
    use super::slide_offset_along_path;

    #[test]
    fn slide_offset_moves_toward_next_cell() {
        // 等距邻格：进度一半时应有明显非零偏移（非整格瞬移）。
        let path = [(2u16, 1u16)];
        let (ox0, oy0) = slide_offset_along_path(1, 1, &path, 0, 0, 0.0, 64, |_, _| 0);
        assert_eq!((ox0, oy0), (0, 0));
        let (ox, oy) = slide_offset_along_path(1, 1, &path, 32, 0, 0.0, 64, |_, _| 0);
        assert!(ox != 0 || oy != 0, "mid-cell slide must leave cell origin");
        let (ox1, oy1) = slide_offset_along_path(1, 1, &path, 64, 0, 0.0, 64, |_, _| 0);
        let (ox_half, oy_half) = (ox, oy);
        assert!(ox1.abs() >= ox_half.abs() || oy1.abs() >= oy_half.abs());
    }

    #[test]
    fn tick_fraction_extends_slide_between_logic_ticks() {
        let path = [(2u16, 1u16)];
        let (a, b) = slide_offset_along_path(1, 1, &path, 0, 32, 0.0, 64, |_, _| 0);
        let (c, d) = slide_offset_along_path(1, 1, &path, 0, 32, 0.5, 64, |_, _| 0);
        assert_eq!((a, b), (0, 0));
        assert!(c != 0 || d != 0, "render fraction must advance slide without waiting for next logic tick");
    }
}

/// 对局控制器向外壳报告的导航意图（外壳改 `AppScreen`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleNav {
    /// 无导航。
    None,
    /// 战役续关：胜用 `NextMission`，败用 `AlternateNextMission`（无则回选边）。
    ContinueCampaign,
    /// 对局已结束，应切到结算页。
    ToResults,
    /// 离开对局/结算：战役回选边，遭遇战回大厅。
    ToMainMenu,
    /// 暂停菜单打开选项页（对局保持暂停，接受/取消后回到对局）。
    OpenOptions,
    /// 切换无边框全屏。
    ToggleFullscreen,
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
    /// 建造放置模式（建筑类型键）。
    place_mode: Option<String>,
    /// 侧栏修理工具是否激活（与出售互斥；激活时贴按下帧）。
    repair_mode: bool,
    /// 侧栏出售工具是否激活（与修理互斥；激活时贴按下帧）。
    sell_mode: bool,
    /// 侧栏分类页签（0=建筑 / 1=防御 / 2=步兵 / 3=载具）。
    sidebar_tab: usize,
    /// 当前页签 cameo 列表滚动起点（可视槽 0 对应的条目下标）。
    cameo_scroll: usize,
    /// 侧栏按下（页签 / cameo），松手命中一致时生效。
    sidebar_pressed: Option<BattleHudHit>,
    /// 建造栏图标缓存（按类型键；`None` 表示已尝试但缺图，避免每帧重解）。
    cameo_cache: HashMap<String, Option<DecodedUiSprite>>,
    /// 测试旁路：曾表示「再按 Esc 回大厅」武装态；现由暂停菜单「放弃」离开，恒为 false。
    leave_armed: bool,
    /// 对局 Esc 暂停菜单阵营素材（`radar` / `sidebttn`，跟本地 house）。
    pause_menu_chrome: Option<BattlePauseChrome>,
    /// 是否已尝试解码暂停菜单（避免每帧重试；换边时清掉重解）。
    pause_menu_tried_side: Option<String>,
    /// 暂停菜单悬停入口 id。
    pause_hover: Option<&'static str>,
    /// 暂停菜单按下入口 id。
    pause_pressed: Option<&'static str>,
    /// 标题用版本短名。
    title_base: String,
    /// 测试状态旁路文件。
    status_path: Option<PathBuf>,
    /// 测试场景名（重开用；当前由外壳 `LoadJob` 持有同名副本）。
    #[allow(dead_code)]
    test_scene: Option<String>,
    /// 局内 HUD chrome（按本地阵营缓存；换边或重开时刷新）。
    hud_chrome: Option<BattleHudChrome>,
    /// rules `Side=`（如 `ThirdSide`）；模组未知国名时与 house 一起选 UI chrome。
    ui_faction_side: Option<String>,
    /// 已解析的壳层 chrome（优先 rules Side 段 `MixFileIndex` / 结算键）。
    ui_faction_chrome: Option<ra_widgets::skirmish_setup::UiFactionChrome>,
    /// 是否已尝试装入 `mouse.shp` 命令图标。
    order_icons_loaded: bool,
    /// 命令条悬停槽。
    command_hover: Option<usize>,
    /// 命令条按下槽（高亮）。
    command_pressed: Option<usize>,
    /// 不含建筑/地形活动层的预览底图（可含开局移动单位与已定格建造场）。
    preview_base: Option<RgbaImage>,
    /// 无开局移动单位、可烘焙已定格动态建筑的底图。
    preview_clean: Option<RgbaImage>,
    /// 无可采矿的定格底图（产矿/采集脏刷新）。
    preview_ore_underlay: Option<RgbaImage>,
    /// 建筑活动层银行。
    structure_anims: StructureAnimBank,
    /// 动画地形物件银行（旗帜等常循环）。
    terrain_anims: TerrainAnimBank,
    /// 矿柱帧银行（由产矿状态机选帧）。
    ore_tree_anims: TerrainAnimBank,
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
    /// 对局短音效 / EVA 事件 id 队列（如 `PlaceBuilding`、`EVA_UnitLost`；由壳层播放）。
    pending_battle_sfx: Vec<String>,
    /// 本机低电 EVA 已闩住（恢复供电后清闩，再掉电才再播）。
    eva_low_power_latched: bool,
    /// 本机基地遇袭 EVA 已闩住（本地建筑无 `hit_flash` 后清闩）。
    eva_base_under_attack_latched: bool,
    /// 已观测到的本机存活机动单位（用于阵亡边沿 → `EVA_UnitLost`）。
    eva_alive_local_mobiles: HashSet<EntityId>,
    /// 是否已用当前存活集播种（首帧只建集、不播报）。
    eva_alive_seeded: bool,
    /// 上一帧本机工厂仍在生产的实体 id（队列清空边沿 → `EVA_UnitReady`）。
    eva_producing_factories: HashSet<EntityId>,
    /// 生产观测是否已播种（首帧只建集、不播报）。
    eva_producing_seeded: bool,
    /// 已见过的侧栏可建造 / 可生产类型（集合增大 → `EVA_NewConstructionOptions`）。
    eva_known_options: HashSet<String>,
    /// 建造选项集是否已播种。
    eva_options_seeded: bool,
    /// 胜负已定后的结算延迟截止（先播 EVA，再 `ToResults`）。
    outcome_hold_until: Option<Instant>,
    /// 当前边缘滚屏光标（整窗边缘；右栏 / 命令条有效）。
    edge_scroll_cursor: EdgeScrollCursor,
    /// 方向键按住状态（渲染帧推进镜头，不跟逻辑 tick / OS 按键重复）。
    camera_pan_keys: CameraPanKeys,
    /// 选中行动线计时起点（仿真 tick；`None` 表示未启动）。
    action_lines_start_tick: Option<u64>,
    /// 当前地图剧院（壳层挂载剧院 MIX 用）。
    map_theater: Option<Theater>,
    /// 天气氛围粒子（呈现层；雪地剧院默认飘雪）。
    weather: WeatherParticleField,
    /// 天气粒子时钟起点。
    weather_started: Instant,
    /// 上一帧已推进的天气毫秒（避免重复 tick）。
    weather_last_ms: u64,
}

impl BattleController {
    /// 由装载结果构造；可无会话（装载失败时仍占位）。
    pub fn from_boot(boot: BootResult, status_path: Option<PathBuf>, test_scene: Option<String>) -> Self {
        let edition = boot.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.edition.as_str()).unwrap_or("—");
        let has_session = boot.session.as_ref().and_then(|s| s.battle()).is_some();
        let map_theater = boot.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.map.theater);
        let weather = match (map_theater, boot.preview_base.as_ref()) {
            (Some(theater), Some(img)) => WeatherParticleField::for_theater(theater, img.width(), img.height()),
            (Some(theater), None) => WeatherParticleField::for_theater(theater, 1, 1),
            _ => WeatherParticleField::none(),
        };
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
            repair_mode: false,
            sell_mode: false,
            sidebar_tab: 0,
            cameo_scroll: 0,
            sidebar_pressed: None,
            cameo_cache: HashMap::new(),
            leave_armed: false,
            pause_menu_chrome: None,
            pause_menu_tried_side: None,
            pause_hover: None,
            pause_pressed: None,
            title_base: format!("ra2 ({edition})"),
            status_path,
            test_scene,
            hud_chrome: None,
            ui_faction_side: None,
            ui_faction_chrome: None,
            order_icons_loaded: false,
            command_hover: None,
            command_pressed: None,
            preview_base: boot.preview_base,
            preview_clean: boot.preview_clean,
            preview_ore_underlay: boot.preview_ore_underlay,
            structure_anims: boot.structure_anims,
            terrain_anims: boot.terrain_anims,
            ore_tree_anims: boot.ore_tree_anims,
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
            eva_low_power_latched: false,
            eva_base_under_attack_latched: false,
            eva_alive_local_mobiles: HashSet::new(),
            eva_alive_seeded: false,
            eva_producing_factories: HashSet::new(),
            eva_producing_seeded: false,
            eva_known_options: HashSet::new(),
            eva_options_seeded: false,
            outcome_hold_until: None,
            edge_scroll_cursor: EdgeScrollCursor::Default,
            camera_pan_keys: CameraPanKeys::default(),
            action_lines_start_tick: None,
            map_theater,
            weather,
            weather_started: Instant::now(),
            weather_last_ms: 0,
        };
        this.bind_local_start();
        this
    }

    /// 写入 rules `Side=`，并在变更时清空已缓存的侧栏 / 暂停 chrome。
    pub fn set_ui_faction_side(&mut self, faction_id: Option<String>) {
        if self.ui_faction_side == faction_id {
            return;
        }
        self.ui_faction_side = faction_id;
        self.hud_chrome = None;
        self.pause_menu_chrome = None;
        self.pause_menu_tried_side = None;
    }

    /// 写入已解析的 [`ra_widgets::skirmish_setup::UiFactionChrome`]（含任意 MixFileIndex）。
    pub fn set_ui_faction_chrome(&mut self, chrome: Option<ra_widgets::skirmish_setup::UiFactionChrome>) {
        if self.ui_faction_chrome == chrome {
            return;
        }
        self.ui_faction_chrome = chrome;
        self.hud_chrome = None;
        self.pause_menu_chrome = None;
        self.pause_menu_tried_side = None;
    }

    /// 当前壳层 chrome（若已注入）。
    pub fn ui_faction_chrome(&self) -> Option<&ra_widgets::skirmish_setup::UiFactionChrome> {
        self.ui_faction_chrome.as_ref()
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
        self.sync_camera_content_bounds(renderer);
        self.focus_camera_on_local_start(renderer);
        self.start_view_pending = false;
    }

    /// 按 `[Map] LocalSize` 投影设置镜头内容矩形，避免扫到 `Size` 外缘锯齿外。
    fn sync_camera_content_bounds(&self, renderer: &mut Renderer) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            renderer.clear_camera_content_rect();
            return;
        };
        let map = &game.world.map;
        let Some((x0, y0, x1, y1)) = local_size_preview_rect(
            map.size_width,
            map.local_size,
            game.preview_origin_x,
            game.preview_origin_y,
        )
        else {
            renderer.clear_camera_content_rect();
            return;
        };
        let (pw, ph) = match renderer.preview_size_u32() {
            Some(s) => s,
            None => {
                renderer.clear_camera_content_rect();
                return;
            }
        };
        // 与预览画布求交，避免越界内容矩形。
        let x0 = x0.max(0).min(pw.saturating_sub(1) as i32);
        let y0 = y0.max(0).min(ph.saturating_sub(1) as i32);
        let x1 = x1.max(x0 + 1).min(pw as i32);
        let y1 = y1.max(y0 + 1).min(ph as i32);
        renderer.set_camera_content_rect(x0 as f32, y0 as f32, x1 as f32, y1 as f32);
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
        self.sidebar_tab = 0;
        self.cameo_scroll = 0;
        self.sidebar_pressed = None;
        self.cameo_cache.clear();
        self.leave_armed = false;
        self.pause_menu_chrome = None;
        self.pause_menu_tried_side = None;
        self.pause_hover = None;
        self.pause_pressed = None;
        self.last_pump = Instant::now();
        self.hud_chrome = None;
        self.order_icons_loaded = false;
        self.command_hover = None;
        self.command_pressed = None;
        self.preview_base = boot.preview_base;
        self.preview_clean = boot.preview_clean;
        self.preview_ore_underlay = boot.preview_ore_underlay;
        self.structure_anims = boot.structure_anims;
        self.terrain_anims = boot.terrain_anims;
        self.ore_tree_anims = boot.ore_tree_anims;
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
        self.eva_low_power_latched = false;
        self.eva_base_under_attack_latched = false;
        self.eva_alive_local_mobiles.clear();
        self.eva_alive_seeded = false;
        self.eva_producing_factories.clear();
        self.eva_producing_seeded = false;
        self.eva_known_options.clear();
        self.eva_options_seeded = false;
        self.outcome_hold_until = None;
        self.edge_scroll_cursor = EdgeScrollCursor::Default;
        self.action_lines_start_tick = None;
        self.map_theater = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.map.theater);
        self.weather = match (self.map_theater, self.preview_base.as_ref()) {
            (Some(theater), Some(img)) => WeatherParticleField::for_theater(theater, img.width(), img.height()),
            (Some(theater), None) => WeatherParticleField::for_theater(theater, 1, 1),
            _ => WeatherParticleField::none(),
        };
        self.weather_started = Instant::now();
        self.weather_last_ms = 0;
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

    /// 镜头平移：整窗边缘滚屏 + 方向键按住连续平移（均按真实 `dt`，与逻辑 tick 无关）。
    pub fn tick_edge_scroll(&mut self, renderer: &mut Renderer, window: &Window, dt: f64, enabled: bool) {
        if !enabled || dt <= 0.0 {
            self.edge_scroll_cursor = EdgeScrollCursor::Default;
            return;
        }
        if self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.paused || g.outcome.is_some()) {
            self.edge_scroll_cursor = EdgeScrollCursor::Default;
            self.camera_pan_keys.clear();
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
        let (kx, ky) = keyboard_pan_screen_delta(self.camera_pan_keys, KEYBOARD_PAN_SPEED_PX_PER_SEC, dt);
        dx += kx;
        dy += ky;
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
                || game.pick_local_structure_near_image(wx, wy, 120.0).is_some()
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
        if let Some(type_id) = self.place_mode.clone() {
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
        // 先屏幕距离点本方单位 / 建筑（VXL·SHP 常偏离逻辑格），再回退格点选。
        let picked = game
            .pick_local_mobile_near_image(wx, wy, 72.0)
            .or_else(|| game.pick_local_structure_near_image(wx, wy, 120.0))
            .or_else(|| {
                let cell = game.image_to_cell(wx, wy)?;
                if let Some(house) = local_house.as_deref() {
                    game.pick_mobile_at_owned(cell.0, cell.1, Some(house)).or_else(|| {
                        game.pick_structure_at(cell.0, cell.1)
                            .filter(|&id| game.world.ecs_owner(id).is_some_and(|o| o.as_ref() == house))
                    })
                } else {
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

    fn handle_right_click(&mut self, renderer: &Renderer, window: &Window) {
        if self.place_mode.take().is_some() {
            tracing::info!("建造模式 · 已关闭");
            return;
        }
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
                let is_structure = game
                    .world
                    .ecs_identity(target)
                    .is_some_and(|(_, kind)| kind == MapEntityKind::Structure);
                if is_structure
                    && game.selection_has_engineer(&selected)
                    && game.is_capturable_structure(target)
                {
                    tracing::info!("命令占领 → #{}（选中 {:?}）", target.0, selected);
                    game.order_capture_building(&selected, target);
                } else if is_structure && game.selection_has_agent(&selected) {
                    tracing::info!("命令渗透 → #{}（选中 {:?}）", target.0, selected);
                    game.order_infiltrate(&selected, target);
                } else {
                    tracing::info!("命令攻击 → #{}（选中 {:?}）", target.0, selected);
                    game.order_attack(&selected, target);
                }
                self.pulse_action_lines_at(tick);
                return;
            }
        }
        tracing::info!("命令移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
        game.order_move(&selected, cell.0, cell.1);
        self.pulse_action_lines_at(tick);
    }

    /// 对局页输入。`accept_commands=false`（结算）时仅允许确认离开 / 战役下一关。
    pub fn handle_event(&mut self, event: &WindowEvent, renderer: &mut Renderer, window: &Window, accept_commands: bool) -> BattleNav {
        let battle_paused = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.paused);
        let has_outcome = self
            .session
            .as_ref()
            .and_then(|s| s.battle())
            .is_some_and(|g| g.outcome.is_some());
        // EVA 播报窗口：仍在 Battle 页，但不再接受对局/暂停输入。
        if accept_commands && has_outcome {
            if let WindowEvent::CursorMoved { position, .. } = event {
                self.cursor = (position.x, position.y);
            }
            return BattleNav::None;
        }
        match event {
            WindowEvent::ModifiersChanged(mods) => {
                self.shift_down = mods.state().shift_key();
                self.ctrl_down = mods.state().control_key();
                BattleNav::None
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if accept_commands && battle_paused => {
                self.handle_pause_menu_mouse(*state, window)
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if accept_commands => {
                let mut nav = BattleNav::None;
                match state {
                    ElementState::Pressed => {
                        let x = self.cursor.0 as i32;
                        let y = self.cursor.1 as i32;
                        self.sidebar_pressed = None;
                        match self.hit_hud_at(window, x, y) {
                            Some(BattleHudHit::CommandButton(slot)) => {
                                self.command_pressed = Some(slot);
                                self.left_gesture = LeftGesture::Idle;
                            }
                            Some(
                                hit @ (BattleHudHit::SidebarTab(_)
                                | BattleHudHit::Cameo(_)
                                | BattleHudHit::Repair
                                | BattleHudHit::Sell
                                | BattleHudHit::Options
                                | BattleHudHit::Diplomacy),
                            ) => {
                                self.command_pressed = None;
                                self.sidebar_pressed = Some(hit);
                                self.left_gesture = LeftGesture::Idle;
                            }
                            None => {
                                self.command_pressed = None;
                                let vp = self.map_viewport(window);
                                if vp.contains_cursor(x, y) {
                                    self.left_gesture = LeftGesture::begin(self.cursor.0, self.cursor.1);
                                } else {
                                    self.left_gesture = LeftGesture::Idle;
                                }
                            }
                        }
                    }
                    ElementState::Released => {
                        let pressed_cmd = self.command_pressed.take();
                        let pressed_side = self.sidebar_pressed.take();
                        if let Some(slot) = pressed_cmd {
                            let x = self.cursor.0 as i32;
                            let y = self.cursor.1 as i32;
                            if matches!(
                                self.hit_hud_at(window, x, y),
                                Some(BattleHudHit::CommandButton(s)) if s == slot
                            ) {
                                self.on_command_button(slot);
                            }
                            self.left_gesture = LeftGesture::Idle;
                        } else if let Some(hit) = pressed_side {
                            let x = self.cursor.0 as i32;
                            let y = self.cursor.1 as i32;
                            if self.hit_hud_at(window, x, y) == Some(hit) {
                                nav = self.on_sidebar_hit(hit);
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
                nav
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } if !accept_commands => {
                self.left_gesture = LeftGesture::Idle;
                self.command_pressed = None;
                self.sidebar_pressed = None;
                self.pause_pressed = None;
                BattleNav::None
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. }
                if accept_commands && !battle_paused =>
            {
                self.left_gesture = LeftGesture::Idle;
                self.command_pressed = None;
                self.sidebar_pressed = None;
                self.handle_right_click(renderer, window);
                BattleNav::None
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                if battle_paused {
                    self.left_gesture = LeftGesture::Idle;
                    self.refresh_pause_hover(window);
                } else {
                    // 建造放置模式只认点选，拖拽不升为框选。
                    if accept_commands
                        && self.place_mode.is_none()
                        && self.command_pressed.is_none()
                        && self.sidebar_pressed.is_none()
                    {
                        self.left_gesture = self.left_gesture.on_cursor_moved(position.x, position.y);
                    }
                    self.refresh_command_hover(window);
                }
                BattleNav::None
            }
            WindowEvent::Focused(false) => {
                self.camera_pan_keys.clear();
                BattleNav::None
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if accept_commands && !battle_paused && self.cursor_over_cameo_band(window) {
                    let steps = match delta {
                        MouseScrollDelta::LineDelta(_, y) => {
                            if *y > 0.0 {
                                -1
                            } else if *y < 0.0 {
                                1
                            } else {
                                0
                            }
                        }
                        MouseScrollDelta::PixelDelta(p) => {
                            if p.y > 0.0 {
                                -1
                            } else if p.y < 0.0 {
                                1
                            } else {
                                0
                            }
                        }
                    };
                    if steps != 0 {
                        self.scroll_cameos(window, steps);
                    }
                }
                BattleNav::None
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // 方向键：记录按住态，由 `tick_edge_scroll` 按渲染帧 `dt` 连续平移（勿跟 OS key-repeat 跳 48px）。
                if let PhysicalKey::Code(code) = event.physical_key {
                    if matches!(
                        code,
                        KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown
                    ) {
                        if !accept_commands || battle_paused {
                            self.camera_pan_keys.clear();
                            return BattleNav::None;
                        }
                        let down = event.state == ElementState::Pressed;
                        match code {
                            KeyCode::ArrowLeft => self.camera_pan_keys.left = down,
                            KeyCode::ArrowRight => self.camera_pan_keys.right = down,
                            KeyCode::ArrowUp => self.camera_pan_keys.up = down,
                            KeyCode::ArrowDown => self.camera_pan_keys.down = down,
                            _ => {}
                        }
                        return BattleNav::None;
                    }
                }
                if event.state != ElementState::Pressed {
                    return BattleNav::None;
                }
                // 暂停菜单打开时：只认 Esc / Space 关闭，吞掉其它对局热键。
                if accept_commands && battle_paused {
                    return match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) | PhysicalKey::Code(KeyCode::Space) => {
                            if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                                game.toggle_pause();
                            }
                            self.clear_pause_menu_input();
                            tracing::info!("继续");
                            BattleNav::None
                        }
                        _ => BattleNav::None,
                    };
                }
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) if !accept_commands => {
                        // 结算确认：战役续关（胜 NextMission / 败 AlternateNextMission）或离开。
                        let continue_campaign = self
                            .session
                            .as_ref()
                            .and_then(|s| s.battle())
                            .is_some_and(|g| {
                                if g.boot_kind != ra_engine::SessionBootKind::Campaign {
                                    return false;
                                }
                                match g.outcome.as_ref() {
                                    Some(ra_engine::BattleOutcome::Victory { .. }) => {
                                        g.world.map.campaign_continue_scenario(true).is_some()
                                    }
                                    Some(ra_engine::BattleOutcome::Defeat { .. }) => {
                                        g.world.map.campaign_continue_scenario(false).is_some()
                                    }
                                    None => false,
                                }
                            });
                        if continue_campaign {
                            tracing::info!("战役继续 · campaign continue scenario");
                            BattleNav::ContinueCampaign
                        } else {
                            tracing::info!("结算确认 · 离开");
                            BattleNav::ToMainMenu
                        }
                    }
                    PhysicalKey::Code(KeyCode::Escape) if !accept_commands => {
                        tracing::info!("结算 · 离开");
                        BattleNav::ToMainMenu
                    }
                    PhysicalKey::Code(KeyCode::Escape) if accept_commands => {
                        if self.place_mode.is_some() {
                            self.place_mode = None;
                            self.clear_pause_menu_input();
                            tracing::info!("建造模式 · 已关闭");
                            BattleNav::None
                        } else if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            // Esc：打开暂停菜单。
                            game.toggle_pause();
                            self.clear_pause_menu_input();
                            tracing::info!("暂停菜单");
                            BattleNav::None
                        } else {
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
                    // 镜头平移只用方向键按住态（见上方 KeyboardInput 分支）；此处不再跳格。
                    PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => BattleNav::None,
                    PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => BattleNav::None,
                    PhysicalKey::Code(KeyCode::Tab) if !battle_paused => {
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
                    PhysicalKey::Code(KeyCode::KeyT) if !battle_paused => {
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
                    PhysicalKey::Code(KeyCode::KeyF) if !battle_paused => {
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
                    PhysicalKey::Code(KeyCode::KeyD) if !battle_paused => {
                        self.deploy_selection();
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyX) if !battle_paused => {
                        self.guard_selection();
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        let paused = if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            game.toggle_pause();
                            game.paused
                        } else {
                            false
                        };
                        self.clear_pause_menu_input();
                        if paused {
                            tracing::info!("暂停菜单");
                        } else if self.session.as_ref().and_then(|s| s.battle()).is_some() {
                            tracing::info!("继续");
                        }
                        BattleNav::None
                    }
                    PhysicalKey::Code(KeyCode::KeyY) if !battle_paused => {
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
        if let (Some(engine), Some(session)) = (self.engine.as_ref(), self.session.as_mut()) {
            let _ = session.pump(&engine.runtime(), dt);
            if let Some(game) = session.battle() {
                self.local.prune_dead(game);
            }
        }
        self.poll_in_battle_eva();
        self.drain_engine_eva_cues();
        let nav = self.poll_outcome_nav();
        self.resolve_deploy_watch();
        (nav, started.elapsed())
    }

    /// 消费引擎 `EvaCue`：仅本机 house 入播报队列。
    fn drain_engine_eva_cues(&mut self) {
        let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut())
        else {
            return;
        };
        let Some(local_house) = game
            .world
            .players
            .iter()
            .find(|p| p.id == game.world.local_player)
            .map(|p| p.house.to_string())
        else {
            return;
        };
        let cues = game.world.take_eva_cues();
        for cue in cues {
            if cue.house.eq_ignore_ascii_case(local_house.as_str()) {
                self.queue_battle_sfx_once(cue.event);
            }
        }
    }

    /// 排队对局音效 / EVA（同 id 未播前不重复入队）。
    fn queue_battle_sfx_once(&mut self, event_id: &str) {
        if event_id.is_empty() {
            return;
        }
        if self.pending_battle_sfx.iter().any(|e| e.eq_ignore_ascii_case(event_id)) {
            return;
        }
        self.pending_battle_sfx.push(event_id.to_string());
    }

    /// 局内 EVA：低电 / 资金不足 / 单位阵亡 / 基地遇袭 / 单位出厂 / 新建造选项。
    ///
    /// 结束播报仍由 [`Self::note_outcome_once`] 排队；本函数在已有胜负时跳过。
    /// 建造完成由 [`Self::settle_deployed_structure`] 另行排队。
    fn poll_in_battle_eva(&mut self) {
        let mut to_queue: Vec<&'static str> = Vec::new();
        let mut next_alive: Option<HashSet<EntityId>> = None;
        let mut seed_alive = false;
        let mut set_low_latch: Option<bool> = None;
        let mut set_base_latch: Option<bool> = None;
        let mut next_producing: Option<HashSet<EntityId>> = None;
        let mut seed_producing = false;
        let mut unit_ready = false;
        let mut next_options: Option<HashSet<String>> = None;
        let mut seed_options = false;
        let mut new_options = false;

        {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            if game.outcome.is_some() {
                return;
            }

            let local_id = game.world.local_player;
            let Some(local) = game.world.players.iter().find(|p| p.id == local_id)
            else {
                return;
            };
            let local_house = local.house.as_ref();
            let low_power = local.low_power();

            if game
                .world
                .last_rejects()
                .iter()
                .any(|r| matches!(r.reason, ra_engine::CommandRejectReason::InsufficientFunds))
            {
                to_queue.push("EVA_InsufficientFunds");
            }

            if !low_power {
                set_low_latch = Some(false);
            } else if !self.eva_low_power_latched {
                set_low_latch = Some(true);
                to_queue.push("EVA_LowPower");
            }

            let mut alive_now: HashSet<EntityId> = HashSet::new();
            let mut producing_now: HashSet<EntityId> = HashSet::new();
            let mut base_hit = false;
            for id in game.world.entity_ids() {
                let Some((_, _, dead)) = game.world.ecs_health(id)
                else {
                    continue;
                };
                if dead {
                    continue;
                }
                let Some(owner) = game.world.ecs_owner(id)
                else {
                    continue;
                };
                if !owner.eq_ignore_ascii_case(local_house) {
                    continue;
                }
                let Some((_, kind)) = game.world.ecs_identity(id)
                else {
                    continue;
                };
                match kind {
                    MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft => {
                        alive_now.insert(id);
                    }
                    MapEntityKind::Structure => {
                        if game
                            .world
                            .ecs_animation(id)
                            .is_some_and(|(_, hit_flash)| hit_flash > 0)
                        {
                            base_hit = true;
                        }
                        if game
                            .world
                            .ecs_produce_item(id)
                            .is_some_and(|item| item.is_some())
                        {
                            producing_now.insert(id);
                        }
                    }
                }
            }

            if !base_hit {
                set_base_latch = Some(false);
            } else if !self.eva_base_under_attack_latched {
                set_base_latch = Some(true);
                to_queue.push("EVA_OurBaseIsUnderAttack");
            }

            let gained_mobile = self.eva_alive_seeded
                && alive_now
                    .iter()
                    .any(|id| !self.eva_alive_local_mobiles.contains(id));
            if !self.eva_alive_seeded {
                next_alive = Some(alive_now);
                seed_alive = true;
            } else {
                let lost = self
                    .eva_alive_local_mobiles
                    .iter()
                    .any(|id| !alive_now.contains(id));
                next_alive = Some(alive_now);
                if lost {
                    to_queue.push("EVA_UnitLost");
                }
            }

            if !self.eva_producing_seeded {
                next_producing = Some(producing_now);
                seed_producing = true;
            } else {
                // 仅「出厂」视为就绪：队列清空且本机机动单位集合出现新 ID。
                // 取消生产也会清队列，但不能播 `EVA_UnitReady`。
                let factory_finished = self
                    .eva_producing_factories
                    .iter()
                    .any(|id| !producing_now.contains(id));
                unit_ready = factory_finished && gained_mobile;
                next_producing = Some(producing_now);
            }

            // 科技/前置解锁使侧栏条目集合变大 → 新建造选项（资金/电力禁用不计入）。
            let caps = game.snapshot_capabilities(&[]);
            let mut options_now: HashSet<String> = HashSet::new();
            for item in caps
                .build_items
                .iter()
                .chain(caps.defense_items.iter())
                .chain(caps.infantry_items.iter())
                .chain(caps.vehicle_items.iter())
            {
                options_now.insert(item.type_id.as_ref().to_string());
            }
            if !self.eva_options_seeded {
                next_options = Some(options_now);
                seed_options = true;
            } else {
                new_options = options_now
                    .iter()
                    .any(|id| !self.eva_known_options.contains(id));
                next_options = Some(options_now);
            }
        }

        if let Some(latch) = set_low_latch {
            self.eva_low_power_latched = latch;
        }
        if let Some(latch) = set_base_latch {
            self.eva_base_under_attack_latched = latch;
        }
        if let Some(alive) = next_alive {
            self.eva_alive_local_mobiles = alive;
            if seed_alive {
                self.eva_alive_seeded = true;
            }
        }
        if let Some(producing) = next_producing {
            self.eva_producing_factories = producing;
            if seed_producing {
                self.eva_producing_seeded = true;
            }
        }
        if let Some(options) = next_options {
            self.eva_known_options = options;
            if seed_options {
                self.eva_options_seeded = true;
            }
        }
        if unit_ready {
            to_queue.push("EVA_UnitReady");
        }
        if new_options {
            to_queue.push("EVA_NewConstructionOptions");
        }
        for event_id in to_queue {
            self.queue_battle_sfx_once(event_id);
        }
    }

    /// 胜负已定：排队 EVA，留在对局页播报后再 `ToResults`。
    fn poll_outcome_nav(&mut self) -> BattleNav {
        let has_outcome = self
            .session
            .as_ref()
            .and_then(|s| s.battle())
            .is_some_and(|g| g.outcome.is_some());
        if !has_outcome {
            return BattleNav::None;
        }
        if let Some(session) = self.session.as_mut() {
            session.phase = SessionPhase::Finished;
        }
        self.leave_armed = false;
        self.begin_outcome_hold();
        match self.outcome_hold_until {
            Some(deadline) if Instant::now() >= deadline => BattleNav::ToResults,
            _ => BattleNav::None,
        }
    }

    /// 首次记录胜负并启动 EVA 播报窗口（幂等）。
    fn begin_outcome_hold(&mut self) {
        self.note_outcome_once();
        if self.outcome_hold_until.is_none() {
            // 原版先播 Battle control terminated / Mission Accomplished，再进积分页。
            // 实际时长由壳层按采样长度 `extend_outcome_hold` 校正。
            self.outcome_hold_until = Some(Instant::now() + Duration::from_millis(2500));
            tracing::info!("胜负已定 · 播报 EVA 后进结算");
        }
    }

    /// 按已播放 EVA 采样时长拉长结算延迟（至少覆盖播完）。
    pub fn extend_outcome_hold(&mut self, sample: &ra_assets::PcmAudio) {
        let ch = sample.channels.max(1) as u64;
        let rate = u64::from(sample.sample_rate.max(1));
        let frames = (sample.samples.len() as u64) / ch;
        let ms = frames.saturating_mul(1000) / rate;
        // 尾音留白，避免切页掐断。
        let hold = Duration::from_millis(ms.saturating_add(400).max(1200));
        let deadline = Instant::now() + hold;
        match self.outcome_hold_until {
            Some(prev) if prev >= deadline => {}
            _ => {
                self.outcome_hold_until = Some(deadline);
                tracing::debug!(ms = hold.as_millis(), "已按 EVA 采样延长结算延迟");
            }
        }
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

    /// 对当前选中下发就地警戒（`X` 键 / 命令条 Guard）。
    fn guard_selection(&mut self) {
        let selected = self.local.selected.clone();
        if selected.is_empty() {
            return;
        }
        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
            tracing::info!("警戒选中 · {:?}", selected);
            game.order_guard(&selected);
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
        let Some(outcome) = game.outcome.as_ref()
        else {
            return;
        };
        let label = match outcome {
            BattleOutcome::Victory { owner } => owner.clone(),
            BattleOutcome::Defeat { reason } => {
                if reason.is_empty() {
                    "defeat".into()
                } else {
                    format!("defeat:{reason}")
                }
            }
        };
        if self.logged_outcome.as_deref() == Some(label.as_str()) {
            return;
        }
        self.logged_outcome = Some(label.clone());
        let stats = game
            .battle_stats
            .as_ref()
            .map(|s| format!(" · {}tick · 损单位{} · 损建筑{} · 花费{}", s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent))
            .unwrap_or_default();
        tracing::info!("对局结束 · {label} · tick={}{stats}", game.world.tick);

        // EVA：放弃/败北播 Battle control terminated；胜利用 Mission Accomplished。
        let eva = match outcome {
            BattleOutcome::Victory { .. } => "EVA_MissionAccomplished",
            BattleOutcome::Defeat { .. } => "EVA_BattleControlTerminated",
        };
        self.queue_battle_sfx_once(eva);
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
        self.ensure_pause_menu_chrome(assets);
        self.ensure_order_icons(renderer, assets);
        self.ensure_cameo_cache(assets);
        self.ensure_start_view(renderer);
        let (vw, vh) = window
            .map(|w| {
                let s = w.inner_size();
                (s.width.max(1), s.height.max(1))
            })
            .unwrap_or((800, 600));
        self.sync_world_view(renderer, vw, vh);
        self.tick_deploy_visuals(assets, renderer);
        let overlay_patched = assets
            .map(|a| self.apply_overlay_paint_dirty(a))
            .unwrap_or(false);
        let structure_patched = assets
            .map(|a| self.apply_structure_paint_dirty(a))
            .unwrap_or(false);
        if self.pending_buildups.is_empty() {
            // 移动单位烤在预览底图上：脏集或仍在滑移时都要重绘（含渲染帧格内插值）。
            let mobiles_moved = match &pending {
                PendingDraw::Incremental { dirty, .. } => self.dirty_includes_mobile(dirty),
                PendingDraw::Full(_) => false,
            };
            let mobiles_sliding = self.any_mobile_sliding();
            if mobiles_moved || mobiles_sliding || overlay_patched || structure_patched {
                if let Some(assets) = assets {
                    self.rebuild_preview_base_with_mobiles(assets);
                    self.present_preview_base(renderer);
                }
            }
            else {
                self.refresh_structure_anims(renderer);
            }
        }
        self.upload_battle_hud(renderer, &hud, fnt, csf, vw, vh, present, screen_label);
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

    /// 是否有移动单位仍在寻路 / 滑移（需每渲染帧重烤以插值格内位置）。
    fn any_mobile_sliding(&self) -> bool {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return false;
        };
        if game.paused || game.outcome.is_some() {
            return false;
        }
        game.world.entity_ids().iter().any(|&id| {
            if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                return false;
            }
            if !game
                .world
                .ecs_identity(id)
                .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
            {
                return false;
            }
            game.world.ecs_path(id).is_some_and(|p| !p.is_empty())
                || game.world.ecs_move_destination(id).is_some_and(|(dx, _)| dx.is_some())
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
        let local_house = self
            .session
            .as_ref()
            .and_then(|s| s.battle())
            .and_then(|g| {
                g.world
                    .players
                    .iter()
                    .find(|p| p.id == g.world.local_player)
                    .map(|p| p.house.to_string())
            });
        if local_house
            .as_deref()
            .is_some_and(|house| owner.eq_ignore_ascii_case(house))
        {
            self.queue_battle_sfx_once("EVA_ConstructionComplete");
        }
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
        mission: String::new(),
        tag: String::new(),
            });
            let lobby = &self.lobby_primaries;
            let mut n = paint_structures_onto_rgba(
                assets,
                &one,
                clean,
                origin.0,
                origin.1,
                art_ini,
                self.rules_ini,
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
            let bank = collect_structure_anim_bank(assets, &one, art_ini, self.rules_ini, &|base, own| {
                remap_owner_palette(rules, Some(lobby), base, own)
            });
            (n, bank)
        };
        let (_n, bank) = painted;
        // underlay 与 clean 同步定格，避免产矿/采集脏刷新丢掉已展开建筑。
        if let (Some(rules), Some(underlay)) = (self.rules.as_ref(), self.preview_ore_underlay.as_mut()) {
            if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
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
                    mission: String::new(),
                    tag: String::new(),
                });
                let lobby = &self.lobby_primaries;
                let mut n = paint_structures_onto_rgba(
                    assets,
                    &one,
                    underlay,
                    origin.0,
                    origin.1,
                    art_ini,
                    self.rules_ini,
                    &|base, own| remap_owner_palette(rules, Some(lobby), base, own),
                );
                if n == 0 {
                    if let Some(clip) = clip {
                        if let Some(last) = clip.frames.len().checked_sub(1) {
                            if paint_structure_buildup_onto_rgba(underlay, origin.0, origin.1, clip, last) {
                                n = 1;
                            }
                        }
                    }
                }
                let _ = n;
            }
        }
        self.structure_anims.layers.extend(bank.layers);
        self.last_anim_sig = u64::MAX;
        // `rules.ini` `[AudioVisual] BuildingSlam=PlaceBuilding`：建造落位 / MCV 展开定格。
        self.pending_battle_sfx.push("PlaceBuilding".into());
        self.rebuild_preview_base_with_mobiles(assets);
    }

    /// 将产矿/采集写入的 overlay 脏格刷回 `preview_clean`。返回是否实际更新。
    ///
    /// 有 underlay 时：`clean = underlay` + 叠全部可采矿（覆盖加矿与扣矿擦除）。
    /// 无 underlay 时回退为仅叠仍存在的脏格。
    fn apply_overlay_paint_dirty(&mut self, assets: &GameAssetSource) -> bool {
        let Some(overlay_types) = self.rules.as_ref().map(|r| r.overlay_types.clone())
        else {
            return false;
        };
        let dirty = self
            .session
            .as_mut()
            .and_then(|s| s.battle_mut())
            .map(|g| g.world.take_overlay_paint_dirty())
            .unwrap_or_default();
        if dirty.is_empty() {
            return false;
        }
        let Some(map) = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.map.clone())
        else {
            return false;
        };
        let harvestable: Vec<_> = map
            .overlays
            .iter()
            .copied()
            .filter(|c| overlay_types.is_harvestable(c.overlay_id))
            .collect();

        if let Some(underlay) = self.preview_ore_underlay.as_ref() {
            let mut clean = underlay.clone();
            let (shp, mark) = paint_overlays_onto_preview_rgba(
                assets,
                &map,
                &harvestable,
                &mut clean,
                self.preview_origin.0,
                self.preview_origin.1,
                self.art_ini,
                self.rules_ini,
                &|id| overlay_types.name(id).map(str::to_owned),
                &|id| overlay_types.is_harvestable(id),
                &|_| None,
                OverlayLayerFilter::Ground,
            );
            let _ = (shp, mark);
            self.preview_clean = Some(clean);
            self.last_anim_sig = u64::MAX;
            return true;
        }

        let cells: Vec<_> = dirty
            .iter()
            .filter_map(|(x, y)| harvestable.iter().find(|c| c.x == *x && c.y == *y).copied())
            .collect();
        if cells.is_empty() {
            return false;
        }
        let Some(clean) = self.preview_clean.as_mut()
        else {
            return false;
        };
        let (shp, mark) = paint_overlays_onto_preview_rgba(
            assets,
            &map,
            &cells,
            clean,
            self.preview_origin.0,
            self.preview_origin.1,
            self.art_ini,
            self.rules_ini,
            &|id| overlay_types.name(id).map(str::to_owned),
            &|id| overlay_types.is_harvestable(id),
            &|_| None,
            OverlayLayerFilter::Ground,
        );
        if shp + mark == 0 {
            return false;
        }
        self.last_anim_sig = u64::MAX;
        true
    }

    /// 将占领等房主变更的建筑按新房主色烤进 `preview_clean` / underlay，并刷新活动层。
    fn apply_structure_paint_dirty(&mut self, assets: &GameAssetSource) -> bool {
        let dirty = self
            .session
            .as_mut()
            .and_then(|s| s.battle_mut())
            .map(|g| g.world.take_structure_paint_dirty())
            .unwrap_or_default();
        if dirty.is_empty() {
            return false;
        }
        let Some(rules) = self.rules.as_ref()
        else {
            return false;
        };
        let jobs: Vec<(String, String, u16, u16)> = {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return false;
            };
            dirty
                .iter()
                .filter_map(|&id| {
                    if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                        return None;
                    }
                    let (type_id, kind) = game.world.ecs_identity(id)?;
                    if kind != MapEntityKind::Structure {
                        return None;
                    }
                    let owner = game.world.ecs_owner(id)?;
                    let (x, y, _) = game.world.ecs_transform(id)?;
                    Some((type_id.to_string(), owner.to_string(), x, y))
                })
                .collect()
        };
        if jobs.is_empty() {
            return false;
        }
        let art_ini = self.art_ini;
        let origin = self.preview_origin;
        let lobby = self.lobby_primaries.clone();
        let mut any = false;
        for (type_id, owner, x, y) in &jobs {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                break;
            };
            let mut one = game.world.map.clone();
            one.entities.clear();
            one.entities.push(MapEntity {
                kind: MapEntityKind::Structure,
                owner: owner.clone(),
                type_id: type_id.clone(),
                health: 256,
                x: *x,
                y: *y,
                facing: 0,
                sub_cell: 0,
                mission: String::new(),
                tag: String::new(),
            });
            let remap = |base: &ra_assets::Palette, own: &str| remap_owner_palette(rules, Some(&lobby), base, own);
            if let Some(clean) = self.preview_clean.as_mut() {
                let n = paint_structures_onto_rgba(
                    assets,
                    &one,
                    clean,
                    origin.0,
                    origin.1,
                    art_ini,
                    self.rules_ini,
                    &remap,
                );
                any |= n > 0;
            }
            if let Some(underlay) = self.preview_ore_underlay.as_mut() {
                let n = paint_structures_onto_rgba(
                    assets,
                    &one,
                    underlay,
                    origin.0,
                    origin.1,
                    art_ini,
                    self.rules_ini,
                    &remap,
                );
                any |= n > 0;
            }
            self.structure_anims.layers.retain(|layer| !(layer.x == *x && layer.y == *y));
            let bank = collect_structure_anim_bank(assets, &one, art_ini, self.rules_ini, &remap);
            self.structure_anims.layers.extend(bank.layers);
        }
        if any {
            self.last_anim_sig = u64::MAX;
        }
        any
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
        let tick_fraction = self.session.as_ref().map(|s| s.tick_fraction()).unwrap_or(0.0);
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let mut mobile_map = game.world.map.clone();
        mobile_map.entities.clear();
        let mut poses: HashMap<(u16, u16, String, String), MobilePaintPose> = HashMap::new();
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
            let owner_s = owner.to_string();
            let type_s = type_id.to_string();
            poses.insert(
                (x, y, type_s.clone(), owner_s.clone()),
                mobile_paint_pose_for(game, id, x, y, tick_fraction),
            );
            mobile_map.entities.push(MapEntity {
                kind,
                owner: owner_s,
                type_id: type_s,
                health: 256,
                x,
                y,
                facing,
                sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
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
            &|ent| {
                poses
                    .get(&(ent.x, ent.y, ent.type_id.clone(), ent.owner.clone()))
                    .copied()
                    .unwrap_or_default()
            },
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
        let tick_fraction = self.session.as_ref().map(|s| s.tick_fraction()).unwrap_or(0.0);
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let mut mobile_map = game.world.map.clone();
        mobile_map.entities.clear();
        let mut poses: HashMap<(u16, u16, String, String), MobilePaintPose> = HashMap::new();
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
            let owner_s = owner.to_string();
            let type_s = type_id.to_string();
            poses.insert(
                (x, y, type_s.clone(), owner_s.clone()),
                mobile_paint_pose_for(game, id, x, y, tick_fraction),
            );
            mobile_map.entities.push(MapEntity {
                kind,
                owner: owner_s,
                type_id: type_s,
                health: 256,
                x,
                y,
                facing,
                sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
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
            &|ent| {
                poses
                    .get(&(ent.x, ent.y, ent.type_id.clone(), ent.owner.clone()))
                    .copied()
                    .unwrap_or_default()
            },
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
        paint_terrain_anims_onto_rgba(
            &mut composed,
            self.preview_origin.0,
            self.preview_origin.1,
            &self.terrain_anims,
            clock_ms,
        );
        self.paint_ore_tree_frames_onto(&mut composed);
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

    /// 上传当前 `preview_base`（可叠活动层与天气粒子）。定格后即使无 ActiveAnim 也必须调用。
    fn present_preview_base(&mut self, renderer: &mut Renderer) {
        let Some(base) = self.preview_base.as_ref()
        else {
            return;
        };
        let mut composed = base.clone();
        let clock_ms = self.anim_started.elapsed().as_millis() as u64;
        let has_anims = self.has_preview_anims();
        if has_anims {
            paint_terrain_anims_onto_rgba(
                &mut composed,
                self.preview_origin.0,
                self.preview_origin.1,
                &self.terrain_anims,
                clock_ms,
            );
            self.paint_ore_tree_frames_onto(&mut composed);
            paint_structure_anims_onto_rgba(
                &mut composed,
                self.preview_origin.0,
                self.preview_origin.1,
                &self.structure_anims,
                clock_ms,
            );
            self.last_anim_sig = self.preview_anim_signature(clock_ms);
        }
        else {
            self.last_anim_sig = 0;
        }
        self.paint_weather_onto(&mut composed);
        renderer.update_map_preview(composed);
    }

    /// 按呈现时钟刷新建筑 ActiveAnim（旗帜 / 泵机）、常循环地形、矿柱状态机帧与天气粒子，不重置相机。
    fn refresh_structure_anims(&mut self, renderer: &mut Renderer) {
        let Some(base) = self.preview_base.as_ref()
        else {
            return;
        };
        let clock_ms = self.anim_started.elapsed().as_millis() as u64;
        let sig = self.preview_anim_signature(clock_ms);
        let weather_active = self.weather.is_active();
        let has_anims = self.has_preview_anims();
        if !weather_active && (!has_anims || sig == self.last_anim_sig) {
            return;
        }
        let mut composed = base.clone();
        if has_anims {
            paint_terrain_anims_onto_rgba(
                &mut composed,
                self.preview_origin.0,
                self.preview_origin.1,
                &self.terrain_anims,
                clock_ms,
            );
            self.paint_ore_tree_frames_onto(&mut composed);
            paint_structure_anims_onto_rgba(
                &mut composed,
                self.preview_origin.0,
                self.preview_origin.1,
                &self.structure_anims,
                clock_ms,
            );
        }
        self.paint_weather_onto(&mut composed);
        renderer.update_map_preview(composed);
        self.last_anim_sig = sig;
    }

    /// 是否有需叠画的活动层（建筑 / 常循环地形 / 矿柱）。
    fn has_preview_anims(&self) -> bool {
        !self.structure_anims.is_empty() || !self.terrain_anims.is_empty() || !self.ore_tree_anims.is_empty()
    }

    /// 从世界矿柱状态机读取当前帧并叠画。
    fn paint_ore_tree_frames_onto(&self, image: &mut RgbaImage) {
        if self.ore_tree_anims.is_empty() {
            return;
        }
        let frames = self.ore_tree_render_frames();
        paint_ore_tree_frames_onto_rgba(
            image,
            self.preview_origin.0,
            self.preview_origin.1,
            &self.ore_tree_anims,
            &frames,
        );
    }

    /// `(格x, 格y, 帧)`；无会话时回退 Idle 帧 0。
    fn ore_tree_render_frames(&self) -> Vec<(u16, u16, u16)> {
        if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
            if !game.world.terrain_spawners.is_empty() {
                return game
                    .world
                    .terrain_spawners
                    .iter()
                    .map(|s| (s.x, s.y, s.render_frame()))
                    .collect();
            }
        }
        self.ore_tree_anims
            .layers
            .iter()
            .map(|layer| (layer.x, layer.y, 0))
            .collect()
    }

    /// 建筑 + 常循环地形 + 矿柱状态机帧签名（用于跳过无变化上传）。
    fn preview_anim_signature(&self, clock_ms: u64) -> u64 {
        let mut h = self.structure_anims.frame_signature(clock_ms);
        h ^= self.terrain_anims.frame_signature(clock_ms).rotate_left(17);
        let spawners = self
            .session
            .as_ref()
            .and_then(|s| s.battle())
            .map(|g| g.world.terrain_spawners.as_slice())
            .unwrap_or(&[]);
        h ^= terrain_spawner_frame_signature(spawners).rotate_left(29);
        h
    }

    /// 按预览尺寸推进并叠画天气粒子。
    fn paint_weather_onto(&mut self, image: &mut RgbaImage) {
        if self.map_theater.is_none() && !self.weather.is_active() {
            return;
        }
        self.weather.resize(image.width(), image.height());
        let now_ms = self.weather_started.elapsed().as_millis() as u64;
        let dt = now_ms.saturating_sub(self.weather_last_ms);
        self.weather_last_ms = now_ms;
        self.weather.tick(dt);
        self.weather.paint_onto(image);
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
        let chrome = decode_battle_hud_chrome_with(
            source,
            &side,
            self.ui_faction_side.as_deref(),
            self.ui_faction_chrome.as_ref(),
        );
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
                faction = ?self.ui_faction_side,
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

    /// 按本地阵营解码暂停菜单素材（换边重解；必须 prefer `sidec*`）。
    fn ensure_pause_menu_chrome(&mut self, assets: Option<&GameAssetSource>) {
        let Some(side) = self.local_house_name()
        else {
            return;
        };
        if self.pause_menu_tried_side.as_deref() == Some(side.as_str()) {
            return;
        }
        self.pause_menu_tried_side = Some(side.clone());
        let Some(source) = assets
        else {
            self.pause_menu_chrome = None;
            return;
        };
        let decoded = battle_pause_menu::decode_battle_pause_chrome_with(
            source,
            &side,
            self.ui_faction_side.as_deref(),
            self.ui_faction_chrome.as_ref(),
        );
        if !decoded.errors.is_empty() {
            tracing::warn!(side = %side, mix = %decoded.mix, errors = ?decoded.errors, "暂停菜单素材有缺口");
        } else {
            tracing::info!(side = %side, mix = %decoded.mix, "暂停菜单素材已解码");
        }
        self.pause_menu_chrome = Some(decoded);
    }

    fn clear_pause_menu_input(&mut self) {
        self.pause_hover = None;
        self.pause_pressed = None;
        self.leave_armed = false;
        self.command_hover = None;
        self.command_pressed = None;
        self.left_gesture = LeftGesture::Idle;
    }

    fn pause_hud_metrics(&self) -> BattleHudChromeMetrics {
        self.hud_chrome
            .as_ref()
            .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            .or_else(|| {
                self.pause_menu_chrome
                    .as_ref()
                    .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            })
            .unwrap_or_else(BattleHudChromeMetrics::sidec01)
    }

    fn refresh_pause_hover(&mut self, window: &Window) {
        let size = window.inner_size();
        let metrics = self.pause_hud_metrics();
        self.pause_hover = battle_pause_menu::hit_at(
            size.width.max(1),
            size.height.max(1),
            metrics,
            self.cursor.0 as i32,
            self.cursor.1 as i32,
        )
        .map(|h| h.entry_id());
    }

    fn handle_pause_menu_mouse(&mut self, state: ElementState, window: &Window) -> BattleNav {
        let size = window.inner_size();
        let metrics = self.pause_hud_metrics();
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        match state {
            ElementState::Pressed => {
                self.pause_pressed =
                    battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, x, y)
                        .map(|h| h.entry_id());
                BattleNav::None
            }
            ElementState::Released => {
                let pressed = self.pause_pressed.take();
                let hit =
                    battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, x, y);
                if pressed.is_some_and(|id| hit.is_some_and(|h| h.entry_id() == id)) {
                    if let Some(hit) = hit {
                        return self.on_pause_menu_hit(hit);
                    }
                }
                BattleNav::None
            }
        }
    }

    fn on_pause_menu_hit(&mut self, hit: BattlePauseMenuHit) -> BattleNav {
        match hit {
            BattlePauseMenuHit::Resume => {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                self.clear_pause_menu_input();
                tracing::info!("继续");
                BattleNav::None
            }
            BattlePauseMenuHit::Abort => {
                self.clear_pause_menu_input();
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    game.apply_scripted_outcome(BattleOutcome::Defeat {
                        reason: "放弃任务".into(),
                    });
                    // 关掉暂停菜单输入路径；仿真仍因 `outcome` 停住。
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                // 留在对局页播 EVA，由 `pump` → `poll_outcome_nav` 延后进结算。
                self.begin_outcome_hold();
                tracing::info!("放弃任务 · 先播报再结算");
                BattleNav::None
            }
            BattlePauseMenuHit::Options => {
                self.clear_pause_menu_input();
                tracing::info!("暂停菜单 · 打开选项");
                BattleNav::OpenOptions
            }
            BattlePauseMenuHit::Fullscreen => {
                tracing::info!("暂停菜单 · 切换全屏");
                BattleNav::ToggleFullscreen
            }
        }
    }

    fn hud_snap_for_window(&self, window: &Window) -> ra_layout::LayoutSnapshot {
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        let metrics = self
            .hud_chrome
            .as_ref()
            .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            .unwrap_or_else(BattleHudChromeMetrics::sidec01);
        solve_battle_hud_with_metrics(w, h, metrics)
    }

    fn hit_hud_at(&self, window: &Window, x: i32, y: i32) -> Option<BattleHudHit> {
        let snap = self.hud_snap_for_window(window);
        let metrics = self
            .hud_chrome
            .as_ref()
            .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            .unwrap_or_else(BattleHudChromeMetrics::sidec01);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let visible = cameo_visible_slot_count(band.h);
        let cameo_count = self.current_tab_cameo_count(visible);
        let hit = hit_at_with_chrome(
            &snap,
            self.hud_chrome.as_ref(),
            metrics.power_w,
            cameo_count,
            x,
            y,
        );
        if let Some(BattleHudHit::SidebarTab(tab)) = hit {
            let tabs_visible = Self::sidebar_tabs_visible(self.current_capabilities().as_ref());
            if !tabs_visible.get(tab).copied().unwrap_or(false) {
                return None;
            }
        }
        hit
    }

    fn tab_items<'a>(caps: &'a BattleCapabilitiesSnapshot, tab: usize) -> &'a [CapabilityItem] {
        match tab.min(SIDEBAR_TAB_COUNT.saturating_sub(1)) {
            0 => caps.build_items.as_slice(),
            1 => caps.defense_items.as_slice(),
            2 => caps.infantry_items.as_slice(),
            3 => caps.vehicle_items.as_slice(),
            _ => &[],
        }
    }

    /// 无对应可建造基础的分类页签不显示。
    ///
    /// Q/W 建筑与防御均依赖建造场；E 步兵依赖兵营；R 载具依赖战车厂。
    fn sidebar_tabs_visible(caps: Option<&BattleCapabilitiesSnapshot>) -> [bool; SIDEBAR_TAB_COUNT] {
        let Some(caps) = caps
        else {
            return [false; SIDEBAR_TAB_COUNT];
        };
        [
            caps.has_construction_yard,
            caps.has_construction_yard,
            caps.has_infantry_factory,
            caps.has_vehicle_factory,
        ]
    }

    /// 当前页签若已无基础，切到第一个仍可见的页签。
    fn sync_sidebar_tab_to_visible(&mut self, visible: [bool; SIDEBAR_TAB_COUNT]) {
        if visible.get(self.sidebar_tab).copied().unwrap_or(false) {
            return;
        }
        let next = visible.iter().position(|&v| v).unwrap_or(0);
        if self.sidebar_tab != next {
            self.sidebar_tab = next;
            self.cameo_scroll = 0;
            if next > 1 {
                self.place_mode = None;
            }
        }
    }

    fn current_capabilities(&self) -> Option<BattleCapabilitiesSnapshot> {
        let game = self.session.as_ref().and_then(|s| s.battle())?;
        Some(game.snapshot_capabilities(&self.local.selected))
    }

    fn current_tab_cameo_count(&self, visible_slots: usize) -> usize {
        let Some(caps) = self.current_capabilities()
        else {
            return 0;
        };
        let total = Self::tab_items(&caps, self.sidebar_tab).len();
        total.min(visible_slots)
    }

    fn clamp_cameo_scroll(&mut self, visible_slots: usize) {
        let Some(caps) = self.current_capabilities()
        else {
            self.cameo_scroll = 0;
            return;
        };
        let total = Self::tab_items(&caps, self.sidebar_tab).len();
        let max_scroll = total.saturating_sub(visible_slots);
        if self.cameo_scroll > max_scroll {
            self.cameo_scroll = max_scroll;
        }
    }

    fn cursor_over_cameo_band(&self, window: &Window) -> bool {
        let snap = self.hud_snap_for_window(window);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        band.w > 0 && band.h > 0 && band.contains(self.cursor.0 as i32, self.cursor.1 as i32)
    }

    fn scroll_cameos(&mut self, window: &Window, steps: i32) {
        let snap = self.hud_snap_for_window(window);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let visible = cameo_visible_slot_count(band.h);
        let next = self.cameo_scroll as i32 + steps;
        self.cameo_scroll = next.max(0) as usize;
        self.clamp_cameo_scroll(visible);
    }

    fn ensure_cameo_cache(&mut self, assets: Option<&GameAssetSource>) {
        let Some(source) = assets
        else {
            return;
        };
        let Some(caps) = self.current_capabilities()
        else {
            return;
        };
        let art = source
            .resolve(self.art_ini)
            .and_then(|hit| IniDocument::parse(&hit.bytes).ok());
        let art_ref = art.as_ref();
        for item in caps
            .build_items
            .iter()
            .chain(caps.defense_items.iter())
            .chain(caps.infantry_items.iter())
            .chain(caps.vehicle_items.iter())
        {
            let key = item.type_id.as_ref();
            if self.cameo_cache.contains_key(key) {
                continue;
            }
            let sprite = decode_cameo_sprite(source, art_ref, key);
            self.cameo_cache.insert(key.to_string(), sprite);
        }
    }

    fn on_sidebar_hit(&mut self, hit: BattleHudHit) -> BattleNav {
        match hit {
            BattleHudHit::SidebarTab(tab) => {
                let tab = tab.min(SIDEBAR_TAB_COUNT.saturating_sub(1));
                let visible = Self::sidebar_tabs_visible(self.current_capabilities().as_ref());
                if !visible.get(tab).copied().unwrap_or(false) {
                    return BattleNav::None;
                }
                if self.sidebar_tab != tab {
                    self.sidebar_tab = tab;
                    self.cameo_scroll = 0;
                    if tab > 1 {
                        self.place_mode = None;
                    }
                    tracing::info!("侧栏页签 · {tab}");
                }
                BattleNav::None
            }
            BattleHudHit::Cameo(slot) => {
                let Some(caps) = self.current_capabilities()
                else {
                    return BattleNav::None;
                };
                let items = Self::tab_items(&caps, self.sidebar_tab);
                let index = self.cameo_scroll.saturating_add(slot);
                let Some(item) = items.get(index)
                else {
                    return BattleNav::None;
                };
                if !item.enabled {
                    if let Some(reason) = item.disabled_reason {
                        tracing::info!(
                            "建造栏不可用 · {} · {}",
                            item.type_id,
                            reason.as_hud_label()
                        );
                    }
                    return BattleNav::None;
                }
                match self.sidebar_tab {
                    0 | 1 => {
                        let type_id = item.type_id.as_ref();
                        if self.place_mode.as_deref() == Some(type_id) {
                            self.place_mode = None;
                            tracing::info!("建造模式 · 已关闭");
                        } else {
                            self.repair_mode = false;
                            self.sell_mode = false;
                            self.place_mode = Some(type_id.to_string());
                            tracing::info!("建造模式 · 放置 {type_id}（点地图落地，右键/Esc 取消）");
                        }
                    }
                    2 | 3 => {
                        let type_id = item.type_id.as_ref().to_string();
                        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            if game.is_local_producing(&type_id) {
                                tracing::info!("取消生产 · {type_id}");
                                game.order_cancel_produce(type_id);
                            } else {
                                tracing::info!("生产 · {type_id}");
                                game.order_produce(type_id);
                            }
                        }
                    }
                    _ => {}
                }
                BattleNav::None
            }
            BattleHudHit::Options => {
                tracing::info!("侧栏 · 打开选项");
                BattleNav::OpenOptions
            }
            BattleHudHit::Repair => {
                self.sell_mode = false;
                self.repair_mode = !self.repair_mode;
                if self.repair_mode {
                    self.place_mode = None;
                }
                tracing::info!(active = self.repair_mode, "侧栏 · 修理工具");
                BattleNav::None
            }
            BattleHudHit::Sell => {
                self.repair_mode = false;
                self.sell_mode = !self.sell_mode;
                if self.sell_mode {
                    self.place_mode = None;
                }
                tracing::info!(active = self.sell_mode, "侧栏 · 出售工具");
                BattleNav::None
            }
            BattleHudHit::Diplomacy => {
                // 外交语义随后续引擎命令接线。
                tracing::info!(?hit, "侧栏按钮 · 尚未接线");
                BattleNav::None
            }
            BattleHudHit::CommandButton(_) => BattleNav::None,
        }
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
        let name = ra_widgets::skin::text::SKIRMISH_COMMAND_BAR
            .get(slot)
            .copied()
            .unwrap_or("?");
        let tip = command_button_csf_tooltip(slot).unwrap_or("?");
        tracing::info!(slot, name, tip, "命令条按钮");
        match name {
            "Deploy" => self.deploy_selection(),
            "Guard" => self.guard_selection(),
            "TypeSelect" => {
                let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                    let tick = game.world.tick;
                    self.local.select_same_type(game);
                    tracing::info!(
                        "同类型选中 · {} 个 · {:?}",
                        self.local.selected.len(),
                        self.local.selected
                    );
                    tick
                });
                if let Some(tick) = pulse_tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            _ => {
                // 编队 / 路径点等随后续对局命令接线补齐。
            }
        }
    }

    fn upload_battle_hud(
        &mut self,
        renderer: &mut Renderer,
        hud: &HudSnapshot,
        fnt: Option<&FntFile>,
        csf: Option<&CsfFile>,
        viewport_w: u32,
        viewport_h: u32,
        present: PresentFeel,
        screen_label: &str,
    ) {
        let w = viewport_w.max(1);
        let h = viewport_h.max(1);
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
        let tip_owned = self
            .command_hover
            .and_then(command_button_csf_tooltip)
            .and_then(|key| resolve_csf_text(csf, key));
        // 暂停菜单打开时不再画「已暂停」横幅文案。
        let show_pause_banner = hud.paused && hud.outcome.is_none();

        let caps = game.map(|g| g.snapshot_capabilities(&self.local.selected));
        let tabs_visible = Self::sidebar_tabs_visible(caps.as_ref());
        self.sync_sidebar_tab_to_visible(tabs_visible);
        let metrics = self
            .hud_chrome
            .as_ref()
            .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            .unwrap_or_else(BattleHudChromeMetrics::sidec01);
        let snap = solve_battle_hud_with_metrics(w, h, metrics);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let visible = cameo_visible_slot_count(band.h);
        self.clamp_cameo_scroll(visible);
        let items = caps
            .as_ref()
            .map(|c| Self::tab_items(c, self.sidebar_tab))
            .unwrap_or(&[]);
        let start = self.cameo_scroll.min(items.len());
        let end = (start + visible).min(items.len());
        let page_items = &items[start..end];
        let cameos: Vec<BattleCameoPaint<'_>> = page_items
            .iter()
            .map(|item| {
                let key = item.type_id.as_ref();
                BattleCameoPaint {
                    type_id: key,
                    image: self
                        .cameo_cache
                        .get(key)
                        .and_then(|opt| opt.as_ref())
                        .map(|s| &s.image),
                    enabled: item.enabled,
                    selected: matches!(self.sidebar_tab, 0 | 1)
                        && self.place_mode.as_deref() == Some(key),
                }
            })
            .collect();

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
            paused: show_pause_banner,
            pause_reason: None,
            command_pressed: if show_pause_banner { None } else { self.command_pressed },
            command_hovered: if show_pause_banner { None } else { self.command_hover },
            command_tip: if show_pause_banner { None } else { tip_owned.as_deref() },
            repair_active: !show_pause_banner && self.repair_mode,
            sell_active: !show_pause_banner && self.sell_mode,
            sidebar_tab: self.sidebar_tab.min(SIDEBAR_TAB_COUNT.saturating_sub(1)),
            sidebar_tabs_visible: tabs_visible,
            cameos: if show_pause_banner { &[] } else { &cameos },
        };
        // 与命中 / `world_viewport` 同口径：按窗口像素合成，避免 800×600 letterbox 错位。
        if let Some(mut page) = compose_battle_hud_overlay(w, h, fnt, paint, self.hud_chrome.as_ref()) {
            if let Some(rect) = self.left_gesture.marquee_rect() {
                stroke_marquee_rect(&mut page, rect);
            }
            if show_pause_banner {
                let metrics = self.pause_hud_metrics();
                if let Some(pause) = compose_battle_pause_menu_overlay(
                    w,
                    h,
                    self.pause_pressed,
                    self.pause_hover,
                    fnt,
                    csf,
                    self.pause_menu_chrome.as_ref(),
                    metrics,
                ) {
                    blit_rgba(&mut page, &pause, 0, 0);
                }
            }
            // 与壳层菜单同走 `[present]`，避免对局侧栏仍以满 8-bit 显得过亮。
            let page = present::present_ui_page(page, present);
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
                let place = self.place_mode.as_deref().unwrap_or("-");
                if screen_label == "results" {
                    format!("{} · [results] · t{} · Enter确认 Esc离开", self.title_base, hud.tick)
                }
                else if hud.paused {
                    format!(
                        "{} · [{screen_label}] · t{} · 暂停菜单 · Esc/回到游戏 · 放弃回大厅",
                        self.title_base, hud.tick
                    )
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
