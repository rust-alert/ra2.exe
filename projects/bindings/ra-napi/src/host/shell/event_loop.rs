//! winit ApplicationHandler。

use std::sync::Arc;

use ra_widgets::menu_action::MenuAction;
use ra_widgets::options_dialog::OptionsHit;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::input::hit;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use super::Shell;

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("ra2")
                        .with_inner_size(winit::dpi::LogicalSize::new(self.window_width, self.window_height))
                        // 客户区尺寸由 `DisplayMode` 离散档决定，禁止自由拉伸窗口。
                        .with_resizable(false)
                        // GPU / 闪屏首帧完成前保持隐藏，避免 Windows 默认纯白客户区闪一下。
                        .with_visible(false),
                )
                .expect("创建窗口失败"),
        );
        if let Err(e) = self.renderer.attach_window(window.clone()) {
            tracing::error!("wgpu 附着失败: {e}");
            self.banner = format!("GPU 附着失败 · {e}");
            // 无 GPU 表面则禁止空转主循环；窗口也不再保留。
            event_loop.exit();
            return;
        }
        tracing::info!(
            "gpu={} preview={} zoom={:.2} screen={}",
            self.renderer.backend_name(),
            if self.renderer.has_preview() { "yes" } else { "no" },
            self.renderer.camera().zoom,
            self.screen.as_str()
        );
        self.window = Some(window.clone());
        if self.screen == OriginalScreen::Splash {
            self.ensure_startup_splash_presented();
        }
        else {
            self.refresh_menu_backdrop();
        }
        self.refresh_shell_title();
        // 先提交一帧（闪屏或菜单），再显示窗口，消除启动纯白闪屏。
        self.renderer.draw_frame(None);
        window.set_visible(true);
        window.request_redraw();
        #[cfg(feature = "test-harness")]
        if self.auto_screenshots.should_capture(self.screen) {
            self.queue_screenshot(self.screen.as_str());
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match &event {
            WindowEvent::CloseRequested => {
                // 前置壳层页先进入退出确认；已在确认页或对局中则直接退出。
                if self.screen == OriginalScreen::ExitConfirm
                    || matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results | OriginalScreen::Splash)
                {
                    event_loop.exit();
                }
                else if matches!(
                    self.screen,
                    OriginalScreen::MainMenu
                        | OriginalScreen::SinglePlayerMenu
                        | OriginalScreen::Options
                        | OriginalScreen::SkirmishLobby
                        | OriginalScreen::Network
                        | OriginalScreen::LoadScreen
                ) {
                    if self.screen == OriginalScreen::Options {
                        self.discard_options_draft();
                    }
                    self.banner = "确认退出？".into();
                    self.set_screen(OriginalScreen::ExitConfirm);
                }
                else {
                    event_loop.exit();
                }
                return;
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
                // GPU 表面跟物理缓冲；命中/布局锁定在选定 `DisplayMode` 客户区。
                let (w, h) = self.display_mode.size();
                self.window_width = w as f64;
                self.window_height = h as f64;
                if !matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
                    self.refresh_menu_backdrop();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let (w, h) = self.display_mode.size();
                self.window_width = w as f64;
                self.window_height = h as f64;
                if !matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
                    self.refresh_menu_backdrop();
                }
            }
            WindowEvent::RedrawRequested => {
                self.tick_menu_frame_wave(event_loop);
                self.ensure_battle_mouse_cursors(event_loop);
                self.redraw();
                return;
            }
            WindowEvent::KeyboardInput { event: key_ev, .. }
                if key_ev.state == ElementState::Pressed && matches!(key_ev.physical_key, PhysicalKey::Code(KeyCode::F12)) =>
            {
                // 对局页也走同一截图路径（不交给 BattleController）。
                self.queue_screenshot(self.screen.as_str());
                return;
            }
            _ => {}
        }

        match self.screen {
            OriginalScreen::Battle | OriginalScreen::Results => {
                let accept = self.screen.accepts_battle_commands();
                let Some(window) = self.window.clone()
                else {
                    return;
                };
                if let Some(ctrl) = self.battle_controller.as_mut() {
                    let nav = ctrl.handle_event(&event, &mut self.renderer, &window, accept);
                    self.apply_nav(nav);
                }
            }
            OriginalScreen::Splash
            | OriginalScreen::MainMenu
            | OriginalScreen::SinglePlayerMenu
            | OriginalScreen::Campaign
            | OriginalScreen::SkirmishLobby
            | OriginalScreen::ChooseMap
            | OriginalScreen::Network
            | OriginalScreen::Options
            | OriginalScreen::ExitConfirm
            | OriginalScreen::LoadScreen => match &event {
                WindowEvent::CursorMoved { position, .. } => {
                    // 与 window_width/height 同用逻辑像素，避免 HiDPI 下物理光标打偏命中框。
                    let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                    let logical = position.to_logical::<f64>(scale);
                    self.cursor = (logical.x, logical.y);
                    if self.screen == OriginalScreen::Options && self.handle_options_drag() {
                        // 拖动滑条已刷新。
                    }
                    else if self.screen == OriginalScreen::SkirmishLobby && self.handle_skirmish_drag() {
                        // 遭遇战滑条拖动已刷新。
                    }
                    else if self.screen == OriginalScreen::Campaign && self.handle_campaign_drag() {
                        // 战役难度滑条拖动已刷新。
                    }
                    else if matches!(
                        self.screen,
                        OriginalScreen::MainMenu
                            | OriginalScreen::SinglePlayerMenu
                            | OriginalScreen::Campaign
                            | OriginalScreen::Options
                            | OriginalScreen::ExitConfirm
                            | OriginalScreen::SkirmishLobby
                            | OriginalScreen::ChooseMap
                    ) {
                        let next = self.menu_entry_under_cursor();
                        if next != self.menu_hovered_entry {
                            if self.screen == OriginalScreen::Campaign {
                                if matches!(next, Some("allied" | "tutorial" | "soviet")) {
                                    if let Some(side) = next {
                                        self.play_campaign_side_hover(side);
                                    }
                                    self.campaign_side_anim_frame = 1;
                                    self.campaign_side_anim_accum = 0.0;
                                }
                            }
                            self.menu_hovered_entry = next;
                            self.sync_status_line_from_hover();
                            self.refresh_menu_backdrop();
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } if self.screen == OriginalScreen::ChooseMap => {
                    let rows = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => {
                            if *y > 0.0 {
                                -1
                            } else if *y < 0.0 {
                                1
                            } else {
                                0
                            }
                        }
                        winit::event::MouseScrollDelta::PixelDelta(p) => {
                            if p.y > 0.0 {
                                -1
                            } else if p.y < 0.0 {
                                1
                            } else {
                                0
                            }
                        }
                    };
                    if rows != 0 {
                        self.nudge_map_list_scroll(rows);
                    }
                }
                WindowEvent::MouseInput { state, button: winit::event::MouseButton::Left, .. } => match state {
                    ElementState::Pressed => {
                        if self.shell_slide_busy() {
                            // 切页波浪 / 卡顿进行中忽略新按下。
                        }
                        else if self.screen == OriginalScreen::Splash {
                            self.request_splash_skip();
                        }
                        else if self.screen == OriginalScreen::Options && self.handle_options_press() {
                            // 选项左/右栏已处理。
                        }
                        else if self.screen == OriginalScreen::SkirmishLobby && self.handle_skirmish_press() {
                            // 遭遇战左栏勾选/滑条已处理。
                        }
                        else if self.screen == OriginalScreen::Campaign && self.handle_campaign_press() {
                            // 战役难度滑条已处理。
                        }
                        else if matches!(
                            self.screen,
                            OriginalScreen::MainMenu
                                | OriginalScreen::SinglePlayerMenu
                                | OriginalScreen::Campaign
                                | OriginalScreen::Options
                                | OriginalScreen::ExitConfirm
                                | OriginalScreen::SkirmishLobby
                                | OriginalScreen::ChooseMap
                        ) {
                            let next = self.menu_entry_under_cursor();
                            if self.screen == OriginalScreen::SkirmishLobby && next.is_some() {
                                self.skirmish.end_name_edit();
                                self.skirmish.close_combo();
                            }
                            if next != self.menu_pressed_entry {
                                self.menu_pressed_entry = next;
                                // 选图列表行在 `SelectMap` / `SelectMode` 提交时再播，避免连点同 entry 无声或按下+松开双响。
                                if next.is_some_and(|id| !matches!(id, "mode_row" | "map_row")) {
                                    self.play_menu_click();
                                }
                                self.refresh_menu_backdrop();
                            }
                        }
                    }
                    ElementState::Released => {
                        if self.shell_slide_busy() {
                            // 切页波浪 / 卡顿进行中忽略释放提交。
                        }
                        else if self.screen == OriginalScreen::Splash {
                            // 闪屏仅接受按下跳过请求；释放不走菜单命中。
                        }
                        else if self.screen == OriginalScreen::Options {
                            let consumed = self.options_pointer_consumed;
                            self.options_pointer_consumed = false;
                            if let Some(state) = self.options_state.as_mut() {
                                state.on_release();
                            }
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            if consumed {
                                self.refresh_menu_backdrop();
                            }
                            else if let Some(action) = hit::hit_action(
                                self.screen,
                                &self.maps_for_menu_hit(),
                                self.lobby_modes.len(),
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.map_list_scroll,
                                self.load_allow_retry(),
                            ) {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                        else if self.screen == OriginalScreen::SkirmishLobby {
                            let consumed = self.skirmish_pointer_consumed;
                            self.skirmish_pointer_consumed = false;
                            self.skirmish.on_release();
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            if consumed {
                                self.refresh_menu_backdrop();
                            }
                            else if let Some(action) = hit::hit_action(
                                self.screen,
                                &self.maps_for_menu_hit(),
                                self.lobby_modes.len(),
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.map_list_scroll,
                                self.load_allow_retry(),
                            ) {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                        else if self.screen == OriginalScreen::Campaign {
                            let consumed = self.campaign_pointer_consumed;
                            self.campaign_pointer_consumed = false;
                            self.campaign_dragging = false;
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            if consumed {
                                // 难度已在按下/拖动时落档，勿再 CycleCampaignDifficulty。
                            }
                            else if let Some(action) = hit::hit_action(
                                self.screen,
                                &self.maps_for_menu_hit(),
                                self.lobby_modes.len(),
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.map_list_scroll,
                                self.load_allow_retry(),
                            ) {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                        else {
                            if self.menu_pressed_entry.take().is_some() {
                                self.refresh_menu_backdrop();
                            }
                            let action = hit::hit_action(
                                self.screen,
                                &self.maps_for_menu_hit(),
                                self.lobby_modes.len(),
                                self.selected_map.as_deref(),
                                self.cursor,
                                self.window_width,
                                self.window_height,
                                self.map_list_scroll,
                                self.load_allow_retry(),
                            );
                            if let Some(action) = action {
                                tracing::debug!(?action, "菜单逻辑命中");
                                self.apply_menu_action(event_loop, action);
                            }
                        }
                    }
                },
                WindowEvent::KeyboardInput { event: key_ev, .. } => {
                    if key_ev.state == ElementState::Pressed {
                        if self.handle_skirmish_name_key(key_ev) {
                            // 编辑玩家名时吞掉大厅快捷键。
                        }
                        else {
                            self.handle_pre_game_key(event_loop, key_ev.physical_key);
                        }
                    }
                }
                _ => {}
            },
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
