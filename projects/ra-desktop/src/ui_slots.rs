//! 原版产品页的逻辑 UI 资源槽（按页面组织，不依赖 `ui.ini` 当素材目录）。
//!
//! 槽位先对齐入口页面的入口 id 与占位命中框；具体 SHP/PAL 文件名可后填。
//! **空文件名 ≠ 已交付原版 UI。**

use crate::{
    menu_view::MenuAction,
    screen::OriginalScreen,
};

/// 单个按钮/入口的资源与命中约定。
#[derive(Debug, Clone)]
pub struct UiButtonSlot {
    /// 与机读 UI 状态一致的入口 id（如 `single_player`）。
    pub entry_id: &'static str,
    /// 壳层导航动作。
    pub action: MenuAction,
    /// 是否可点（未实现模式保留位置但禁用）。
    pub enabled: bool,
    /// 归一化命中框（左、上、右、下，0..1）。
    pub hit: (f32, f32, f32, f32),
    /// 常态 SHP（可空；接线前保持 `None`）。
    #[allow(dead_code)]
    pub normal_shp: Option<&'static str>,
    /// 悬停 SHP（可空）。
    #[allow(dead_code)]
    pub hover_shp: Option<&'static str>,
    /// 按下 SHP（可空）。
    #[allow(dead_code)]
    pub pressed_shp: Option<&'static str>,
    /// 禁用 SHP（可空）。
    #[allow(dead_code)]
    pub disabled_shp: Option<&'static str>,
}

/// 一页的逻辑资源描述。
#[derive(Debug, Clone)]
pub struct UiPageSlots {
    /// 原版产品页。
    pub screen: OriginalScreen,
    /// 背景 SHP（可空）。
    #[allow(dead_code)]
    pub background_shp: Option<&'static str>,
    /// 背景调色板（可空；常见为独立 PAL）。
    #[allow(dead_code)]
    pub background_pal: Option<&'static str>,
    /// 页面入口。
    pub buttons: &'static [UiButtonSlot],
}

impl UiPageSlots {
    /// 是否已为任一槽填了具体文件名（用于区分「模型」与「已接线资产」）。
    #[allow(dead_code)]
    pub fn has_any_asset_name(&self) -> bool {
        if self.background_shp.is_some() || self.background_pal.is_some() {
            return true;
        }
        self.buttons.iter().any(|b| {
            b.normal_shp.is_some()
                || b.hover_shp.is_some()
                || b.pressed_shp.is_some()
                || b.disabled_shp.is_some()
        })
    }
}

const MAIN_MENU_BUTTONS: &[UiButtonSlot] = &[
    UiButtonSlot {
        entry_id: "single_player",
        action: MenuAction::OpenSinglePlayer,
        enabled: true,
        hit: (0.28, 0.32, 0.72, 0.40),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "network",
        action: MenuAction::OpenNetwork,
        enabled: false,
        hit: (0.28, 0.44, 0.72, 0.52),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "options",
        action: MenuAction::OpenOptions,
        enabled: true,
        hit: (0.28, 0.56, 0.72, 0.64),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "exit",
        action: MenuAction::Exit,
        enabled: true,
        hit: (0.28, 0.68, 0.72, 0.76),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
];

const SINGLE_PLAYER_BUTTONS: &[UiButtonSlot] = &[
    UiButtonSlot {
        entry_id: "campaign",
        action: MenuAction::Back,
        enabled: false,
        hit: (0.28, 0.30, 0.72, 0.38),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "skirmish",
        action: MenuAction::OpenSkirmish,
        enabled: true,
        hit: (0.28, 0.42, 0.72, 0.50),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "training",
        action: MenuAction::Back,
        enabled: false,
        hit: (0.28, 0.54, 0.72, 0.62),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "back",
        action: MenuAction::Back,
        enabled: true,
        hit: (0.28, 0.68, 0.72, 0.76),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
];

const SKIRMISH_LOBBY_BUTTONS: &[UiButtonSlot] = &[
    UiButtonSlot {
        entry_id: "side",
        action: MenuAction::CycleSide,
        enabled: true,
        hit: (0.18, 0.68, 0.48, 0.75),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "difficulty",
        action: MenuAction::CycleDifficulty,
        enabled: true,
        hit: (0.52, 0.68, 0.82, 0.75),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "start",
        action: MenuAction::StartSkirmish,
        enabled: true,
        hit: (0.28, 0.80, 0.72, 0.87),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "back",
        action: MenuAction::Back,
        enabled: true,
        hit: (0.28, 0.90, 0.72, 0.97),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
];

const LOAD_SCREEN_BUTTONS: &[UiButtonSlot] = &[
    UiButtonSlot {
        entry_id: "loading",
        action: MenuAction::CancelLoad,
        enabled: false,
        hit: (0.30, 0.40, 0.74, 0.48),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "cancel",
        action: MenuAction::CancelLoad,
        enabled: true,
        hit: (0.30, 0.56, 0.74, 0.64),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
];

const OPTIONS_BUTTONS: &[UiButtonSlot] = &[
    UiButtonSlot {
        entry_id: "audio",
        action: MenuAction::Noop,
        enabled: false,
        hit: (0.28, 0.28, 0.72, 0.36),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "video",
        action: MenuAction::Noop,
        enabled: false,
        hit: (0.28, 0.40, 0.72, 0.48),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "back",
        action: MenuAction::Back,
        enabled: true,
        hit: (0.28, 0.60, 0.72, 0.68),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
];

const NETWORK_BUTTONS: &[UiButtonSlot] = &[
    UiButtonSlot {
        entry_id: "online",
        action: MenuAction::Noop,
        enabled: false,
        hit: (0.22, 0.36, 0.78, 0.44),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
    UiButtonSlot {
        entry_id: "back",
        action: MenuAction::Back,
        enabled: true,
        hit: (0.22, 0.56, 0.78, 0.64),
        normal_shp: None,
        hover_shp: None,
        pressed_shp: None,
        disabled_shp: None,
    },
];

/// 返回某原版产品页的逻辑槽位；对局/结算无前置菜单槽。
pub fn slots_for(screen: OriginalScreen) -> Option<UiPageSlots> {
    let (buttons, bg, pal) = match screen {
        OriginalScreen::MainMenu => (MAIN_MENU_BUTTONS, None, None),
        OriginalScreen::SinglePlayerMenu => (SINGLE_PLAYER_BUTTONS, None, None),
        OriginalScreen::SkirmishLobby => (SKIRMISH_LOBBY_BUTTONS, None, None),
        OriginalScreen::LoadScreen => (LOAD_SCREEN_BUTTONS, None, None),
        OriginalScreen::Options => (OPTIONS_BUTTONS, None, None),
        OriginalScreen::Network => (NETWORK_BUTTONS, None, None),
        OriginalScreen::Match | OriginalScreen::Results => return None,
    };
    Some(UiPageSlots {
        screen,
        background_shp: bg,
        background_pal: pal,
        buttons,
    })
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
        assert!(!page.has_any_asset_name());
    }

    #[test]
    fn single_player_keeps_disabled_campaign_slot() {
        let page = slots_for(OriginalScreen::SinglePlayerMenu).unwrap();
        assert_eq!(page.buttons[0].entry_id, "campaign");
        assert!(!page.buttons[0].enabled);
        assert!(page.buttons.iter().any(|b| b.entry_id == "skirmish" && b.enabled));
    }

    #[test]
    fn load_screen_exposes_cancel_slot() {
        let page = slots_for(OriginalScreen::LoadScreen).unwrap();
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
    }
}
