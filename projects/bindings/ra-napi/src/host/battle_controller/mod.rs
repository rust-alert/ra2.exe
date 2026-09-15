//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

mod assets;
mod camera;
mod deployment;
mod hud;
mod input;
pub mod movement;
mod pause;
mod radar;
mod render;
mod tick;

// 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::{Duration, Instant},
};

use ra_adaptor::RulesSystem;
use ra_assets::Rgba;
use ra_engine::{Engine, Session};
use ra_map::{PaintDefinitions, StructureAnimBank, TerrainAnimBank, Theater, WeatherParticleField};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::EntityId;
use ra_widgets::{
    battle_hud::{BattleHudChrome, BattleHudHit},
    battle_pause_menu::BattlePauseChrome,
    skin::decode::DecodedUiSprite,
};

use super::{
    battle_input::{
        BattleInputTracker, BattleInteractionMode, BattlePointer, BattlePresentationState, BattleUiCapture, CameraPanKeys,
        EdgeScrollCursor, LeftGesture,
    },
    boot::BootResult,
    local_player::LocalPlayerController,
};

/// 遭遇战开局默认缩放（1 屏幕像素 ≈ 1 预览像素；禁止整图 fit）。
pub(super) const BATTLE_START_ZOOM: f32 = 1.0;

/// `keyboard.ini` `View1`–`View4` / `SetView` 书签槽数。
pub(super) const VIEW_BOOKMARK_COUNT: usize = 4;

/// 选中行动线可见时长（仿真 tick，对齐原版约 25 帧窗口）。
pub(super) const ACTION_LINES_DURATION_TICKS: u64 = 25;

/// 宿主镜头书签（世界中心 + 缩放；不进仿真权威）。
#[derive(Debug, Clone, Copy)]
pub(super) struct ViewBookmark {
    center_x: f32,
    center_y: f32,
    zoom: f32,
}

/// 待播对局音效 / EVA（可选声源格供距离衰减）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingBattleSfx {
    /// `sound.ini` / EVA 事件 id。
    pub event: String,
    /// 声源地图格；`None` = 非空间（EVA / UI 落位音等）。
    pub cell: Option<(u16, u16)>,
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
    /// 暂停菜单打开选项页（历史路径；局内现改走暂停子层 `InGameOptions`）。
    OpenOptions,
    /// 按 `keyboard.ini` ScreenCapture 请求截图。
    QueueScreenshot,
}

use deployment::{DeployVisualJob, PendingBuildup, PendingTeardown, TeardownVisualJob};

