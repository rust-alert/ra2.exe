//! 与后端无关的渲染计划（几何来自 `LayoutSnapshot`）。

use ra_layout::{
    solve_battle_hud, solve_battle_pause, solve_campaign, solve_choose_map, solve_exit_confirm,
    solve_load_screen, solve_network_page, solve_options_page, solve_shell_page, solve_skirmish_lobby,
    CAMPAIGN_BUTTON_IDS, CHOOSE_MAP_BUTTON_IDS, EXIT_CONFIRM_BUTTON_IDS, LOAD_SCREEN_BUTTON_IDS,
    LayoutId, LayoutSnapshot, MAIN_MENU_BUTTON_IDS, NETWORK_BUTTON_IDS, OPTIONS_BUTTON_IDS, Rect,
    SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_LOBBY_BUTTON_IDS,
};

use crate::OriginalScreen;

/// 单条可绘制命令。
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommand {
    /// 纯色矩形（占位 / 调试 / 未绑资源时）。
    SolidRect {
        /// 与 snapshot 对齐的控件 id。
        id: LayoutId,
        /// 视口矩形（与 snapshot 同源）。
        rect: Rect,
        /// RGBA。
        color: [u8; 4],
    },
}

/// 一帧的有序绘制计划。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderPlan {
    /// 按绘制序排列的命令。
    pub commands: Vec<RenderCommand>,
}

impl RenderPlan {
    /// 由快照生成占位色块计划（跳过根容器 id）。
    pub fn solid_placeholders_from_snapshot(snapshot: &LayoutSnapshot, root_id: &str) -> Self {
        let commands = snapshot
            .elements
            .iter()
            .filter(|e| e.id.0 != root_id)
            .map(|e| RenderCommand::SolidRect {
                id: e.id.clone(),
                rect: e.layout.rect,
                color: [80, 80, 80, 255],
            })
            .collect();
        Self { commands }
    }

    /// 按控件 id 查找命令矩形（与 snapshot 同源校验用）。
    pub fn rect_of(&self, id: &str) -> Option<Rect> {
        self.commands.iter().find_map(|cmd| match cmd {
            RenderCommand::SolidRect { id: cid, rect, .. } if cid.0 == id => Some(*rect),
            RenderCommand::SolidRect { .. } => None,
        })
    }

    /// 丢掉指定 id 的命令（例如 HUD 战术区底边 `command_bar`）。
    pub fn excluding_ids(&self, skip: &[&str]) -> Self {
        Self {
            commands: self
                .commands
                .iter()
                .filter(|cmd| match cmd {
                    RenderCommand::SolidRect { id, .. } => {
                        !skip.iter().any(|s| id.0.as_str() == *s)
                    }
                })
                .cloned()
                .collect(),
        }
    }

    /// 只保留指定 id 的命令。
    pub fn retaining_ids(&self, keep: &[&str]) -> Self {
        Self {
            commands: self
                .commands
                .iter()
                .filter(|cmd| match cmd {
                    RenderCommand::SolidRect { id, .. } => {
                        keep.iter().any(|s| id.0.as_str() == *s)
                    }
                })
                .cloned()
                .collect(),
        }
    }

    /// 覆盖指定 id 的纯色（诊断高亮右栏按钮等）。
    pub fn recolor_ids(&self, ids: &[&str], color: [u8; 4]) -> Self {
        Self {
            commands: self
                .commands
                .iter()
                .map(|cmd| match cmd {
                    RenderCommand::SolidRect {
                        id,
                        rect,
                        color: prev,
                    } => {
                        let color = if ids.iter().any(|s| id.0.as_str() == *s) {
                            color
                        } else {
                            *prev
                        };
                        RenderCommand::SolidRect {
                            id: id.clone(),
                            rect: *rect,
                            color,
                        }
                    }
                })
                .collect(),
        }
    }

    /// 对局 HUD：`solve_battle_hud` → snapshot → 占位 `RenderPlan`。
    pub fn battle_hud_placeholders(viewport_w: u32, viewport_h: u32) -> Self {
        let snap = solve_battle_hud(viewport_w, viewport_h);
        Self::solid_placeholders_from_snapshot(&snap, "battle_hud")
    }

    /// 壳层页：`solve_shell_page` → snapshot → 占位 `RenderPlan`。
    pub fn shell_page_placeholders(
        root_id: &str,
        stacked_ids: &[&str],
        bottom_id: Option<&str>,
    ) -> Self {
        let snap = solve_shell_page(root_id, stacked_ids, bottom_id);
        Self::solid_placeholders_from_snapshot(&snap, root_id)
    }

