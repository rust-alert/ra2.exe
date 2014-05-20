//! 原版产品页的逻辑 UI 资源槽（按页面组织，不依赖 `ui.ini` 当素材目录）。
//!
//! 第一阶段对照锁定 **RA2 原版**（非默认 YR）。槽位先对齐入口 id 与命中框；
//! 具体 SHP/PAL/帧须有安装内证据后再填，禁止臆造。
//! **填了文件名 ≠ 已解码 ≠ 已 GPU 绘制 ≠ Pre-Alpha 视觉交付。**
//! 页面级资源索引见 [`crate::ui_page`]；可读性探测见 [`crate::ui_resolve`]；逻辑命中见 [`crate::ui_hit`]。

use crate::{menu_action::MenuAction, screen::OriginalScreen};

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
    /// 归一化命中框（左、上、右、下，0..1）。几何仍待与原版布局对齐。
    pub hit: (f32, f32, f32, f32),
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
        if self.background_shp.is_some() || self.background_pal.is_some() || !self.panels.is_empty() {
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
/// 常态 / 按下帧；主菜单不启用悬停换帧。
const SDBTNANM_FRAME_NORMAL: u16 = 2;
const SDBTNANM_FRAME_PRESSED: u16 = 4;

const MAIN_MENU_PANELS: &[UiPanelSlot] = &[
    UiPanelSlot { id: "right_top", shp: "sdtp.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "right_tile", shp: "sdbtnbkgd.shp", pal: "shell2.pal", frame: 0 },
    UiPanelSlot { id: "right_bottom", shp: "sdbtm.shp", pal: "shell.pal", frame: 0 },
    UiPanelSlot { id: "lower_side", shp: "lwscrnl.shp", pal: "shell.pal", frame: 0 },
];

const MAIN_MENU_FONTS: &[&str] = &["game.fnt"];

const fn main_menu_button(
    entry_id: &'static str,
    action: MenuAction,
    enabled: bool,
    hit: (f32, f32, f32, f32),
) -> UiButtonSlot {
    UiButtonSlot {
        entry_id,
        action,
        enabled,
        hit,
        anim_shp: Some(SDBTNANM_SHP),
        anim_pal: Some(SDBTNANM_PAL),
        normal_frame: Some(SDBTNANM_FRAME_NORMAL),
        // 主菜单不闪悬停帧；图集可另有 hover 帧，页面策略关闭。
        hover_frame: None,
        pressed_frame: Some(SDBTNANM_FRAME_PRESSED),
        disabled_frame: if enabled { None } else { Some(SDBTNANM_FRAME_NORMAL) },
    }
}

const fn empty_button(entry_id: &'static str, action: MenuAction, enabled: bool, hit: (f32, f32, f32, f32)) -> UiButtonSlot {
    UiButtonSlot {
        entry_id,
        action,
        enabled,
        hit,
        anim_shp: None,
        anim_pal: None,
        normal_frame: None,
        hover_frame: None,
        pressed_frame: None,
        disabled_frame: None,
    }
}

// 命中框为 800×600 内容归一化；实际点击经 `ui_layout` + fit 相机，不直接用窗口比例。
const MAIN_MENU_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("single_player", MenuAction::OpenSinglePlayer, true, (0.805, 0.3317, 1.0, 0.4017)),
    main_menu_button("network", MenuAction::OpenNetwork, false, (0.805, 0.4017, 1.0, 0.4717)),
    main_menu_button("options", MenuAction::OpenOptions, true, (0.805, 0.4717, 1.0, 0.5417)),
    main_menu_button("exit", MenuAction::Exit, true, (0.805, 0.5417, 1.0, 0.6117)),
];

// 命中框占位；实际点击走 `ui_layout` 单人页像素格。
const SINGLE_PLAYER_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("campaign", MenuAction::Noop, false, (0.805, 0.3317, 1.0, 0.4017)),
    main_menu_button("skirmish", MenuAction::OpenSkirmish, true, (0.805, 0.4017, 1.0, 0.4717)),
    main_menu_button("training", MenuAction::Noop, false, (0.805, 0.4717, 1.0, 0.5417)),
    main_menu_button("back", MenuAction::Back, true, (0.805, 0.5417, 1.0, 0.6117)),
];

