//! 指针命中与菜单前按键。

use ra_layout;
use ra_widgets::menu_action::MenuAction;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::options_dialog::OptionsHit;
use ra_widgets::skirmish_setup::hover_entry_at;
use ra_widgets::input::hit;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};

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
            OriginalScreen::ChooseMap => {
                let n_btn = ra_layout::CHOOSE_MAP_BUTTON_IDS.len();
                if let Some(id) = ra_layout::CHOOSE_MAP_BUTTON_IDS.get(idx).copied() {
                    return Some(id);
                }
                let mode_base = n_btn;
                let mode_count = self.lobby_modes.len();
                if idx < mode_base + mode_count {
                    Some("mode_row")
                } else if idx < mode_base + mode_count + self.maps_matching_selected_mode().len() {
                    Some("map_row")
                } else {
                    Some("map_preview")
                }
            }
            _ => None,
        }
    }

    pub(super) fn handle_pre_game_key(&mut self, event_loop: &ActiveEventLoop, key: PhysicalKey) {
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
            OriginalScreen::MainMenu => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    self.set_screen(OriginalScreen::SinglePlayerMenu);
                }
                PhysicalKey::Code(KeyCode::KeyN) => {
                    tracing::info!("网络入口未开放（Beta）");
                    self.set_screen(OriginalScreen::Network);
                }
                PhysicalKey::Code(KeyCode::KeyO) => self.set_screen(OriginalScreen::Options),
                PhysicalKey::Code(KeyCode::Escape) => {
                    self.banner = "确认退出？".into();
                    self.set_screen(OriginalScreen::ExitConfirm);
                }
                _ => {}
            },
            OriginalScreen::SinglePlayerMenu => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) | PhysicalKey::Code(KeyCode::KeyS) => {
                    self.ensure_lobby_maps();
                    self.set_screen(OriginalScreen::SkirmishLobby);
                }
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::MainMenu),
                _ => {}
            },
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
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    self.begin_skirmish_load();
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) => self.cycle_lobby_map(-1),
                PhysicalKey::Code(KeyCode::ArrowRight) => self.cycle_lobby_map(1),
                PhysicalKey::Code(KeyCode::Home) => self.jump_lobby_map_edge(false),
                PhysicalKey::Code(KeyCode::End) => self.jump_lobby_map_edge(true),
                PhysicalKey::Code(KeyCode::KeyQ) => {
                    self.skirmish.cycle_side();
                    self.banner = format!("阵营 · {}", self.skirmish.side);
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
                PhysicalKey::Code(KeyCode::KeyE) => {
                    self.skirmish.cycle_difficulty();
                    self.banner = format!("难度 · {}", self.skirmish.difficulty);
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::SinglePlayerMenu),
                _ => {}
            },
            OriginalScreen::ChooseMap => {
                match key {
                    PhysicalKey::Code(KeyCode::Escape) => self.cancel_choose_map(),
                    PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::PageUp) => {
                        self.nudge_map_list_scroll(-1);
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::PageDown) => {
                        self.nudge_map_list_scroll(1);
                    }
                    _ => {}
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
            OriginalScreen::ExitConfirm => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    self.apply_menu_action(event_loop, MenuAction::ConfirmExit);
                }
                PhysicalKey::Code(KeyCode::Escape) => self.set_screen(OriginalScreen::MainMenu),
                _ => {}
            },
            OriginalScreen::LoadScreen => match key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                    self.retry_load();
                }
                PhysicalKey::Code(KeyCode::Escape) => self.cancel_load(),
                _ => {}
            },
            OriginalScreen::Battle | OriginalScreen::Results => {}
        }
    }
}
