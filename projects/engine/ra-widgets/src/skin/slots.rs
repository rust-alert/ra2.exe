//! 原版产品页的逻辑 UI 资源槽（按页面组织，不依赖 `ui.ini` 当素材目录）。
//!
//! 第一阶段对照锁定 **RA2 原版**（非默认 YR）。槽位先对齐入口 id 与资源名；
//! 具体 SHP/PAL/帧须有安装内证据后再填，禁止臆造。
//! **填了文件名 ≠ 已解码 ≠ 已 GPU 绘制 ≠ Pre-Alpha 视觉交付。**
//! 页面级资源索引见 [`crate::screens::page`]；可读性探测见 [`crate::skin::resolve`]；逻辑命中见 [`crate::input::hit`]。

use ra_types::GameEdition;

use crate::{menu_action::MenuAction, original_screen::OriginalScreen};

/// 主菜单 / 单人页循环片逻辑名（大布局）。
pub const MENU_MOVIE_BIK_LARGE: &str = "ra2ts_l.bik";
/// 主菜单循环片逻辑名（640 窄布局）。
pub const MENU_MOVIE_BIK_SMALL: &str = "ra2ts_s.bik";

/// YR / Mo3 同名片在 `langmd.mix`（覆盖 `language.mix` 的原版 `ra2ts_*`）。
pub const MENU_MOVIE_PREFER_MIX_MD: &str = "langmd.mix";

/// 资料片主菜单片应优先从哪个 MIX 读取（`None` = 默认全局解析）。
pub fn menu_movie_prefer_mix(edition: Option<GameEdition>) -> Option<&'static str> {
    match edition {
        Some(GameEdition::Yr | GameEdition::Mo3) => Some(MENU_MOVIE_PREFER_MIX_MD),
        _ => None,
    }
}

/// 侧板 / 装饰层槽。
#[derive(Debug, Clone, Copy)]
pub struct UiPanelSlot {
    /// 逻辑 id（诊断用）。
    pub id: &'static str,
    /// SHP 名。
    pub shp: &'static str,
    /// 调色板名。
    pub pal: &'static str,
    /// 帧号（通常为 0）。
    pub frame: u16,
}

/// 单个按钮/入口的资源与命中约定。
#[derive(Debug, Clone)]
pub struct UiButtonSlot {
    /// 与机读 UI 状态一致的入口 id（如 `single_player`）。
    pub entry_id: &'static str,
    /// 壳层导航动作。
    pub action: MenuAction,
    /// 是否可点（未实现模式保留位置但禁用）。
    pub enabled: bool,
    /// 按钮动画 SHP（多状态常为同文件不同帧）。
    pub anim_shp: Option<&'static str>,
    /// 按钮调色板。
    pub anim_pal: Option<&'static str>,
    /// 常态帧。
    pub normal_frame: Option<u16>,
    /// 悬停帧（主菜单策略可为 `None`，表示不换悬停帧）。
    pub hover_frame: Option<u16>,
    /// 按下帧。
    pub pressed_frame: Option<u16>,
    /// 禁用帧。
    pub disabled_frame: Option<u16>,
}

/// 一页的逻辑资源描述。
#[derive(Debug, Clone)]
pub struct UiPageSlots {
    /// 原版产品页。
    pub screen: OriginalScreen,
    /// 背景 SHP（可空）。
    pub background_shp: Option<&'static str>,
    /// 背景 PCX（与 SHP 二选一优先 PCX；进程启动闪屏不走本槽）。
    pub background_pcx: Option<&'static str>,
    /// 背景调色板（可空）。
    pub background_pal: Option<&'static str>,
    /// 背景帧（缺省 0）。
    pub background_frame: u16,
    /// 主菜单循环影片（如 `ra2ts_l.bik`）；可空。
    pub movie_bik: Option<&'static str>,
    /// 侧板 / 装饰。
    pub panels: &'static [UiPanelSlot],
    /// 本页字体逻辑名。
    pub fonts: &'static [&'static str],
    /// 页面入口。
    pub buttons: &'static [UiButtonSlot],
}

impl UiPageSlots {
    /// 是否已为任一槽填了具体文件名（用于区分「模型」与「已接线资产」）。
    pub fn has_any_asset_name(&self) -> bool {
        if self.background_shp.is_some() || self.background_pcx.is_some() || self.background_pal.is_some() || !self.panels.is_empty() {
            return true;
        }
        if self.movie_bik.is_some() {
            return true;
        }
        if !self.fonts.is_empty() {
            return true;
        }
        self.buttons.iter().any(|b| b.anim_shp.is_some())
    }
}

