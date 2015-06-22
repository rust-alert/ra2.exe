//! 壳层导航、切页波浪与菜单动作提交。

use std::time::{Duration, Instant};

use ra_layout::ui_layout;
use ra_widgets::menu_action::MenuAction;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::shell_slide::{
    CAMPAIGN_SLIDE, CHOOSE_MAP_SLIDE, MAIN_MENU_SLIDE, SINGLE_PLAYER_SLIDE, SKIRMISH_SLIDE,
    ShellFrameWave, ShellSlideSpec, WAVE_STOWED_FRAME, WaveDirection,
};
use ra_widgets::ui_compose::ShellWaveFrames;
use winit::event_loop::ActiveEventLoop;

use crate::host::battle_controller::BattleNav;

use super::Shell;

impl Shell {
    pub(super) fn set_screen(&mut self, next: OriginalScreen) {
        if self.screen != next {
            if self.screen == OriginalScreen::SkirmishLobby {
                self.skirmish.end_name_edit();
                self.skirmish.close_combo();
            }
            tracing::info!("页面 {} → {}", self.screen.as_str(), next.as_str());
            self.screen = next;
            self.menu_pressed_entry = None;
            self.menu_pending_commit = None;
            self.menu_hovered_entry = None;
            self.status_line.clear();
            // `menu_frame_wave` / `menu_slide_gap_until` 由切页状态机显式启停，不在此清空。
            if !matches!(
                next,
                OriginalScreen::MainMenu | OriginalScreen::SinglePlayerMenu | OriginalScreen::Options | OriginalScreen::ExitConfirm
            ) {
                self.menu_movie = None;
                self.menu_movie_clock = None;
            }
            self.sync_shell_audio();
            self.refresh_ui_resolve_note();
            self.refresh_menu_backdrop();
            self.refresh_shell_title();
            #[cfg(feature = "test-harness")]
            if self.auto_screenshots.should_capture(next) {
                self.queue_screenshot(next.as_str());
            }
        }
    }

    pub(super) fn apply_menu_action(&mut self, event_loop: &ActiveEventLoop, action: MenuAction) {
        self.request_menu_action(event_loop, action);
    }

    /// 是否为会换壳层页的导航动作（才走出去 / 进来波浪）。
    pub(super) fn action_uses_shell_slide(action: MenuAction) -> bool {
        matches!(
            action,
            MenuAction::OpenSinglePlayer
                | MenuAction::OpenNetwork
                | MenuAction::OpenOptions
                | MenuAction::Exit
                | MenuAction::OpenSkirmish
                | MenuAction::OpenCampaign
                | MenuAction::Back
                | MenuAction::StartSkirmish
                | MenuAction::OptionsAccept
                | MenuAction::OptionsCancel
                | MenuAction::ChooseMap
                | MenuAction::UseMap
        )
    }

    /// 当前页若在允许列表内，返回波浪规格。
    pub(super) fn slide_spec_for(screen: OriginalScreen) -> Option<ShellSlideSpec> {
        match screen {
            OriginalScreen::MainMenu => Some(MAIN_MENU_SLIDE),
            OriginalScreen::SinglePlayerMenu => Some(SINGLE_PLAYER_SLIDE),
            OriginalScreen::SkirmishLobby => Some(SKIRMISH_SLIDE),
            OriginalScreen::Campaign => Some(CAMPAIGN_SLIDE),
            OriginalScreen::ChooseMap => Some(CHOOSE_MAP_SLIDE),
            _ => None,
        }
    }

