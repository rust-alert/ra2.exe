//! 壳层会话与平台宿主：页面导航、窗口生命周期；对局逻辑委托 `BattleController`。

use std::{path::PathBuf, sync::Arc, time::{Duration, Instant}};

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

use super::{
    boot::BootResult,
    load_job::LoadJob,
    battle_controller::{BattleController, BattleNav},
    preview_job::PreviewJob,
};
use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    shell_slide::{
        CAMPAIGN_SLIDE, CHOOSE_MAP_SLIDE, MAIN_MENU_SLIDE, SINGLE_PLAYER_SLIDE, SKIRMISH_SLIDE, ShellFrameWave, ShellSlideSpec,
        WAVE_STOWED_FRAME, WaveDirection,
    },
    skirmish_setup::{SkirmishBootRequest, hover_entry_at, side_flag_pcx},
    startup_splash::{self, StartupSplashPresentation},
    ui_assets::{MenuUiAssets, load_menu_ui_assets},
    ui_compose::{self, SkirmishChromeSprites},
    ui_decode, ui_hit,
    ui_movie::MenuMoviePlayer,
    ui_page::{page_resources_for_load_screen, page_resources_from_slots_with_edition},
    ui_present, ui_resolve,
    ui_text::{campaign_csf_tooltip, main_menu_csf_tooltip, resolve_csf_text, single_player_csf_tooltip, skirmish_lobby_csf_tooltip},
    ui_typewriter::TypewriterText,
};
use ra_layout::ui_layout;

