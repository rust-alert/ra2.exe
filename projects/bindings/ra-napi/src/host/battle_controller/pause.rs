//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use ra_engine::BattleOutcome;
use ra_layout::BattleHudChromeMetrics;
use ra_widgets::battle_pause_menu::{self, BattlePauseMenuHit};
use winit::{event::ElementState, window::Window};

use super::super::battle_input::LeftGesture;

use super::{BattleController, BattleNav};

impl BattleController {
    pub(super) fn clear_pause_menu_input(&mut self) {
        self.pause_hover = None;
        self.pause_pressed = None;
        self.leave_armed = false;
        self.command_hover = None;
        self.command_pressed = None;
        self.left_gesture = LeftGesture::Idle;
    }

    pub(super) fn pause_hud_metrics(&self) -> BattleHudChromeMetrics {
        self.hud_chrome
            .as_ref()
            .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            .or_else(|| self.pause_menu_chrome.as_ref().map(|c| BattleHudChromeMetrics::for_mix(&c.mix)))
            .unwrap_or_else(BattleHudChromeMetrics::sidec01)
    }

    pub(super) fn refresh_pause_hover(&mut self, window: &Window) {
        let size = window.inner_size();
        let metrics = self.pause_hud_metrics();
        self.pause_hover =
            battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, self.cursor.0 as i32, self.cursor.1 as i32)
                .map(|h| h.entry_id());
    }

    pub(super) fn handle_pause_menu_mouse(&mut self, state: ElementState, window: &Window) -> BattleNav {
        let size = window.inner_size();
        let metrics = self.pause_hud_metrics();
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        match state {
            ElementState::Pressed => {
                self.pause_pressed = battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, x, y).map(|h| h.entry_id());
                BattleNav::None
            }
            ElementState::Released => {
                let pressed = self.pause_pressed.take();
                let hit = battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, x, y);
                if pressed.is_some_and(|id| hit.is_some_and(|h| h.entry_id() == id)) {
                    if let Some(hit) = hit {
                        return self.on_pause_menu_hit(hit);
                    }
                }
                BattleNav::None
            }
        }
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
                self.clear_pause_menu_input();
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    game.apply_scripted_outcome(BattleOutcome::Defeat { reason: "放弃任务".into() });
                    // 关掉暂停菜单输入路径；仿真仍因 `outcome` 停住。
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                // 留在对局页播 EVA，由 `pump` → `poll_outcome_nav` 延后进结算。
                self.begin_outcome_hold();
                tracing::info!("放弃任务 · 先播报再结算");
                BattleNav::None
            }
            BattlePauseMenuHit::Options => {
                self.clear_pause_menu_input();
                tracing::info!("暂停菜单 · 打开选项");
                BattleNav::OpenOptions
            }
            BattlePauseMenuHit::Fullscreen => {
                tracing::info!("暂停菜单 · 切换全屏");
                BattleNav::ToggleFullscreen
            }
        }
    }
}