const SKIRMISH_LOBBY_BUTTONS: &[UiButtonSlot] = &[
    main_menu_button("side", MenuAction::CycleSide, true, (0.805, 0.3317, 1.0, 0.4017)),
    main_menu_button("difficulty", MenuAction::CycleDifficulty, true, (0.805, 0.4017, 1.0, 0.4717)),
    main_menu_button("start", MenuAction::StartSkirmish, true, (0.805, 0.4717, 1.0, 0.5417)),
    main_menu_button("back", MenuAction::Back, true, (0.805, 0.5417, 1.0, 0.6117)),
];

const LOAD_SCREEN_BUTTONS: &[UiButtonSlot] = &[
    empty_button("loading", MenuAction::Noop, false, (0.30, 0.40, 0.74, 0.48)),
    empty_button("retry", MenuAction::RetryLoad, true, (0.30, 0.52, 0.50, 0.60)),
    empty_button("cancel", MenuAction::CancelLoad, true, (0.54, 0.52, 0.74, 0.60)),
];

const OPTIONS_BUTTONS: &[UiButtonSlot] = &[
    empty_button("audio", MenuAction::Noop, false, (0.28, 0.28, 0.72, 0.36)),
    empty_button("video", MenuAction::Noop, false, (0.28, 0.40, 0.72, 0.48)),
    empty_button("back", MenuAction::Back, true, (0.28, 0.60, 0.72, 0.68)),
];

const NETWORK_BUTTONS: &[UiButtonSlot] = &[
    empty_button("online", MenuAction::Noop, false, (0.22, 0.36, 0.78, 0.44)),
    empty_button("back", MenuAction::Back, true, (0.22, 0.56, 0.78, 0.64)),
];