/// 外壳持有的可导航壳层会话状态。
pub struct Shell {
    pub(super) window: Option<Arc<Window>>,
    pub(super) screen: OriginalScreen,
    /// 对局 / 结算页控制器；菜单页可为空。
    pub(super) battle_controller: Option<BattleController>,
    pub(super) renderer: Renderer,
    /// 菜单或装载说明。
    pub(super) banner: String,
    pub(super) window_width: f64,
    pub(super) window_height: f64,
    /// 客户区分辨率档（布局与缓冲基准，非自由拉伸）。
    pub(super) display_mode: DisplayMode,
    /// 壳层质感呈现（来自 `RustAlert.toml` `[present]`）。
    pub(super) present: PresentFeel,
    pub(super) status_path: Option<PathBuf>,
    pub(super) test_scene: Option<String>,
    /// 进程启动闪屏 presentation（独立 owner；非菜单槽）。
    pub(super) startup_splash: Option<StartupSplashPresentation>,
    /// 闪屏最短展示秒数（首次成功 present 后起算；可调，默认 3）。
    pub(super) splash_min_secs: f64,
    /// 遭遇战装载页最短展示秒数（`RustAlert.toml` 的 `load_min_secs`，默认 3；`0` 关闭）。
    pub(super) load_min_secs: f64,
    /// 壳层切页出去→进来之间的停顿秒数（模拟原版重型机械卡顿；`0` 关闭）。
    pub(super) shell_slide_gap_secs: f64,
    /// 闪屏预处理是否完成。
    pub(super) splash_preload_done: bool,
    /// 用户请求跳过闪屏（仍须预处理完成才进主菜单）。
    pub(super) splash_skip: bool,
    /// 装载完成后待切到的目标页。
    pub(super) pending_after_load: Option<OriginalScreen>,
    /// 光标位置（逻辑像素，与 `window_width` / `window_height` 同单位）。
    pub(super) cursor: (f64, f64),
    /// 后台遭遇战装载（`LoadScreen` 期间轮询）。
    pub(super) load_job: Option<LoadJob>,
    /// 当前装载开始时刻。
    pub(super) load_started: Option<Instant>,
    /// 后台已完成、等待最短展示时间后再 `finish_load` 的结果。
    pub(super) pending_load_boot: Option<BootResult>,
    /// 遭遇战大厅可选地图。
    pub(super) lobby_maps: Vec<super::boot::BootMapCandidate>,
    /// 当前选中的地图文件名。
    pub(super) selected_map: Option<String>,
    /// 大厅缩略图对应的地图名（与 `lobby_preview` 配对）。
    pub(super) lobby_preview_for: Option<String>,
    /// 已缩小的选中地图预览。
    pub(super) lobby_preview: Option<RgbaImage>,
    /// 后台地图预览任务。
    pub(super) lobby_preview_job: Option<PreviewJob>,
    /// 遭遇战控件 PCX 缓存（勾选/滑条拇指/旗标）。
    pub(super) skirmish_chrome: Option<SkirmishChromeSprites>,
    /// 旗标缓存对应的阵营名（换边时重载）。
    pub(super) skirmish_chrome_side: Option<String>,
    /// 遭遇战左栏按下是否已消费（勾选/滑条，勿再走右栏按钮命中）。
    pub(super) skirmish_pointer_consumed: bool,
    /// 主菜单阶段已挂载资源（惰性一次）。
    pub(super) menu_assets: Option<MenuUiAssets>,
    /// 当前页 chrome 解码缓存（切换页或重探时刷新）。
    pub(super) ui_decode_cache: Option<ui_decode::PageDecodeReport>,
    /// 主菜单当前按住的按钮入口 id（按下帧合成）。
    pub(super) menu_pressed_entry: Option<&'static str>,
    /// 切页排队：`SlideOut` 完成后提交的 `MenuAction`。
    pub(super) menu_pending_commit: Option<MenuAction>,
    /// 进行中的右栏 `SDBTNANM` 帧波浪（出去 / 进来）。
    pub(super) menu_frame_wave: Option<ShellFrameWave>,
    /// 出去结束后、进来开始前的卡顿截止时刻（无字、钮面收起）。
    pub(super) menu_slide_gap_until: Option<Instant>,
    /// 主菜单当前悬停的按钮入口 id（悬停帧合成）。
    pub(super) menu_hovered_entry: Option<&'static str>,
    /// 底栏状态提示打字机（与按钮 hover 图解耦；亦可复用于局内右上消息）。
    pub(super) status_line: TypewriterText,
    /// 菜单字体（`game.fnt`）。
    pub(super) menu_font: Option<FntFile>,
    /// 是否已尝试装载菜单字体（失败后不再每帧读盘/打日志）。
    pub(super) menu_font_tried: bool,
    /// 菜单文案表（`ra2.csf` / `ra2md.csf`）。
    pub(super) menu_csf: Option<CsfFile>,
    /// 是否已尝试装载菜单文案表。
    pub(super) menu_csf_tried: bool,
    /// 主菜单 / 单人页循环影片。
    pub(super) menu_movie: Option<MenuMoviePlayer>,
    /// 影片时钟（`tick` 用）。
    pub(super) menu_movie_clock: Option<Instant>,
    /// WARNING 窗内 `sdwrnanm` 动画时钟（与侧图箭头分离）。
    pub(super) menu_panel_anim_clock: Option<Instant>,
    /// WARNING 动画未消耗的累计秒。
    pub(super) menu_panel_anim_accum: f64,
    /// `sdwrnanm` 帧序号（对解码帧数取模）。
    pub(super) menu_panel_anim_frame: usize,
    /// 战役侧图箭头动画时钟。
    pub(super) campaign_side_anim_clock: Option<Instant>,
    /// 侧图动画累计秒。
    pub(super) campaign_side_anim_accum: f64,
    /// 侧图箭头帧序号。
    pub(super) campaign_side_anim_frame: usize,
    /// 战役选边悬停语音（`AlliedCampaignSelect` 等，惰性）。
    pub(super) campaign_side_sfx: [Option<PcmAudio>; 3],
    /// 下一帧回读后落盘的截图短名（`OriginalScreen::as_str`）；F12 手动截图用。
    pub(super) pending_screenshot: Option<&'static str>,
    /// 自动关键页截图去重（仅 `test-harness`）。
    #[cfg(feature = "test-harness")]
    pub(super) auto_screenshots: super::screenshot::AutoScreenshotTracker,
    /// 遭遇战大厅阵营 / 难度（进入装载请求）。
    pub(super) skirmish: SkirmishBootRequest,
    /// 进入选图页前的 `preferred_map` 快照（取消时还原）。
    pub(super) choose_map_revert: Option<Option<String>>,
    /// 战役选边：`allied` / `tutorial` / `soviet`。
    pub(super) campaign_side: Option<&'static str>,
    /// 战役难度档：0 易 / 1 中 / 2 难。
    pub(super) campaign_difficulty: u8,
    /// 战役难度滑条是否正在拖动。
    pub(super) campaign_dragging: bool,
    /// 战役左栏按下是否已消费（难度滑条，勿再走点击轮换）。
    pub(super) campaign_pointer_consumed: bool,
    /// 桌面音频输出（设备不可用则为 `None`）。
    pub(super) audio: Option<super::audio::ShellAudio>,
    /// 主菜单 BGM PCM（`theme.ini` `[INTRO]` → `{Sound}.wav`）。
    pub(super) menu_bgm: Option<PcmAudio>,
    /// 是否已尝试装载菜单 BGM（失败后不再每帧重试）。
    pub(super) menu_bgm_tried: bool,
    /// 菜单点击音效 PCM（`GUIMainButtonSound` → `sound.ini` → `audio.bag`）。
    pub(super) menu_click: Option<PcmAudio>,
    /// 壳层出去音效（`GUIMoveOutSound` → 默认 `MenuSlideOut` / `uslide2`）。
    pub(super) menu_move_out: Option<PcmAudio>,
    /// 是否已尝试装载出去音效。
    pub(super) menu_move_out_tried: bool,
    /// 壳层进来音效（`GUIMoveInSound` → 默认 `MenuSlideIn` / `uslide1`）。
    pub(super) menu_move_in: Option<PcmAudio>,
    /// 是否已尝试装载进来音效。
    pub(super) menu_move_in_tried: bool,
    /// 当前是否已在播壳层 BGM。
    pub(super) menu_bgm_playing: bool,
    /// 已解析的 `audio.bag` 索引（惰性）。
    pub(super) audio_bag: Option<AudioIndex>,
    /// 是否已尝试装载 `audio.bag`（避免反复读盘）。
    pub(super) audio_bag_tried: bool,
    /// 选项页草稿（进入 Options 时创建，接受/取消后清空）。
    pub(super) options_state: Option<ra_widgets::options_dialog::OptionsDialogState>,
    /// 进入选项页时的音量快照（取消时还原实时预览）。
    pub(super) options_volume_baseline: Option<(f32, f32)>,
    /// 进入选项页时的质感快照（取消时还原实时预览）。
    pub(super) options_present_baseline: Option<PresentFeel>,
    /// 本轮按下已由左栏控件消费（释放时勿再走右栏命中）。
    pub(super) options_pointer_consumed: bool,
    /// 上次已写入的窗口标题（避免每帧 `set_title` 卡顿）。
    pub(super) last_shell_title: String,
}

impl Shell {

}

mod construct;
mod host;
mod audio;
mod splash;
mod options;
mod lobby;
mod campaign;
mod assets;
mod diagnostics;
mod navigation;
mod loading;
mod input;
mod redraw;
mod event_loop;
mod launch;

pub use host::Host;
pub use launch::{campaign_difficulty_from_track_x, run_shell};
