//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

mod assets;
mod camera;
mod deployment;
mod hud;
mod input;
pub mod movement;
mod pause;
mod render;
mod tick;

// 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Instant,
};

use ra_adaptor::RulesSystem;
use ra_assets::Rgba;
use ra_engine::{Engine, Session};
use ra_map::{PaintIniDocs, StructureAnimBank, TerrainAnimBank, Theater, WeatherParticleField};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::EntityId;
use ra_widgets::{
    battle_hud::{BattleHudChrome, BattleHudHit},
    battle_pause_menu::BattlePauseChrome,
    skin::decode::DecodedUiSprite,
};

use super::{
    battle_input::{CameraPanKeys, EdgeScrollCursor, LeftGesture},
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
    /// 切换无边框全屏。
    ToggleFullscreen,
    /// 按 `keyboard.ini` ScreenCapture 请求截图。
    QueueScreenshot,
}

use deployment::{DeployVisualJob, PendingBuildup};

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
    /// 最近光标位置（窗口像素）。
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
    /// 建造放置模式（建筑类型键）。
    pub(super) place_mode: Option<String>,
    /// 侧栏修理工具是否激活（与出售互斥；激活时贴按下帧）。
    pub(super) repair_mode: bool,
    /// 侧栏出售工具是否激活（与修理互斥；激活时贴按下帧）。
    pub(super) sell_mode: bool,
    /// 命令条路径点规划模式（激活时命令条贴按下帧）。
    pub(super) planning_mode: bool,
    /// 规划中暂存的航点（关闭规划时对当前选中下发 `order_move_path`）。
    pub(super) planning_waypoints: Vec<(u16, u16)>,
    /// 侧栏分类页签（0=建筑 / 1=防御 / 2=步兵 / 3=载具+飞行器）。
    pub(super) sidebar_tab: usize,
    /// 当前页签 cameo 列表滚动起点（可视槽 0 对应的条目下标）。
    pub(super) cameo_scroll: usize,
    /// 侧栏按下（页签 / cameo），松手命中一致时生效。
    pub(super) sidebar_pressed: Option<BattleHudHit>,
    /// 建造栏图标缓存（按类型键；`None` 表示已尝试但缺图，避免每帧重解）。
    pub(super) cameo_cache: HashMap<String, Option<DecodedUiSprite>>,
    /// 测试旁路：曾表示「再按 Esc 回大厅」武装态；现由暂停菜单「放弃」离开，恒为 false。
    pub(super) leave_armed: bool,
    /// 对局 Esc 暂停菜单阵营素材（`radar` / `sidebttn`，跟本地 house）。
    pub(super) pause_menu_chrome: Option<BattlePauseChrome>,
    /// 是否已尝试解码暂停菜单（避免每帧重试；换边时清掉重解）。
    pub(super) pause_menu_tried_side: Option<String>,
    /// 暂停菜单悬停入口 id。
    pub(super) pause_hover: Option<&'static str>,
    /// 暂停菜单按下入口 id。
    pub(super) pause_pressed: Option<&'static str>,
    /// 暂停子层（Menu / AbortConfirm / InGameOptions）。
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
    /// 命令条按下槽（高亮）。
    pub(super) command_pressed: Option<usize>,
    /// 不含建筑/地形活动层的预览底图（可含开局移动单位与已定格建造场）。
    pub(super) preview_base: Option<RgbaImage>,
    /// 无开局移动单位、可烘焙已定格动态建筑的底图。
    pub(super) preview_clean: Option<RgbaImage>,
    /// 无可采矿的定格底图（产矿/采集脏刷新）。
    pub(super) preview_ore_underlay: Option<RgbaImage>,
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
    pub(super) paint_ini: PaintIniDocs,
    /// 规则快照（房屋色调）。
    pub(super) rules: Option<RulesSystem>,
    /// 大厅行色 → house 主色。
    pub(super) lobby_primaries: HashMap<String, Rgba>,
    /// 正在播放的 Buildup。
    pub(super) pending_buildups: Vec<PendingBuildup>,
    /// 权威部署完成后待启动的呈现任务。
    pub(super) deploy_visual_queue: Vec<DeployVisualJob>,
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
    /// 对局短音效 / EVA 事件 id 队列（如 `PlaceBuilding`、`EVA_UnitLost`；由壳层播放）。
    pub(super) pending_battle_sfx: Vec<String>,
    /// 本机低电 EVA 已闩住（恢复供电后清闩，再掉电才再播）。
    pub(super) eva_low_power_latched: bool,
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
    /// 当前边缘滚屏光标（整窗边缘；右栏 / 命令条有效）。
    pub(super) edge_scroll_cursor: EdgeScrollCursor,
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
            cursor: (0.0, 0.0),
            last_pump: Instant::now(),
            logged_outcome: None,
            logged_reject: None,
            shift_down: false,
            ctrl_down: false,
            alt_down: false,
            hotkeys: boot.hotkeys,
            view_bookmarks: [None; VIEW_BOOKMARK_COUNT],
            place_mode: None,
            repair_mode: false,
            sell_mode: false,
            planning_mode: false,
            planning_waypoints: Vec::new(),
            sidebar_tab: 0,
            cameo_scroll: 0,
            sidebar_pressed: None,
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
            command_pressed: None,
            preview_base: boot.preview_base,
            preview_clean: boot.preview_clean,
            preview_ore_underlay: boot.preview_ore_underlay,
            structure_anims: boot.structure_anims,
            terrain_anims: boot.terrain_anims,
            ore_tree_anims: boot.ore_tree_anims,
            art_ini: boot.art_ini,
            rules_ini: boot.rules_ini,
            paint_ini: boot.paint_ini,
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
    pub(super) fn bind_local_start(&mut self) {
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
        self.logged_outcome = None;
        self.logged_reject = None;
        self.place_mode = None;
        self.repair_mode = false;
        self.sell_mode = false;
        self.planning_mode = false;
        self.planning_waypoints.clear();
        self.sidebar_tab = 0;
        self.cameo_scroll = 0;
        self.sidebar_pressed = None;
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
        self.command_pressed = None;
        self.preview_base = boot.preview_base;
        self.preview_clean = boot.preview_clean;
        self.preview_ore_underlay = boot.preview_ore_underlay;
        self.structure_anims = boot.structure_anims;
        self.terrain_anims = boot.terrain_anims;
        self.ore_tree_anims = boot.ore_tree_anims;
        self.art_ini = boot.art_ini;
        self.rules_ini = boot.rules_ini;
        self.paint_ini = boot.paint_ini;
        self.rules = boot.rules;
        self.lobby_primaries = boot.lobby_primaries;
        self.hotkeys = boot.hotkeys;
        self.view_bookmarks = [None; VIEW_BOOKMARK_COUNT];
        self.pending_buildups.clear();
        self.deploy_visual_queue.clear();
        self.preview_origin = boot.preview_origin;
        self.anim_started = Instant::now();
        self.last_anim_sig = u64::MAX;
        self.deploy_watch = None;
        self.pending_battle_sfx.clear();
        self.eva_low_power_latched = false;
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
        self.local.selected.iter().any(|&id| game.deploy_target_of(id).is_some())
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