/// 返回某原版产品页的逻辑槽位；对局/结算无前置菜单槽。
pub fn slots_for(screen: OriginalScreen) -> Option<UiPageSlots> {
    match screen {
        OriginalScreen::Splash => Some(UiPageSlots {
            screen,
            // 闪屏专用 SHP 待安装内证据后再填；先占位以便页面机存在。
            background_shp: None,
            background_pal: None,
            background_frame: 0,
            movie_bik: None,
            panels: &[],
            fonts: MAIN_MENU_FONTS,
            buttons: &[],
        }),
        OriginalScreen::MainMenu => Some(UiPageSlots {
            screen,
            // 非 640 宽窗口默认大背景；640 分支后续按视口另选 `mnscrns.shp`。
            background_shp: Some("mnscrnl.shp"),
            background_pal: Some("shell.pal"),
            background_frame: 0,
            // 大布局默认 `ra2ts_l.bik`；640 窄布局后续切 `ra2ts_s.bik`。
            movie_bik: Some("ra2ts_l.bik"),
            panels: MAIN_MENU_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: MAIN_MENU_BUTTONS,
        }),
        OriginalScreen::SinglePlayerMenu => Some(UiPageSlots {
            screen,
            // 与主菜单共用壳层 chrome（安装内已证实）；按钮文案待字体链路。
            background_shp: Some("mnscrnl.shp"),
            background_pal: Some("shell.pal"),
            background_frame: 0,
            movie_bik: Some("ra2ts_l.bik"),
            panels: MAIN_MENU_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: SINGLE_PLAYER_BUTTONS,
        }),
        OriginalScreen::SkirmishLobby => Some(UiPageSlots {
            screen,
            // Pre-Alpha：与主菜单/单人页共用已证实壳层 chrome；大厅专用板面后续再换。
            background_shp: Some("mnscrnl.shp"),
            background_pal: Some("shell.pal"),
            background_frame: 0,
            movie_bik: None,
            panels: MAIN_MENU_PANELS,
            fonts: MAIN_MENU_FONTS,
            buttons: SKIRMISH_LOBBY_BUTTONS,
        }),
        OriginalScreen::LoadScreen => Some(UiPageSlots {
            screen,
            background_shp: None,
            background_pal: None,
            background_frame: 0,
            movie_bik: None,
            panels: &[],
            fonts: &[],
            buttons: LOAD_SCREEN_BUTTONS,
        }),
        OriginalScreen::Options => Some(UiPageSlots {
            screen,
            background_shp: None,
            background_pal: None,
            background_frame: 0,
            movie_bik: None,
            panels: &[],
            fonts: &[],
            buttons: OPTIONS_BUTTONS,
        }),
        OriginalScreen::Network => Some(UiPageSlots {
            screen,
            background_shp: None,
            background_pal: None,
            background_frame: 0,
            movie_bik: None,
            panels: &[],
            fonts: &[],
            buttons: NETWORK_BUTTONS,
        }),
        OriginalScreen::Match | OriginalScreen::Results => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_entries_match_expected_ids() {
        let page = slots_for(OriginalScreen::MainMenu).unwrap();
        let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
        assert_eq!(ids, ["single_player", "network", "options", "exit"]);
        assert!(!page.buttons[1].enabled);
        assert!(page.has_any_asset_name());
        assert_eq!(page.background_shp, Some("mnscrnl.shp"));
        assert_eq!(page.background_pal, Some("shell.pal"));
        assert_eq!(page.movie_bik, Some("ra2ts_l.bik"));
        assert!(page.buttons[0].hover_frame.is_none());
        assert_eq!(page.buttons[0].normal_frame, Some(2));
        assert_eq!(page.buttons[0].pressed_frame, Some(4));
        assert!(page.panels.iter().any(|p| p.shp == "sdtp.shp"));
        assert_eq!(page.fonts, &["game.fnt"]);
    }

    #[test]
    fn single_player_keeps_disabled_campaign_slot() {
        let page = slots_for(OriginalScreen::SinglePlayerMenu).unwrap();
        assert_eq!(page.buttons[0].entry_id, "campaign");
        assert!(!page.buttons[0].enabled);
        assert!(matches!(page.buttons[0].action, MenuAction::Noop));
        assert!(matches!(page.buttons[2].action, MenuAction::Noop));
        assert!(page.buttons.iter().any(|b| b.entry_id == "skirmish" && b.enabled));
        assert!(page.has_any_asset_name());
        assert_eq!(page.background_shp, Some("mnscrnl.shp"));
        assert_eq!(page.buttons[1].normal_frame, Some(2));
        assert_eq!(page.buttons[1].pressed_frame, Some(4));
    }

    #[test]
    fn load_screen_exposes_retry_and_cancel_slots() {
        let page = slots_for(OriginalScreen::LoadScreen).unwrap();
        let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
        assert_eq!(ids, ["loading", "retry", "cancel"]);
        let retry = page.buttons.iter().find(|b| b.entry_id == "retry").unwrap();
        assert!(retry.enabled);
        assert!(matches!(retry.action, MenuAction::RetryLoad));
        let cancel = page.buttons.iter().find(|b| b.entry_id == "cancel").unwrap();
        assert!(cancel.enabled);
        assert!(matches!(cancel.action, MenuAction::CancelLoad));
    }

    #[test]
    fn options_keeps_disabled_audio_video_slots() {
        let page = slots_for(OriginalScreen::Options).unwrap();
        let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
        assert_eq!(ids, ["audio", "video", "back"]);
        assert!(!page.buttons[0].enabled);
        assert!(!page.buttons[1].enabled);
        assert!(page.buttons[2].enabled);
    }

    #[test]
    fn skirmish_lobby_exposes_side_and_difficulty() {
        let page = slots_for(OriginalScreen::SkirmishLobby).unwrap();
        let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
        assert_eq!(ids, ["side", "difficulty", "start", "back"]);
        assert!(matches!(page.buttons[0].action, MenuAction::CycleSide));
        assert!(matches!(page.buttons[1].action, MenuAction::CycleDifficulty));
        assert!(page.has_any_asset_name());
        assert_eq!(page.background_shp, Some("mnscrnl.shp"));
        assert_eq!(page.buttons[0].normal_frame, Some(2));
        assert_eq!(page.buttons[0].pressed_frame, Some(4));
        assert!(page.panels.iter().any(|p| p.shp == "sdtp.shp"));
        assert_eq!(page.fonts, &["game.fnt"]);
    }
}