    /// 当前页参与波浪的按钮 id 表。
    pub(super) fn wave_button_ids(screen: OriginalScreen) -> Option<&'static [&'static str]> {
        match screen {
            OriginalScreen::MainMenu => Some(&ui_layout::MAIN_MENU_BUTTON_IDS),
            OriginalScreen::SinglePlayerMenu => Some(&ui_layout::SINGLE_PLAYER_BUTTON_IDS),
            OriginalScreen::SkirmishLobby => Some(&ui_layout::SKIRMISH_LOBBY_BUTTON_IDS),
            OriginalScreen::Campaign => Some(&ui_layout::CAMPAIGN_BUTTON_IDS),
            OriginalScreen::ChooseMap => Some(&ui_layout::CHOOSE_MAP_BUTTON_IDS),
            _ => None,
        }
    }

    /// 当前页壳层 chrome 布局（波浪按物理平铺格取帧）。
    pub(super) fn wave_shell_layout(screen: OriginalScreen) -> Option<ui_layout::MainMenuLayout> {
        Some(match screen {
            OriginalScreen::MainMenu => ui_layout::main_menu_layout(0, 0),
            OriginalScreen::SinglePlayerMenu => ui_layout::single_player_layout(0, 0),
            OriginalScreen::SkirmishLobby => ui_layout::skirmish_lobby_layout(0, 0).shell,
            OriginalScreen::Campaign => ui_layout::campaign_layout(0, 0).shell,
            OriginalScreen::ChooseMap => ui_layout::choose_map_layout(0, 0).shell,
            _ => return None,
        })
    }

    /// 按钮格相对 `panel_tile` 的平铺下标（贴底 Exit/返回落在末格）。
    pub(super) fn panel_tile_index(layout: &ui_layout::MainMenuLayout, cell: ui_layout::RectPx) -> u32 {
        let tile_h = layout.panel_tile.h.max(1);
        ((cell.y - layout.panel_tile.y) / tile_h).max(0) as u32
    }

    /// 切页波浪或出去→进来卡顿进行中。
    pub(super) fn shell_slide_busy(&self) -> bool {
        self.menu_frame_wave.is_some() || self.menu_slide_gap_until.is_some()
    }

    /// 合成用收起帧（卡顿间隙：钮面停在 `WAVE_STOWED_FRAME`，不叠字）。
    pub(super) fn stowed_wave_frames(&self) -> Option<(Vec<u16>, Vec<u16>)> {
        let ids = Self::wave_button_ids(self.screen)?;
        let layout = Self::wave_shell_layout(self.screen)?;
        let buttons = vec![WAVE_STOWED_FRAME; ids.len()];
        let tiles = vec![WAVE_STOWED_FRAME; layout.panel_tile_count.max(0) as usize];
        Some((buttons, tiles))
    }

    /// 合成用：按钮帧 + 空格平铺帧；无波浪且无卡顿时为 `None`。
    /// 帧序按物理格自上而下统一交错，末钮与中间无字格同一波浪。
    pub(super) fn current_wave_frames(&self) -> Option<(Vec<u16>, Vec<u16>)> {
        if self.menu_slide_gap_until.is_some() {
            return self.stowed_wave_frames();
        }
        let wave = self.menu_frame_wave.as_ref()?;
        let ids = Self::wave_button_ids(self.screen)?;
        let layout = Self::wave_shell_layout(self.screen)?;
        let buttons = ids
            .iter()
            .enumerate()
            .map(|(i, _id)| {
                let cell = layout.buttons.get(i).copied().unwrap_or(ui_layout::RectPx::new(0, 0, 0, 0));
                let ti = if cell.w > 0 && cell.h > 0 {
                    Self::panel_tile_index(&layout, cell)
                } else {
                    i as u32
                };
                wave.frame_for_slot(ti)
            })
            .collect::<Vec<_>>();
        let tile_count = layout.panel_tile_count.max(0) as u32;
        let tiles = (0..tile_count).map(|ti| wave.frame_for_slot(ti)).collect::<Vec<_>>();
        Some((buttons, tiles))
    }

    /// 新页进场波浪（仅当目标页有规格且当前无波浪 / 卡顿）。
    pub(super) fn maybe_start_slide_in(&mut self) {
        if self.shell_slide_busy() {
            return;
        }
        let Some(spec) = Self::slide_spec_for(self.screen)
        else {
            return;
        };
        self.menu_frame_wave = Some(ShellFrameWave::new(spec, WaveDirection::SlideIn, Instant::now()));
        self.play_menu_move_in();
        self.refresh_menu_backdrop();
    }

    /// `SlideOut` 完成后：可选卡顿，再 `SlideIn`（`shell_slide_gap_secs=0` 则立刻进）。
    pub(super) fn begin_slide_gap_or_in(&mut self) {
        let gap = self.shell_slide_gap_secs.max(0.0);
        if gap > 0.0 {
            self.menu_slide_gap_until = Some(Instant::now() + Duration::from_secs_f64(gap));
            self.refresh_menu_backdrop();
        }
        else {
            self.maybe_start_slide_in();
        }
    }

    /// 菜单导航入口：可切页动作先 SlideOut，完成后再提交，再对目标页 SlideIn。
    pub(super) fn request_menu_action(&mut self, event_loop: &ActiveEventLoop, action: MenuAction) {
        if self.shell_slide_busy() {
            return;
        }
        if Self::action_uses_shell_slide(action) {
            if let Some(spec) = Self::slide_spec_for(self.screen) {
                self.menu_pressed_entry = None;
                self.menu_pending_commit = Some(action);
                self.menu_frame_wave = Some(ShellFrameWave::new(spec, WaveDirection::SlideOut, Instant::now()));
                self.play_menu_move_out();
                self.refresh_menu_backdrop();
                return;
            }
            let before = self.screen;
            self.commit_menu_action(event_loop, action);
            if self.screen != before {
                self.maybe_start_slide_in();
            }
            return;
        }
        self.commit_menu_action(event_loop, action);
    }

    /// 推进切页波浪 / 卡顿。`SlideOut` 结束后提交排队动作，经间隔再 `SlideIn`。
    pub(super) fn tick_menu_frame_wave(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.menu_slide_gap_until {
            if Instant::now() >= deadline {
                self.menu_slide_gap_until = None;
                self.maybe_start_slide_in();
            }
            return;
        }
        let Some(wave) = self.menu_frame_wave.as_mut()
        else {
            return;
        };
        let now = Instant::now();
        if !wave.advance(now) {
            return;
        }
        if !wave.is_complete() {
            self.refresh_menu_backdrop();
            return;
        }
        let direction = wave.direction();
        self.menu_frame_wave = None;
        if direction == WaveDirection::SlideOut {
            if let Some(action) = self.menu_pending_commit.take() {
                self.commit_menu_action(event_loop, action);
                self.begin_slide_gap_or_in();
            }
            else {
                self.refresh_menu_backdrop();
            }
        }
        else {
            self.refresh_menu_backdrop();
        }
    }

    pub(super) fn commit_menu_action(&mut self, event_loop: &ActiveEventLoop, action: MenuAction) {
        // 直接提交路径：取消尚未完成的出去波浪排队（卡顿由 `begin_slide_gap_or_in` 另管）。
        self.menu_pending_commit = None;
        let _ = self.menu_pressed_entry.take();
        match action {
            MenuAction::OpenSinglePlayer => self.set_screen(OriginalScreen::SinglePlayerMenu),
            MenuAction::OpenNetwork => {
                tracing::info!("网络入口未开放（Beta）");
                self.set_screen(OriginalScreen::Network);
            }
            MenuAction::OpenOptions => self.open_options_page(),
            MenuAction::Exit => {
                self.banner = "确认退出？".into();
                self.set_screen(OriginalScreen::ExitConfirm);
            }
            MenuAction::ConfirmExit => {
                tracing::info!("用户确认退出");
                event_loop.exit();
            }
            MenuAction::OpenSkirmish => {
                self.ensure_lobby_maps();
                self.set_screen(OriginalScreen::SkirmishLobby);
            }
            MenuAction::OpenCampaign => {
                self.campaign_side = None;
                self.campaign_difficulty = 1;
                self.campaign_dragging = false;
                self.campaign_pointer_consumed = false;
                self.set_screen(OriginalScreen::Campaign);
                self.banner = "战役选边".into();
                self.refresh_shell_title();
            }
            MenuAction::SelectCampaignAllied => {
                self.begin_campaign_load("allied");
            }
            MenuAction::SelectCampaignTutorial => {
                self.begin_campaign_load("tutorial");
            }
            MenuAction::SelectCampaignSoviet => {
                self.begin_campaign_load("soviet");
            }
            MenuAction::CycleCampaignDifficulty => {
                self.campaign_difficulty = (self.campaign_difficulty + 1) % 3;
                let label = match self.campaign_difficulty {
                    0 => "易",
                    2 => "难",
                    _ => "中",
                };
                self.banner = format!("战役难度 · {label}");
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::Back => match self.screen {
                OriginalScreen::SinglePlayerMenu | OriginalScreen::Network | OriginalScreen::Options | OriginalScreen::ExitConfirm => {
                    if self.screen == OriginalScreen::Options {
                        self.discard_options_draft();
                    }
                    self.set_screen(OriginalScreen::MainMenu);
                }
                OriginalScreen::SkirmishLobby | OriginalScreen::Campaign => {
                    self.set_screen(OriginalScreen::SinglePlayerMenu);
                }
                OriginalScreen::ChooseMap => self.cancel_choose_map(),
                _ => self.set_screen(OriginalScreen::MainMenu),
            },
            MenuAction::StartSkirmish => self.begin_skirmish_load(),
            MenuAction::CancelLoad => self.cancel_load(),
            MenuAction::RetryLoad => self.retry_load(),
            MenuAction::Noop => {}
            MenuAction::CycleSide => {
                self.skirmish.cycle_side();
                self.banner = format!("阵营 · {}", self.skirmish.side);
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::CycleDifficulty => {
                self.skirmish.cycle_difficulty();
                self.banner = format!("难度 · {}", self.skirmish.difficulty);
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
            MenuAction::OptionsAccept => self.apply_options_accept(),
            MenuAction::OptionsCancel => {
                self.discard_options_draft();
                self.set_screen(OriginalScreen::MainMenu);
                self.banner = "选项已取消".into();
                self.refresh_shell_title();
            }
            MenuAction::SelectMode(i) => self.select_lobby_mode_index(i),
            MenuAction::SelectMap(i) => {
                let maps = self.maps_for_menu_hit();
                if let Some(map) = maps.get(i) {
                    self.selected_map = Some(map.file_name.clone());
                    self.skirmish.preferred_map = Some(map.file_name.clone());
                    self.ensure_lobby_preview();
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
            }
            MenuAction::ChooseMap => self.open_choose_map_page(),
            MenuAction::UseMap => self.confirm_choose_map(),
        }
    }

    pub(super) fn apply_nav(&mut self, nav: BattleNav) {
        match nav {
            BattleNav::None => {}
            BattleNav::Rematch => {
                self.banner = "重开…".into();
                self.begin_skirmish_load();
            }
            BattleNav::ToResults => self.set_screen(OriginalScreen::Results),
            BattleNav::ToMainMenu => {
                // Pre-Alpha：从对局/结算回到遭遇战大厅，保留已选地图。
                self.ensure_lobby_maps();
                self.banner = format!("已返回大厅 · 地图 {}", self.selected_map.as_deref().unwrap_or("（未选）"));
                self.set_screen(OriginalScreen::SkirmishLobby);
            }
        }
    }
}
