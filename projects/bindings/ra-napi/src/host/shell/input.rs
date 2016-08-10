//! 指针命中与菜单前按键。

use ra_layout;
use ra_widgets::{input::hit, original_screen::OriginalScreen, skirmish_setup::hover_entry_at};
use winit::{
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

use super::Shell;

impl Shell {
    pub(super) fn shell_cursor_px(&self) -> (i32, i32) {
        ra_layout::window_to_shell_px(self.cursor.0, self.cursor.1, self.window_width, self.window_height)
    }

    /// 当前光标下的可点按钮入口（逻辑窗口坐标）。
    pub(super) fn menu_entry_under_cursor(&self) -> Option<&'static str> {
        if self.screen == OriginalScreen::Campaign {
            return hit::campaign_entry_at(self.cursor.0, self.cursor.1, self.window_width, self.window_height);
        }
        let hit_maps = self.maps_for_menu_hit();
        if self.screen == OriginalScreen::ChooseMap {
            return hit::choose_map_entry_at(
                &hit_maps,
                self.lobby_modes.len(),
                self.map_list_scroll,
                self.cursor.0,
                self.cursor.1,
                self.window_width,
                self.window_height,
            );
        }
        if self.screen == OriginalScreen::SkirmishLobby {
            if let Some(id) = hit::hover_index(
                self.screen,
                &hit_maps,
                self.lobby_modes.len(),
                self.selected_map.as_deref(),
                self.cursor,
                self.window_width,
                self.window_height,
                self.map_list_scroll,
                self.load_allow_retry(),
            )
            .and_then(|idx| ra_layout::SKIRMISH_LOBBY_BUTTON_IDS.get(idx).copied())
            {
                return Some(id);
            }
            let (x, y) = self.shell_cursor_px();
            return hover_entry_at(x, y);
        }
        let idx = hit::hover_index(
            self.screen,
            &hit_maps,
            self.lobby_modes.len(),
            self.selected_map.as_deref(),
            self.cursor,
            self.window_width,
            self.window_height,
            self.map_list_scroll,
            self.load_allow_retry(),
        )?;
        match self.screen {
            OriginalScreen::MainMenu => ra_layout::MAIN_MENU_BUTTON_IDS.get(idx).copied(),
            OriginalScreen::SinglePlayerMenu => ra_layout::SINGLE_PLAYER_BUTTON_IDS.get(idx).copied(),
            OriginalScreen::Options => ra_layout::OPTIONS_BUTTON_IDS.get(idx).copied(),
            OriginalScreen::ExitConfirm => ra_layout::EXIT_CONFIRM_BUTTON_IDS.get(idx).copied(),
            _ => None,
        }
    }

    pub(super) fn handle_pre_game_key(&mut self, _event_loop: &ActiveEventLoop, key: PhysicalKey) {
        if matches!(key, PhysicalKey::Code(KeyCode::F12)) {
            self.queue_screenshot(self.screen.as_str());
            return;
        }
        match self.screen {
            OriginalScreen::Splash => {
                // 只打跳过标；状态机在预处理完成后切主菜单。
                if matches!(
                    key,
                    PhysicalKey::Code(KeyCode::Escape) | PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter)
                ) {
                    self.request_splash_skip();
                }
            }
            OriginalScreen::MainMenu => {
                // 主菜单导航靠鼠标点按钮；勿用 Enter/N/O 等字母键发明快捷入口。
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.banner = "确认退出？".into();
                    self.set_screen(OriginalScreen::ExitConfirm);
                }
            }
            OriginalScreen::SinglePlayerMenu => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.set_screen(OriginalScreen::MainMenu);
                }
            }
            OriginalScreen::Campaign => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.set_screen(OriginalScreen::SinglePlayerMenu);
                }
            }
            OriginalScreen::SkirmishLobby => match key {
                PhysicalKey::Code(KeyCode::Escape) if self.skirmish.open_combo.is_some() => {
                    self.skirmish.close_combo();
                    self.refresh_menu_backdrop();
                }
                // 大厅改阵营/地图/开局靠鼠标；勿用 Enter/方向键/Q/E 发明快捷键。
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::SinglePlayerMenu),
                _ => {}
            },
            OriginalScreen::ChooseMap => {
                // 选图列表滚动靠鼠标；仅 Escape 取消。
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.cancel_choose_map();
                }
            }
            OriginalScreen::Network | OriginalScreen::Options => {
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    if self.screen == OriginalScreen::Options {
                        self.discard_options_draft();
                    }
                    self.set_screen(OriginalScreen::MainMenu);
                }
            }
            OriginalScreen::ExitConfirm => {
                // 确认退出靠按钮；勿用 Enter 发明快捷确认。
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.set_screen(OriginalScreen::MainMenu);
                }
            }
            OriginalScreen::LoadScreen => {
                // 加载失败重试靠界面按钮；勿用 Enter 发明快捷键。
                if matches!(key, PhysicalKey::Code(KeyCode::Escape)) {
                    self.cancel_load();
                }
            }
            OriginalScreen::Battle | OriginalScreen::Results => {}
        }
    }
}
