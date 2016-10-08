//! 对局暂停层：主菜单 / 放弃确认 / 局内选项。

use ra_widgets::{
    battle_abort_confirm::{self, BattleAbortConfirmHit},
    battle_in_game_options::{self, BattleInGameOptionsHit, BattleInGameOptionsState},
    battle_pause_layer::{BattlePauseLayer, EscapeRoute},
    battle_pause_menu::{self, BattlePauseMenuHit},
};
use winit::{event::ElementState, window::Window};

use super::super::battle_input::LeftGesture;

use super::{BattleController, BattleNav};

impl BattleController {
    pub(super) fn clear_pause_menu_input(&mut self) {
        self.pause_hover = None;
        self.pause_pressed = None;
        self.leave_armed = false;
        self.pause_layer = BattlePauseLayer::Menu;
        self.in_game_options.drag_track = None;
        self.pause_stub_notice = None;
        self.command_hover = None;
        self.command_pressed = None;
        self.left_gesture = LeftGesture::Idle;
    }

    /// 打开暂停主菜单（已暂停时切到 Menu 层）。
    pub(super) fn open_pause_menu_layer(&mut self) {
        self.pause_layer = BattlePauseLayer::Menu;
        self.leave_armed = false;
        self.in_game_options.drag_track = None;
        self.pause_stub_notice = None;
        self.pause_hover = None;
        self.pause_pressed = None;
    }

    /// 打开局内选项层（对局须已暂停）。
    pub(super) fn open_in_game_options_layer(&mut self) {
        self.pause_layer = BattlePauseLayer::InGameOptions;
        self.leave_armed = false;
        self.in_game_options.drag_track = None;
        self.pause_stub_notice = None;
        self.pause_hover = None;
        self.pause_pressed = None;
    }

    pub(super) fn refresh_pause_hover(&mut self, window: &Window) {
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        self.pause_hover = match self.pause_layer {
            BattlePauseLayer::Menu => battle_pause_menu::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
            BattlePauseLayer::AbortConfirm => battle_abort_confirm::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
            BattlePauseLayer::InGameOptions => battle_in_game_options::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
        };
    }

    pub(super) fn handle_pause_menu_mouse(&mut self, state: ElementState, window: &Window) -> BattleNav {
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        match state {
            ElementState::Pressed => {
                self.pause_pressed = match self.pause_layer {
                    BattlePauseLayer::Menu => battle_pause_menu::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
                    BattlePauseLayer::AbortConfirm => battle_abort_confirm::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
                    BattlePauseLayer::InGameOptions => {
                        let hit = battle_in_game_options::hit_at(w, h, x, y);
                        if let Some(hit) = hit {
                            self.on_in_game_options_press(hit, w, h, x);
                        }
                        hit.map(|h| h.entry_id())
                    }
                };
                BattleNav::None
            }
            ElementState::Released => {
                if self.pause_layer == BattlePauseLayer::InGameOptions {
                    self.in_game_options.drag_track = None;
                }
                let pressed = self.pause_pressed.take();
                let hit_id = match self.pause_layer {
                    BattlePauseLayer::Menu => battle_pause_menu::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
                    BattlePauseLayer::AbortConfirm => battle_abort_confirm::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
                    BattlePauseLayer::InGameOptions => battle_in_game_options::hit_at(w, h, x, y).map(|hit| hit.entry_id()),
                };
                if pressed.is_some_and(|id| hit_id == Some(id)) {
                    if let Some(id) = hit_id {
                        return self.on_pause_layer_click(id);
                    }
                }
                BattleNav::None
            }
        }
    }

    /// 暂停层拖动滑条。
    pub(super) fn handle_pause_layer_drag(&mut self, window: &Window) {
        if self.pause_layer != BattlePauseLayer::InGameOptions {
            return;
        }
        let Some(track_id) = self.in_game_options.drag_track
        else {
            return;
        };
        let size = window.inner_size();
        let snap = battle_in_game_options::options_snapshot(size.width.max(1), size.height.max(1));
        let track = ra_layout::rect_px_from_snapshot(&snap, track_id);
        let value = battle_in_game_options::track_value_at(track, self.cursor.0 as i32, 6);
        match track_id {
            "track_game_speed" => self.in_game_options.game_speed = value,
            "track_scroll_rate" => self.in_game_options.scroll_rate = value,
            _ => {}
        }
    }