/// 对局页专用状态（与菜单 / 加载页隔离）。
pub struct BattleController {
    /// 长期引擎。
    pub engine: Option<Engine>,
    /// 当前会话。
    pub session: Option<Session>,
    /// 本地选中与点选指令（非权威）。
    pub local: LocalPlayerController,
    /// 左键点选 / 框选手势（不再用拖拽平移相机）。
    pub(super) left_gesture: LeftGesture,
    /// 指针按键 / 焦点边沿追踪（失焦取消按住，不派发释放点击）。
    pub(super) input_tracker: BattleInputTracker,
    /// 最近光标位置（逻辑像素；与菜单 / `DisplayMode` 同口径）。
    pub(super) cursor: (f64, f64),
    /// 上一帧时间，用于固定仿真时钟。
    pub(super) last_pump: Instant,
    /// 已记录过的胜负文案。
    pub(super) logged_outcome: Option<String>,
    /// 上一回记入日志的拒绝摘要。
    pub(super) logged_reject: Option<String>,
    /// Shift 是否按下（多选）。
    pub(super) shift_down: bool,
    /// Ctrl 是否按下。
    pub(super) ctrl_down: bool,
    /// Alt 是否按下（编队居中等修饰）。
    pub(super) alt_down: bool,
    /// 对局热键表（`keyboard.ini`，boot 装入）。
    pub(super) hotkeys: super::battle_hotkeys::HotkeyMap,
    /// `View1`–`View4` 镜头书签（`SetView` 写入）。
    pub(super) view_bookmarks: [Option<ViewBookmark>; VIEW_BOOKMARK_COUNT],
    /// 互斥交互模式（放置 / 修理 / 出售 / 规划 / 攻击移动 / 跟随）。
    pub(super) interaction_mode: BattleInteractionMode,
    /// 规划中暂存的航点（关闭规划时对当前选中下发 `order_move_path`）。
    pub(super) planning_waypoints: Vec<(u16, u16)>,
    /// 侧栏分类页签（0=建筑 / 1=防御 / 2=步兵 / 3=载具+飞行器）。
    pub(super) sidebar_tab: usize,
    /// 当前页签 cameo 列表滚动起点（可视槽 0 对应的条目下标）。
    pub(super) cameo_scroll: usize,
    /// 左键捕获层（HUD / 战术区互斥；释放必须对照按下捕获）。
    pub(super) ui_capture: BattleUiCapture,
    /// `HudSidebar` 捕获时的具体侧栏命中。
    pub(super) sidebar_capture_hit: Option<BattleHudHit>,
    /// 建造栏图标缓存（按类型键；`None` 表示已尝试但缺图，避免每帧重解）。
    pub(super) cameo_cache: HashMap<String, Option<DecodedUiSprite>>,
    /// 测试旁路：曾表示「再按 Esc 回大厅」武装态；现由暂停菜单「放弃」离开，恒为 false。
    pub(super) leave_armed: bool,
    /// 对局 Esc 暂停菜单阵营素材（`bkgd*` / `sidebttn`，跟本地 house）。
    pub(super) pause_menu_chrome: Option<BattlePauseChrome>,
    /// 是否已尝试解码暂停菜单（避免每帧重试；换边时清掉重解）。
    pub(super) pause_menu_tried_side: Option<String>,
    /// 暂停菜单悬停入口 id。
    pub(super) pause_hover: Option<&'static str>,
    /// 暂停菜单按下入口 id。
    pub(super) pause_pressed: Option<&'static str>,
    /// 暂停子层（Menu / AbortConfirm / InGameOptions / Diplomacy）。
    pub(super) pause_layer: ra_widgets::battle_pause_layer::BattlePauseLayer,
    /// 局内选项 `0xBBB` 草稿。
    pub(super) in_game_options: ra_widgets::battle_in_game_options::BattleInGameOptionsState,
    /// 未实现子页提示（如 Sound / Keyboard），叠在局内选项脚注区。
    pub(super) pause_stub_notice: Option<&'static str>,
    /// 标题用版本短名。
    pub(super) title_base: String,
    /// 测试状态旁路文件。
    pub(super) status_path: Option<PathBuf>,
    /// 测试场景名（重开用；当前由外壳 `LoadJob` 持有同名副本）。
    #[allow(dead_code)]
    pub(super) test_scene: Option<String>,
    /// 局内 HUD chrome（按本地阵营缓存；换边或重开时刷新）。
    pub(super) hud_chrome: Option<BattleHudChrome>,
    /// rules `Side=`（如 `ThirdSide`）；模组未知国名时与 house 一起选 UI chrome。
    pub(super) ui_faction_side: Option<String>,
    /// 已解析的壳层 chrome（优先 rules Side 段 `MixFileIndex` / 结算键）。
    pub(super) ui_faction_chrome: Option<ra_widgets::skirmish_setup::UiFactionChrome>,
    /// 是否已尝试装入 `mouse.shp` 命令图标。
    pub(super) order_icons_loaded: bool,
    /// 是否已尝试装入选中血条 `pips` / `pipbrd`。
    pub(super) selection_overlay_loaded: bool,
    /// `[AudioVisual] ConditionYellow`。
    pub(super) condition_yellow: f32,
    /// `[AudioVisual] ConditionRed`。
    pub(super) condition_red: f32,
    /// 命令条悬停槽。
    pub(super) command_hover: Option<usize>,
    /// 不含建筑/地形活动层的预览底图（可含开局移动单位与已定格建造场）。
    pub(super) preview_base: Option<RgbaImage>,
    /// 无开局移动单位、可烘焙已定格动态建筑的底图。
    pub(super) preview_clean: Option<RgbaImage>,
    /// 无可采矿的定格底图（产矿/采集脏刷新）。
    pub(super) preview_ore_underlay: Option<RgbaImage>,
    /// 无建筑的含矿底图（拆除擦像素）。
    pub(super) preview_structureless_clean: Option<RgbaImage>,
    /// 无建筑且无可采矿的底图。
    pub(super) preview_structureless_underlay: Option<RgbaImage>,
    /// 建筑活动层银行。
    pub(super) structure_anims: StructureAnimBank,
    /// 动画地形物件银行（旗帜等常循环）。
    pub(super) terrain_anims: TerrainAnimBank,
    /// 矿柱帧银行（由产矿状态机选帧）。
    pub(super) ore_tree_anims: TerrainAnimBank,
    /// art.ini 逻辑名。
    pub(super) art_ini: &'static str,
    /// rules.ini 逻辑名。
    pub(super) rules_ini: &'static str,
    /// 叠画用 art/rules 文档（boot 解析一次，热路径复用）。
    pub(super) paint: PaintDefinitions,
    /// 规则快照（房屋色调）。
    pub(super) rules: Option<RulesSystem>,
    /// 大厅行色 → house 主色。
    pub(super) lobby_primaries: HashMap<String, Rgba>,
    /// 正在播放的 Buildup。
    pub(super) pending_buildups: Vec<PendingBuildup>,
    /// 权威部署完成后待启动的呈现任务。
    pub(super) deploy_visual_queue: Vec<DeployVisualJob>,
    /// 正在播放的拆除 / 出售倒放。
    pub(super) pending_teardowns: Vec<PendingTeardown>,
    /// 权威拆除完成后待启动的呈现任务。
    pub(super) teardown_visual_queue: Vec<TeardownVisualJob>,
    /// 预览原点。
    pub(super) preview_origin: (i32, i32),
    /// 活动层呈现时钟起点。
    pub(super) anim_started: Instant,
    /// 上一帧活动层签名（跳过无变化上传）。
    pub(super) last_anim_sig: u64,
    /// 开局镜头尚未按战术区对齐（等表面尺寸可用后再 `focus`）。
    pub(super) start_view_pending: bool,
    /// 等待本 tick 结算的部署实体（`KeyD` 下发后）。
    pub(super) deploy_watch: Option<ra_types::EntityId>,
    /// 对局短音效 / EVA 队列（如 `PlaceBuilding`、`EVA_UnitLost`；由壳层播放）。
    pub(super) pending_battle_sfx: Vec<PendingBattleSfx>,
    /// EVA 串播门闩截止（逻辑层：同通道一次只放一句；设备层由 `ShellAudio::play_voice` 独立轨承载）。
    pub(super) eva_voice_until: Option<Instant>,
    /// 本机低电 EVA 已闩住（恢复供电后清闩，再掉电才再播）。
    pub(super) eva_low_power_latched: bool,
    /// 本机雷达开图闩（边沿驱动开/关音效与开图动画起点）。
    pub(super) radar_online_latched: bool,
    /// 雷达开图动画起点（仿真 tick）；离线为 `None`。
    pub(super) radar_open_started_tick: Option<u64>,
    /// 俯视小地图缓存（开图播完后绘制）。
    pub(super) radar_minimap: Option<RgbaImage>,
    /// 已观测到的本机存活机动单位（集合出现新 ID → 配合出厂边沿播 `EVA_UnitReady`）。
    pub(super) eva_alive_local_mobiles: HashSet<EntityId>,
    /// 是否已用当前存活集播种（首帧只建集、不播报）。
    pub(super) eva_alive_seeded: bool,
    /// 上一帧本机工厂仍在生产的实体 id（队列清空边沿 → `EVA_UnitReady`）。
    pub(super) eva_producing_factories: HashSet<EntityId>,
    /// 生产观测是否已播种（首帧只建集、不播报）。
    pub(super) eva_producing_seeded: bool,
    /// 已见过的侧栏可建造 / 可生产类型（集合增大 → `EVA_NewConstructionOptions`）。
    pub(super) eva_known_options: HashSet<String>,
    /// 建造选项集是否已播种。
    pub(super) eva_options_seeded: bool,
    /// 胜负已定后的结算延迟截止（先播 EVA，再 `ToResults`）。
    pub(super) outcome_hold_until: Option<Instant>,
    /// 收束期局内横幅全帧（战役 `CampaignScore.Animation`；缺图时仅字）。
    pub(super) outcome_banner_frames: Vec<ra_widgets::skin::decode::DecodedUiSprite>,
    /// 横幅动画当前帧下标（播完后停在末帧）。
    pub(super) outcome_banner_frame: usize,
    /// 横幅动画时钟（10 FPS）。
    pub(super) outcome_banner_clock: Option<Instant>,
    /// 横幅动画累加器（秒）。
    pub(super) outcome_banner_accum: f64,
    /// 是否已尝试装入 outcome 横幅（避免每帧扫 MIX）。
    pub(super) outcome_banner_tried: bool,
    /// 当前边缘滚屏光标（整窗边缘；右栏 / 命令条有效）。
    pub(super) edge_scroll_cursor: EdgeScrollCursor,
    /// 本帧呈现快照（壳层只应用指针，不重跑业务判断）。
    pub(super) presentation: BattlePresentationState,
    /// 方向键按住状态（渲染帧推进镜头，不跟逻辑 tick / OS 按键重复）。
    pub(super) camera_pan_keys: CameraPanKeys,
    /// 选中行动线计时起点（仿真 tick；`None` 表示未启动）。
    pub(super) action_lines_start_tick: Option<u64>,
    /// 当前地图剧院（壳层挂载剧院 MIX 用）。
    pub(super) map_theater: Option<Theater>,
    /// 天气氛围粒子（呈现层；雪地剧院默认飘雪）。
    pub(super) weather: WeatherParticleField,
    /// 天气粒子时钟起点。
    pub(super) weather_started: Instant,
    /// 上一帧已推进的天气毫秒（避免重复 tick）。
    pub(super) weather_last_ms: u64,
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
            input_tracker: BattleInputTracker::default(),
            cursor: (0.0, 0.0),
            last_pump: Instant::now(),
            logged_outcome: None,
            logged_reject: None,
            shift_down: false,
            ctrl_down: false,
            alt_down: false,
            hotkeys: boot.hotkeys,
            view_bookmarks: [None; VIEW_BOOKMARK_COUNT],
            interaction_mode: BattleInteractionMode::Normal,
            planning_waypoints: Vec::new(),
            sidebar_tab: 0,
            cameo_scroll: 0,
            ui_capture: BattleUiCapture::None,
            sidebar_capture_hit: None,
            cameo_cache: HashMap::new(),
            leave_armed: false,
            pause_menu_chrome: None,
            pause_menu_tried_side: None,
            pause_hover: None,
            pause_pressed: None,
            pause_layer: ra_widgets::battle_pause_layer::BattlePauseLayer::Menu,
            in_game_options: ra_widgets::battle_in_game_options::BattleInGameOptionsState::default(),
            pause_stub_notice: None,
            title_base: format!("ra2 ({edition})"),
            status_path,
            test_scene,
            hud_chrome: None,
            ui_faction_side: None,
            ui_faction_chrome: None,
            order_icons_loaded: false,
            selection_overlay_loaded: false,
            condition_yellow: 0.5,
            condition_red: 0.25,
            command_hover: None,
            preview_base: boot.preview_base,
            preview_clean: boot.preview_clean,
            preview_ore_underlay: boot.preview_ore_underlay,
            preview_structureless_clean: boot.preview_structureless_clean,
            preview_structureless_underlay: boot.preview_structureless_underlay,
            structure_anims: boot.structure_anims,
            terrain_anims: boot.terrain_anims,
            ore_tree_anims: boot.ore_tree_anims,
            art_ini: boot.art_ini,
            rules_ini: boot.rules_ini,
            paint: {
                debug_assert!(boot.paint.documents_sealed(), "battle paint must be sealed");
                boot.paint
            },
            rules: boot.rules,
            lobby_primaries: boot.lobby_primaries,
            pending_buildups: Vec::new(),
            deploy_visual_queue: Vec::new(),
            pending_teardowns: Vec::new(),
            teardown_visual_queue: Vec::new(),
            preview_origin: boot.preview_origin,
            anim_started: Instant::now(),
            last_anim_sig: u64::MAX,
            start_view_pending: has_session,
            deploy_watch: None,
            pending_battle_sfx: Vec::new(),
            eva_voice_until: None,
            eva_low_power_latched: false,
            radar_online_latched: false,
            radar_open_started_tick: None,
            radar_minimap: None,
            eva_alive_local_mobiles: HashSet::new(),
            eva_alive_seeded: false,
            eva_producing_factories: HashSet::new(),
            eva_producing_seeded: false,
            eva_known_options: HashSet::new(),
            eva_options_seeded: false,
            outcome_hold_until: None,
            outcome_banner_frames: Vec::new(),
            outcome_banner_frame: 0,
            outcome_banner_clock: None,
            outcome_banner_accum: 0.0,
            outcome_banner_tried: false,
            edge_scroll_cursor: EdgeScrollCursor::Default,
            presentation: BattlePresentationState::default(),
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