/// 零售主菜单按钮动画（安装内 `neutral.mix` 证据）。
const SDBTNANM_SHP: &str = "sdbtnanm.shp";
const SDBTNANM_PAL: &str = "sdbtnanm.pal";
/// 常态 / 按下帧。主菜单悬停只更新底栏提示，不换 SHP 帧（零售同款）。
const SDBTNANM_FRAME_NORMAL: u16 = 2;
const SDBTNANM_FRAME_PRESSED: u16 = 4;

/// 退出确认 MessageBox 底板（`pudlgbgn` 帧 0）。
///
/// 调色板随资料片变化：原版 RA2 为 `dialog.pal`；YR / Mo3 的 MD 底板配 `dialogn.pal`。
/// 合集盘上 `expandmd01.mix` 会覆盖同名底板；`edition=ra2` 时 adaptor 不得挂载 `expandmd*`。
const PUDLGBGN_SHP: &str = "pudlgbgn.shp";
const PUDLGBGN_PAL: &str = "dialog.pal";
const PUDLGBGN_PAL_MD: &str = "dialogn.pal";

/// 退出确认底板调色板：RA2 → `dialog.pal`；YR / Mo3 → `dialogn.pal`。
pub fn pudlgbgn_palette(edition: Option<GameEdition>) -> &'static str {
    match edition {
        Some(GameEdition::Yr | GameEdition::Mo3) => PUDLGBGN_PAL_MD,
        _ => PUDLGBGN_PAL,
    }
}
/// 退出确认确定/取消按钮（`mnbttn`：0 抬起 / 1 按下 / 2 禁用；画布 126×25）。
const MNBTTN_SHP: &str = "mnbttn.shp";
const MNBTTN_PAL: &str = "mainbttn.pal";
const MNBTTN_FRAME_UP: u16 = 0;
const MNBTTN_FRAME_PRESSED: u16 = 1;
const MNBTTN_FRAME_DISABLED: u16 = 2;

/// 主菜单 chrome：`sdtp` 帧 0 作外壳，`sdwrnanm` 仅作窗内内容（合成时 1:1 裁入，勿整幅拉伸）。
const MAIN_MENU_PANELS: &[UiPanelSlot] = &[
    UiPanelSlot { id: "right_top", shp: "sdtp.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "warn_anim", shp: "sdwrnanm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "right_tile", shp: "sdbtnbkgd.shp", pal: "shell2.pal", frame: 0 },
    UiPanelSlot { id: "right_bottom", shp: "sdbtm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "lower_side", shp: "lwscrnl.shp", pal: "shell.pal", frame: 0 },
];

/// 主菜单壳层 + 退出确认底板。
const EXIT_CONFIRM_PANELS: &[UiPanelSlot] = &[
    UiPanelSlot { id: "right_top", shp: "sdtp.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "warn_anim", shp: "sdwrnanm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "right_tile", shp: "sdbtnbkgd.shp", pal: "shell2.pal", frame: 0 },
    UiPanelSlot { id: "right_bottom", shp: "sdbtm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "lower_side", shp: "lwscrnl.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "exit_modal_bg", shp: PUDLGBGN_SHP, pal: PUDLGBGN_PAL, frame: 0 },
];

const MAIN_MENU_FONTS: &[&str] = &["game.fnt"];

const fn main_menu_button(entry_id: &'static str, action: MenuAction, enabled: bool) -> UiButtonSlot {
    UiButtonSlot {
        entry_id,
        action,
        enabled,
        anim_shp: Some(SDBTNANM_SHP),
        anim_pal: Some(SDBTNANM_PAL),
        normal_frame: Some(SDBTNANM_FRAME_NORMAL),
        // 悬停不换帧；按下用帧 4。
        hover_frame: None,
        pressed_frame: Some(SDBTNANM_FRAME_PRESSED),
        disabled_frame: if enabled { None } else { Some(SDBTNANM_FRAME_NORMAL) },
    }
}

const fn empty_button(entry_id: &'static str, action: MenuAction, enabled: bool) -> UiButtonSlot {
    UiButtonSlot {
        entry_id,
        action,
        enabled,
        anim_shp: None,
        anim_pal: None,
        normal_frame: None,
        hover_frame: None,
        pressed_frame: None,
        disabled_frame: None,
    }
}

// 命中框为 800×600 内容归一化；实际点击经 `ra_layout` shell helpers + fit 相机，不直接用窗口比例。
const MAIN_MENU_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("single_player", MenuAction::OpenSinglePlayer, true),
    main_menu_button("ww_online", MenuAction::Noop, false),
    main_menu_button("network", MenuAction::OpenNetwork, false),
    main_menu_button("movies", MenuAction::Noop, false),
    main_menu_button("options", MenuAction::OpenOptions, true),
    main_menu_button("exit", MenuAction::Exit, true),
];