    fn on_in_game_options_press(&mut self, hit: BattleInGameOptionsHit, viewport_w: u32, viewport_h: u32, x: i32) {
        match hit {
            BattleInGameOptionsHit::TrackGameSpeed | BattleInGameOptionsHit::TrackScrollRate => {
                let id = hit.entry_id();
                self.in_game_options.drag_track = Some(id);
                let snap = battle_in_game_options::options_snapshot(viewport_w, viewport_h);
                let track = ra_layout::rect_px_from_snapshot(&snap, id);
                let value = battle_in_game_options::track_value_at(track, x, 6);
                if id == "track_game_speed" {
                    self.in_game_options.game_speed = value;
                } else {
                    self.in_game_options.scroll_rate = value;
                }
            }
            BattleInGameOptionsHit::CheckTargetLines => {
                self.in_game_options.target_lines = !self.in_game_options.target_lines;
            }
            BattleInGameOptionsHit::CheckShowHidden => {
                self.in_game_options.show_hidden = !self.in_game_options.show_hidden;
            }
            BattleInGameOptionsHit::CheckTooltips => {
                self.in_game_options.tooltips = !self.in_game_options.tooltips;
            }
            _ => {}
        }
    }

    fn on_pause_layer_click(&mut self, entry_id: &str) -> BattleNav {
        match self.pause_layer {
            BattlePauseLayer::Menu => {
                if let Some(hit) = BattlePauseMenuHit::from_entry_id(entry_id) {
                    return self.on_pause_menu_hit(hit);
                }
            }
            BattlePauseLayer::AbortConfirm => {
                if let Some(hit) = BattleAbortConfirmHit::from_entry_id(entry_id) {
                    return self.on_abort_confirm_hit(hit);
                }
            }
            BattlePauseLayer::InGameOptions => {
                if let Some(hit) = BattleInGameOptionsHit::from_entry_id(entry_id) {
                    return self.on_in_game_options_hit(hit);
                }
            }
        }
        BattleNav::None
    }

    pub(super) fn on_pause_menu_hit(&mut self, hit: BattlePauseMenuHit) -> BattleNav {
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
                self.pause_layer = BattlePauseLayer::AbortConfirm;
                self.leave_armed = true;
                self.pause_hover = None;
                self.pause_pressed = None;
                tracing::info!("放弃任务 · 确认");
                BattleNav::None
            }
            BattlePauseMenuHit::Options => {
                self.open_in_game_options_layer();
                tracing::info!("暂停菜单 · 局内选项");
                BattleNav::None
            }
            BattlePauseMenuHit::Fullscreen => {
                tracing::info!("暂停菜单 · 切换全屏");
                BattleNav::ToggleFullscreen
            }
        }
    }

    fn on_abort_confirm_hit(&mut self, hit: BattleAbortConfirmHit) -> BattleNav {
        match hit {
            BattleAbortConfirmHit::Cancel => {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                self.clear_pause_menu_input();
                tracing::info!("放弃确认 · 取消，恢复对局");
                BattleNav::None
            }
            BattleAbortConfirmHit::Leave => {
                self.clear_pause_menu_input();
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                tracing::info!("放弃确认 · 离开对局");
                BattleNav::ToMainMenu
            }
        }
    }

    fn on_in_game_options_hit(&mut self, hit: BattleInGameOptionsHit) -> BattleNav {
        match hit {
            BattleInGameOptionsHit::Back => {
                self.pause_layer = BattlePauseLayer::Menu;
                self.pause_hover = None;
                self.pause_pressed = None;
                self.pause_stub_notice = None;
                tracing::info!("局内选项 · 返回暂停菜单");
                BattleNav::None
            }
            BattleInGameOptionsHit::Sound | BattleInGameOptionsHit::Keyboard => {
                self.pause_stub_notice = Some("暂未实现");
                tracing::info!(entry = hit.entry_id(), "局内选项 · 暂未实现");
                BattleNav::None
            }
            // 勾选 / 滑条在 press 阶段已处理。
            _ => BattleNav::None,
        }
    }

    /// Esc / Options 热键在暂停层上的路由。
    pub(super) fn handle_pause_layer_escape(&mut self) -> BattleNav {
        match self.pause_layer.on_escape() {
            EscapeRoute::ResumeMission => {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                self.clear_pause_menu_input();
                tracing::info!("暂停层 · Esc 恢复对局");
                BattleNav::None
            }
            EscapeRoute::ToLayer(layer) => {
                self.pause_layer = layer;
                self.pause_hover = None;
                self.pause_pressed = None;
                self.in_game_options.drag_track = None;
                self.pause_stub_notice = None;
                tracing::info!(?layer, "暂停层 · Esc 回上层");
                BattleNav::None
            }
        }
    }

    #[allow(dead_code)]
    pub(super) fn in_game_options_state(&self) -> BattleInGameOptionsState {
        self.in_game_options
    }
}