    /// 退出确认：`solve_exit_confirm` → snapshot → 占位 `RenderPlan`。
    pub fn exit_confirm_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_exit_confirm(), "exit_confirm")
    }

    /// 壳层对话框模板：已知 id 走 `solve_choose_map` / `solve_skirmish_lobby`。
    pub fn shell_dialog_placeholders(dialog_id: u16, root_id: &str) -> Self {
        match dialog_id {
            0x6B => {
                assert_eq!(root_id, "dialog_0x6b");
                Self::choose_map_placeholders()
            }
            0x102 => {
                assert_eq!(root_id, "dialog_0x102");
                Self::skirmish_lobby_placeholders()
            }
            other => panic!("unsupported shell dialog {other:#x}"),
        }
    }

    /// 选图页：`solve_choose_map` → snapshot → 占位 `RenderPlan`。
    pub fn choose_map_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_choose_map(), "dialog_0x6b")
    }

    /// 遭遇战大厅：`solve_skirmish_lobby` → snapshot → 占位 `RenderPlan`。
    pub fn skirmish_lobby_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_skirmish_lobby(), "dialog_0x102")
    }

    /// 战役页：`solve_campaign` → snapshot → 占位 `RenderPlan`。
    pub fn campaign_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_campaign(), "campaign")
    }

    /// 装载页：`solve_load_screen` → snapshot → 占位 `RenderPlan`。
    pub fn load_screen_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_load_screen(), "load_screen")
    }

    /// 网络占位页：`solve_network_page` → snapshot → 占位 `RenderPlan`。
    pub fn network_page_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_network_page(), "network")
    }

    /// 选项整页：`solve_options_page` → snapshot → 占位 `RenderPlan`。
    pub fn options_page_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_options_page(), "options")
    }

    /// 对局暂停菜单：`solve_battle_pause` → snapshot → 占位 `RenderPlan`。
    pub fn battle_pause_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_battle_pause(), "battle_pause")
    }

    /// 壳层前置页诊断占位；闪屏 / 对局 / 结算无计划。
    pub fn for_original_screen(screen: OriginalScreen) -> Option<Self> {
        Some(match screen {
            OriginalScreen::MainMenu => Self::shell_page_placeholders(
                "main_menu",
                &MAIN_MENU_BUTTON_IDS[..5],
                Some(MAIN_MENU_BUTTON_IDS[5]),
            ),
            OriginalScreen::SinglePlayerMenu => Self::shell_page_placeholders(
                "single_player",
                &SINGLE_PLAYER_BUTTON_IDS[..3],
                Some(SINGLE_PLAYER_BUTTON_IDS[3]),
            ),
            OriginalScreen::Campaign => Self::campaign_placeholders(),
            OriginalScreen::SkirmishLobby => Self::skirmish_lobby_placeholders(),
            OriginalScreen::ChooseMap => Self::choose_map_placeholders(),
            OriginalScreen::Options => Self::options_page_placeholders(),
            OriginalScreen::ExitConfirm => Self::exit_confirm_placeholders(),
            OriginalScreen::LoadScreen => Self::load_screen_placeholders(),
            OriginalScreen::Network => Self::network_page_placeholders(),
            OriginalScreen::Splash | OriginalScreen::Battle | OriginalScreen::Results => {
                return None;
            }
        })
    }

    /// 诊断栅格化用：去掉满幅背景 / 影片层，并高亮页内主按钮。
    pub fn diagnostic_for_original_screen(screen: OriginalScreen) -> Option<Self> {
        let rail = diagnostic_button_ids(screen)?;
        Some(
            Self::for_original_screen(screen)?
                .excluding_ids(&["background", "movie"])
                .recolor_ids(rail, [196, 148, 48, 255]),
        )
    }
}

/// 诊断高亮用的主按钮 id（与各页 `*_BUTTON_IDS` 对齐）。
fn diagnostic_button_ids(screen: OriginalScreen) -> Option<&'static [&'static str]> {
    Some(match screen {
        OriginalScreen::MainMenu => &MAIN_MENU_BUTTON_IDS,
        OriginalScreen::SinglePlayerMenu => &SINGLE_PLAYER_BUTTON_IDS,
        OriginalScreen::Campaign => &CAMPAIGN_BUTTON_IDS,
        OriginalScreen::SkirmishLobby => &SKIRMISH_LOBBY_BUTTON_IDS,
        OriginalScreen::ChooseMap => &CHOOSE_MAP_BUTTON_IDS,
        OriginalScreen::Options => &OPTIONS_BUTTON_IDS,
        OriginalScreen::ExitConfirm => &EXIT_CONFIRM_BUTTON_IDS,
        OriginalScreen::LoadScreen => &LOAD_SCREEN_BUTTON_IDS,
        OriginalScreen::Network => &NETWORK_BUTTON_IDS,
        OriginalScreen::Splash | OriginalScreen::Battle | OriginalScreen::Results => {
            return None;
        }
    })
}