// 命中框占位；实际点击走 `ra_layout` shell helpers 单人页像素格。
// 顺序对齐壳层：新战役 / 载入 / 遭遇战 / 主菜单（贴底）。
const SINGLE_PLAYER_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("campaign", MenuAction::OpenCampaign, true),
    main_menu_button("load", MenuAction::Noop, false),
    main_menu_button("skirmish", MenuAction::OpenSkirmish, true),
    main_menu_button("back", MenuAction::Back, true),
];

/// 战役页：右栏仅「上一页」；中列为空部队格（cameo 后续接线）。
const CAMPAIGN_BUTTONS: &[UiButtonSlot] = &[main_menu_button("back", MenuAction::Back, true)];

/// 战役页面板：壳层 chrome。三侧 `fs*.shp` 供悬停箭头透叠；静态徽标在 `fsbkgdlg`。
const CAMPAIGN_PANELS: &[UiPanelSlot] = &[
    UiPanelSlot { id: "right_top", shp: "sdtp.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "warn_anim", shp: "sdwrnanm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "right_tile", shp: "sdbtnbkgd.shp", pal: "shell2.pal", frame: 0 },
    UiPanelSlot { id: "right_bottom", shp: "sdbtm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "lower_side", shp: "lwscrnl.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "allied", shp: "fsalg.shp", pal: "fsscrn.pal", frame: 0 },
    UiPanelSlot { id: "tutorial", shp: "fsbclg.shp", pal: "fsscrn.pal", frame: 0 },
    UiPanelSlot { id: "soviet", shp: "fsslg.shp", pal: "fsscrn.pal", frame: 0 },
];

// 命中框占位；实际点击走 `ra_layout` shell helpers 遭遇战像素格。
// 顺序：开始游戏 / 选图 / 上一页（贴底）。
const SKIRMISH_LOBBY_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("start", MenuAction::StartSkirmish, true),
    main_menu_button("choose_map", MenuAction::ChooseMap, true),
    main_menu_button("back", MenuAction::Back, true),
];

/// 遭遇战 / 选图右栏：`sdtp` 外壳 + `sdmpbtn` 地图名底板 + 底条；合成时再叠 `sdtp` 帧 1 作顶栏高亮牌。
/// 不画 WARNING 动画。
const SKIRMISH_LOBBY_PANELS: &[UiPanelSlot] = &[
    UiPanelSlot { id: "right_top", shp: "sdtp.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "minimap_plate", shp: "sdmpbtn.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "right_tile", shp: "sdbtnbkgd.shp", pal: "shell2.pal", frame: 0 },
    UiPanelSlot { id: "right_bottom", shp: "sdbtm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "lower_side", shp: "lwscrnl.shp", pal: "shell.pal", frame: 0 },
];

/// 遭遇战积分：壳层右栏（含 WARNING 表头区）+ 底栏「继续」。
const SKIRMISH_SCORE_PANELS: &[UiPanelSlot] = &[
    UiPanelSlot { id: "right_top", shp: "sdtp.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "warn_anim", shp: "sdwrnanm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "right_tile", shp: "sdbtnbkgd.shp", pal: "shell2.pal", frame: 0 },
    UiPanelSlot { id: "right_bottom", shp: "sdbtm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "lower_side", shp: "lwscrnl.shp", pal: "shell.pal", frame: 0 },
];

const SKIRMISH_SCORE_BUTTONS: &[UiButtonSlot] = &[main_menu_button("continue", MenuAction::Back, true)];

/// 选图页：使用地图 / 创建随机地图（未实现，仍可点以播反馈）/ 取消。
const CHOOSE_MAP_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("use_map", MenuAction::UseMap, true),
    main_menu_button("create_random", MenuAction::Noop, true),
    main_menu_button("cancel", MenuAction::Back, true),
];

/// 装载页：国家 `ls800*`/`ls640*` 全幅艺术 + `progbarm`；失败时 `mnbttn` 重试/取消。
const LOAD_SCREEN_PANELS: &[UiPanelSlot] = &[UiPanelSlot {
    id: "progress",
    shp: "progbarm.shp",
    // 填充条在 conquer.mix；与国家艺术共用 mpls.pal 会花屏（extract --decode-shp 已证）。
    pal: "shell.pal",
    frame: 0,
}];

const LOAD_SCREEN_BUTTONS: &[UiButtonSlot] =
    &[modal_button("retry", MenuAction::RetryLoad, true), modal_button("cancel", MenuAction::CancelLoad, true)];

// 命中框占位；实际点击走 `ra_layout` shell helpers 选项页像素格。右栏为接受 / 取消 / 主菜单。
const OPTIONS_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("accept", MenuAction::OptionsAccept, true),
    main_menu_button("cancel", MenuAction::OptionsCancel, true),
    main_menu_button("main_menu", MenuAction::Back, true),
];