    /// 对局开局 / 重开：保持本机选择为空，不自动选中 MCV。
    ///
    /// 镜头定位由 [`Self::ensure_start_view`] / [`Self::focus_camera_on_local_start`] 单独完成，
    /// 不与单位选择或动作线绑定。
    pub(super) fn bind_local_start(&mut self) {
        self.local.selected.clear();
        self.action_lines_start_tick = None;
        if self.session.as_ref().and_then(|s| s.battle()).is_some() {
            tracing::debug!("开局保持未选中");
        }
    }

    pub(super) fn pulse_action_lines_at(&mut self, tick: u64) {
        self.action_lines_start_tick = Some(tick);
    }

    pub(super) fn action_lines_active(&self) -> bool {
        if !self.in_game_options.target_lines {
            return false;
        }
        let Some(start) = self.action_lines_start_tick
        else {
            return false;
        };
        let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick).unwrap_or(start);
        tick.saturating_sub(start) < ACTION_LINES_DURATION_TICKS
    }

    /// 局内选项滚屏档 → 相对默认速度的倍率（0 最慢，6 最快）。
    pub(super) fn scroll_rate_speed_scale(&self) -> f32 {
        let t = f32::from(self.in_game_options.scroll_rate.min(6)) / 6.0;
        0.35 + t * 1.30
    }

    /// 局内选项游戏速度档 → 仿真 `dt` 倍率（0 最慢，6 最快）。
    pub(super) fn game_speed_dt_scale(&self) -> f64 {
        let t = f64::from(self.in_game_options.game_speed.min(6)) / 6.0;
        0.25 + t * 1.50
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
        self.reset_transient_input_state(true);
        self.logged_outcome = None;
        self.logged_reject = None;
        self.interaction_mode = BattleInteractionMode::Normal;
        self.planning_waypoints.clear();
        self.sidebar_tab = 0;
        self.cameo_scroll = 0;
        self.ui_capture = BattleUiCapture::None;
        self.sidebar_capture_hit = None;
        self.cameo_cache.clear();
        self.leave_armed = false;
        self.pause_menu_chrome = None;
        self.pause_menu_tried_side = None;
        self.pause_hover = None;
        self.pause_pressed = None;
        self.pause_layer = ra_widgets::battle_pause_layer::BattlePauseLayer::Menu;
        self.in_game_options = ra_widgets::battle_in_game_options::BattleInGameOptionsState::default();
        self.pause_stub_notice = None;
        self.last_pump = Instant::now();
        self.hud_chrome = None;
        self.order_icons_loaded = false;
        self.selection_overlay_loaded = false;
        self.condition_yellow = 0.5;
        self.condition_red = 0.25;
        self.command_hover = None;
        self.preview_base = boot.preview_base;
        self.preview_clean = boot.preview_clean;
        self.preview_ore_underlay = boot.preview_ore_underlay;
        self.preview_structureless_clean = boot.preview_structureless_clean;
        self.preview_structureless_underlay = boot.preview_structureless_underlay;
        self.structure_anims = boot.structure_anims;
        self.terrain_anims = boot.terrain_anims;
        self.ore_tree_anims = boot.ore_tree_anims;
        self.art_ini = boot.art_ini;
        self.rules_ini = boot.rules_ini;
        debug_assert!(boot.paint.documents_sealed(), "battle paint must be sealed");
        self.paint = boot.paint;
        self.rules = boot.rules;
        self.lobby_primaries = boot.lobby_primaries;
        self.hotkeys = boot.hotkeys;
        self.view_bookmarks = [None; VIEW_BOOKMARK_COUNT];
        self.pending_buildups.clear();
        self.deploy_visual_queue.clear();
        self.pending_teardowns.clear();
        self.teardown_visual_queue.clear();
        self.preview_origin = boot.preview_origin;
        self.anim_started = Instant::now();
        self.last_anim_sig = u64::MAX;
        self.deploy_watch = None;
        self.pending_battle_sfx.clear();
        self.eva_voice_until = None;
        self.eva_low_power_latched = false;
        self.radar_online_latched = false;
        self.radar_open_started_tick = None;
        self.radar_minimap = None;
        self.eva_alive_local_mobiles.clear();
        self.eva_alive_seeded = false;
        self.eva_producing_factories.clear();
        self.eva_producing_seeded = false;
        self.eva_known_options.clear();
        self.eva_options_seeded = false;
        self.outcome_hold_until = None;
        self.outcome_banner_frames.clear();
        self.outcome_banner_frame = 0;
        self.outcome_banner_clock = None;
        self.outcome_banner_accum = 0.0;
        self.outcome_banner_tried = false;
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
            let edition = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.edition.as_str()).unwrap_or("—");
            self.title_base = format!("ra2 ({edition})");
            tracing::info!("重开完成 · {}", boot.note);
            self.bind_local_start();
            self.ensure_start_view(renderer);
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
                return match super::test_boot::boot_scene(scene) {
                    Ok(t) => BootResult::from_test(t),
                    Err(e) => BootResult::failed(format!("重开失败: {e}")),
                };
            }
        }
        let _ = &self.test_scene;
        super::boot::boot_from_install()
    }

    /// 当前选中是否含可部署单位（MCV 等）。
    pub fn selection_has_deployable(&self) -> bool {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return false;
        };
        self.local.selected.iter().any(|&id| game.entity_can_deploy(id))
    }

    /// 窗口表面度量（逻辑布局 + 物理表面）。
    pub(super) fn surface_metrics(window: &winit::window::Window) -> super::battle_input::BattleSurfaceMetrics {
        super::battle_input::BattleSurfaceMetrics::from_window(window)
    }

    /// 逻辑客户区尺寸（命中 / HUD / 边缘滚屏）。
    pub(super) fn logical_surface_size(window: &winit::window::Window) -> (u32, u32) {
        let m = Self::surface_metrics(window);
        (m.logical_width, m.logical_height)
    }

    /// 物理光标 → 逻辑像素写入 `cursor`。
    pub(super) fn set_cursor_from_physical(&mut self, window: &winit::window::Window, position: winit::dpi::PhysicalPosition<f64>) {
        self.cursor = Self::surface_metrics(window).cursor_from_physical(position);
    }

    /// 统一清理瞬时输入态（失焦 / 暂停 / 结算 / 页面切换 / 脚本锁）。
    ///
    /// `clear_tool_modes`：是否同时退出交互工具模式（进暂停菜单可清；失焦通常保留）。
    pub fn reset_transient_input_state(&mut self, clear_tool_modes: bool) {
        self.left_gesture = LeftGesture::Idle;
        self.ui_capture = BattleUiCapture::None;
        self.sidebar_capture_hit = None;
        self.pause_pressed = None;
        self.shift_down = false;
        self.ctrl_down = false;
        self.alt_down = false;
        self.camera_pan_keys.clear();
        self.edge_scroll_cursor = EdgeScrollCursor::Default;
        self.command_hover = None;
        self.input_tracker.reset_transient();
        if clear_tool_modes {
            let _ = self.clear_sidebar_tool_modes();
        }
        self.presentation = BattlePresentationState::default();
    }

    /// 本帧边沿已消费后调用：清零边沿，保留按住 / 焦点，供下一轮窗口事件重新累计。
    pub fn begin_input_frame(&mut self) {
        self.input_tracker.begin_frame();
    }

    /// 构造当前不可变输入帧（命中 / 释放对照共用）。
    pub(super) fn input_frame(&self, window: &winit::window::Window) -> super::battle_input::BattleInputFrame {
        use super::battle_input::OrderClickModifier;
        let metrics = Self::surface_metrics(window);
        let (cx, cy) = self.cursor;
        let cursor_in_window = cx >= 0.0 && cy >= 0.0 && cx < f64::from(metrics.logical_width) && cy < f64::from(metrics.logical_height);
        let cursor_in_world = self.map_viewport(window).contains_cursor(cx as i32, cy as i32);
        let mods = self.input_tracker.modifiers;
        super::battle_input::BattleInputFrame {
            metrics,
            cursor: self.cursor,
            cursor_in_window,
            cursor_in_world,
            shift_down: mods.shift,
            ctrl_down: mods.ctrl,
            alt_down: mods.alt,
            order_mod: OrderClickModifier::from_keys(mods.ctrl, mods.alt),
            tool: self.interaction_mode.tool_kind(),
            camera_pan_keys: self.camera_pan_keys,
            marquee: self.left_gesture.marquee_rect(),
            capture: self.ui_capture,
            buttons: self.input_tracker.buttons,
            modifiers: mods,
            edges: self.input_tracker.edges,
        }
    }

    /// 命令条按下槽（渲染高亮）；非 `HudCommand` 捕获时为 `None`。
    pub(super) fn command_button_pressed(&self) -> Option<usize> {
        match self.ui_capture {
            BattleUiCapture::HudCommand(slot) => Some(slot),
            _ => None,
        }
    }

    /// 本帧呈现快照（壳层只读应用）。
    pub fn presentation(&self) -> BattlePresentationState {
        self.presentation
    }

    /// 按当前边缘滚屏与悬停上下文刷新呈现快照（须在镜头 tick 之后、绘制之前调用）。
    pub fn update_presentation(&mut self, renderer: &Renderer, window: &winit::window::Window) {
        let hover = self.resolve_battle_hover(renderer, window);
        let pointer = BattlePointer::resolve(self.edge_scroll_cursor, hover.recommended_pointer);
        self.presentation = BattlePresentationState {
            pointer,
            hover_cell: hover.cell,
            hover_primary: hover.primary,
            marquee: self.left_gesture.marquee_rect(),
        };
    }

    /// 结算页标题刷新（不推进）。
    pub fn take_pump_clock(&mut self) -> Instant {
        let now = Instant::now();
        let prev = self.last_pump;
        self.last_pump = now;
        prev
    }

    /// 取出全部待播对局音效（离场 / 切页冲刷用；不遵守 EVA 串播门闩）。
    pub fn take_pending_battle_sfx(&mut self) -> Vec<PendingBattleSfx> {
        std::mem::take(&mut self.pending_battle_sfx)
    }

    /// 进结算页时清语音门闩与收束 hold（残留短音由壳层 `stop_sfx` 掐断）。
    pub fn clear_outcome_audio_gate(&mut self) {
        self.pending_battle_sfx.clear();
        self.eva_voice_until = None;
        self.outcome_hold_until = None;
    }

    /// 本帧可立即开播的事件：非 EVA 可并行取出；EVA 仅在语音门闩空闲时取队首一句。
    ///
    /// 短音与语音在设备层已分轨；本门闩只保证 EVA 不叠播，不压制开火 Report。
    pub fn drain_playable_battle_sfx(&mut self) -> Vec<PendingBattleSfx> {
        let now = Instant::now();
        let voice_free = self.eva_voice_until.map(|until| now >= until).unwrap_or(true);
        let mut play_now = Vec::new();
        let mut deferred = Vec::new();
        let mut took_eva = false;
        for cue in self.pending_battle_sfx.drain(..) {
            let is_eva = crate::host::audio::is_eva_event_id(&cue.event);
            if is_eva {
                if voice_free && !took_eva {
                    play_now.push(cue);
                    took_eva = true;
                }
                else {
                    deferred.push(cue);
                }
            }
            else {
                play_now.push(cue);
            }
        }
        self.pending_battle_sfx = deferred;
        play_now
    }

    /// 是否仍有未播 EVA，或语音门闩仍占用。
    pub fn eva_voice_busy(&self) -> bool {
        if self.pending_battle_sfx.iter().any(|e| crate::host::audio::is_eva_event_id(&e.event)) {
            return true;
        }
        self.eva_voice_until.is_some_and(|until| Instant::now() < until)
    }

    /// 记录一句 EVA 已开播：锁语音通道，并按采样时长拉长结算 hold。
    pub fn note_eva_voice_started(&mut self, sample: &ra_assets::PcmAudio) {
        let ch = sample.channels.max(1) as u64;
        let rate = u64::from(sample.sample_rate.max(1));
        let frames = (sample.samples.len() as u64) / ch;
        let ms = frames.saturating_mul(1000) / rate;
        // 尾音短留白，再放下一句（对齐 SpeakDelay 量级的间隙，不叠播）。
        let hold = Duration::from_millis(ms.saturating_add(200).max(400));
        self.eva_voice_until = Some(Instant::now() + hold);
        self.extend_outcome_hold(sample);
    }
}