const fn modal_button(entry_id: &'static str, action: MenuAction, enabled: bool) -> UiButtonSlot {
    UiButtonSlot {
        entry_id,
        action,
        enabled,
        anim_shp: Some(MNBTTN_SHP),
        anim_pal: Some(MNBTTN_PAL),
        normal_frame: Some(MNBTTN_FRAME_UP),
        hover_frame: None,
        pressed_frame: Some(MNBTTN_FRAME_PRESSED),
        disabled_frame: if enabled { None } else { Some(MNBTTN_FRAME_DISABLED) },
    }
}

// 退出确认：底下仍画主菜单六钮（仅视觉，命中只走 ok/cancel）+ MessageBox 确定/取消。
const EXIT_CONFIRM_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("single_player", MenuAction::Noop, true),
    main_menu_button("ww_online", MenuAction::Noop, false),
    main_menu_button("network", MenuAction::Noop, false),
    main_menu_button("movies", MenuAction::Noop, false),
    main_menu_button("options", MenuAction::Noop, true),
    main_menu_button("exit", MenuAction::Noop, true),
    modal_button("ok", MenuAction::ConfirmExit, true),
    modal_button("cancel", MenuAction::Back, true),
];

const NETWORK_BUTTONS: &[UiButtonSlot] = &[empty_button("online", MenuAction::Noop, false), empty_button("back", MenuAction::Back, true)];

/// 对局 Esc 暂停菜单：右缘四钮，与 [`ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS`] 对齐。
const BATTLE_PAUSE_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("options", MenuAction::OpenOptions, true),
    main_menu_button("fullscreen", MenuAction::Noop, true),
    main_menu_button("abort", MenuAction::Back, true),
    main_menu_button("resume", MenuAction::Noop, true),
];

/// 对局暂停菜单资源槽（历史壳层路径；现行合成改走 `battle_pause_menu::decode_battle_pause_chrome`）。
pub fn battle_pause_page_slots() -> UiPageSlots {
    UiPageSlots {
        screen: OriginalScreen::Battle,
        background_shp: None,
        background_pcx: None,
        background_pal: None,
        background_frame: 0,
        movie_bik: None,
        // 不再绑主菜单 `sdtp`；保留空面板以免旧诊断路径 panic。
        panels: &[],
        fonts: MAIN_MENU_FONTS,
        buttons: BATTLE_PAUSE_BUTTONS,
    }
}

/// 返回某原版产品页的逻辑槽位；对局/结算无前置菜单槽。
pub fn slots_for(screen: OriginalScreen) -> Option<UiPageSlots> {
    match screen {
        // 进程启动闪屏由 `startup_splash` owner 呈现（`GLSS`/`GLSL` + `GLS.PAL`），
        // 不是菜单壳层槽；勿再绑 `title.pcx`。
        OriginalScreen::Splash => None,
        OriginalScreen::MainMenu => Some(UiPageSlots {
            screen,
            // 非 640 宽窗口默认大背景；640 分支后续按视口另选 `mnscrns.shp`。
            background_shp: Some("mnscrnl.shp"),
            background_pcx: None,
            background_pal: Some("shell.pal"),
            background_frame: 0,
            // 大布局默认 `ra2ts_l.bik`；640 窄布局后续切 `ra2ts_s.bik`。
            // YR 同名片在 `langmd.mix`，装载时经 `menu_movie_prefer_mix` 优先。
            movie_bik: Some(MENU_MOVIE_BIK_LARGE),
            panels: MAIN_MENU_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: MAIN_MENU_BUTTONS,
        }),
        OriginalScreen::SinglePlayerMenu => Some(UiPageSlots {
            screen,
            // 与主菜单共用壳层 chrome（安装内已证实）；按钮文案待字体链路。
            background_shp: Some("mnscrnl.shp"),
            background_pcx: None,
            background_pal: Some("shell.pal"),
            background_frame: 0,
            movie_bik: Some(MENU_MOVIE_BIK_LARGE),
            panels: MAIN_MENU_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: SINGLE_PLAYER_BUTTONS,
        }),
        OriginalScreen::Campaign => Some(UiPageSlots {
            screen,
            // 对话框 `0x94` 父背景是 `fsbkgdlg` + `fsscrn.pal`（非主菜单 `mnscrnl`）。
            // 无循环影片：左区画三侧 `fs*.shp`；右栏仍用壳层 chrome。
            background_shp: Some("fsbkgdlg.shp"),
            background_pcx: None,
            background_pal: Some("fsscrn.pal"),
            background_frame: 0,
            movie_bik: None,
            panels: CAMPAIGN_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: CAMPAIGN_BUTTONS,
        }),
        OriginalScreen::SkirmishLobby => Some(UiPageSlots {
            screen,
            // 背景仍用已证实的 `mnscrnl`；右栏去掉 WARNING / 底条。
            // 合成时叠 `sdtp` 帧 1 顶栏牌 + `sdmpbtn` 地图名底板，预览落在黑窗内。
            background_shp: Some("mnscrnl.shp"),
            background_pcx: None,
            background_pal: Some("shell.pal"),
            background_frame: 0,
            movie_bik: None,
            panels: SKIRMISH_LOBBY_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: SKIRMISH_LOBBY_BUTTONS,
        }),
        OriginalScreen::ChooseMap => Some(UiPageSlots {
            screen,
            background_shp: Some("mnscrnl.shp"),
            background_pcx: None,
            background_pal: Some("shell.pal"),
            background_frame: 0,
            movie_bik: None,
            panels: SKIRMISH_LOBBY_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: CHOOSE_MAP_BUTTONS,
        }),
        OriginalScreen::LoadScreen => Some(UiPageSlots {
            screen,
            // 默认美国人 800 宽艺术；壳层按所选国家 / 视口覆盖资源名与调色板。
            background_shp: Some("ls800ustates.shp"),
            background_pcx: None,
            background_pal: Some("mpls.pal"),
            background_frame: 0,
            movie_bik: None,
            panels: LOAD_SCREEN_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: LOAD_SCREEN_BUTTONS,
        }),
        OriginalScreen::Options => Some(UiPageSlots {
            screen,
            // 与主菜单/单人页共用已证实壳层 chrome；选项专用对话框资源未证实前不臆造。
            background_shp: Some("mnscrnl.shp"),
            background_pcx: None,
            background_pal: Some("shell.pal"),
            background_frame: 0,
            movie_bik: Some(MENU_MOVIE_BIK_LARGE),
            panels: MAIN_MENU_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: OPTIONS_BUTTONS,
        }),
        OriginalScreen::ExitConfirm => Some(UiPageSlots {
            screen,
            // 底下主菜单壳层 + 居中 `pudlgbgn` 确认框。调色板见 `pudlgbgn_palette`。
            background_shp: Some("mnscrnl.shp"),
            background_pcx: None,
            background_pal: Some("shell.pal"),
            background_frame: 0,
            movie_bik: Some(MENU_MOVIE_BIK_LARGE),
            panels: EXIT_CONFIRM_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: EXIT_CONFIRM_BUTTONS,
        }),
        OriginalScreen::Network => Some(UiPageSlots {
            screen,
            background_shp: None,
            background_pcx: None,
            background_pal: None,
            background_frame: 0,
            movie_bik: None,
            panels: &[],
            fonts: &[],
            buttons: NETWORK_BUTTONS,
        }),
        OriginalScreen::Battle => None,
        // 遭遇战积分：壳层右栏；左区战报图由 chrome `MultiplayerScore`（adaptor）注入，槽位不写死盟军皮。
        OriginalScreen::Results => Some(UiPageSlots {
            screen,
            background_shp: None,
            background_pcx: None,
            background_pal: None,
            background_frame: 0,
            movie_bik: None,
            panels: SKIRMISH_SCORE_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: SKIRMISH_SCORE_BUTTONS,
        }),
    }
}
