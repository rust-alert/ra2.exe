//! 每帧合成、上传与标题刷新。

use std::time::Instant;

use ra_layout::ui_layout;
use ra_renderer::RgbaImage;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::shell_slide::WaveDirection;
use ra_widgets::ui_compose::{self, ShellWaveFrames};
use ra_widgets::ui_decode;
use ra_widgets::ui_present;
use ra_widgets::ui_text::resolve_caption;

use super::Shell;

impl Shell {
    /// 上传 UI 页：先按 `[present]` 做质感变换再进 GPU。
    pub(super) fn upload_ui_page(&mut self, page: RgbaImage) {
        let page = ui_present::present_ui_page(page, self.present);
        self.renderer.set_ui_page(page);
    }

    /// 前置页：主菜单 / 单人 / 选项 / 遭遇战大厅 / 装载页上传合成 chrome；启动闪屏由独立 owner 保持。
    pub(super) fn refresh_menu_backdrop(&mut self) {
        if matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
            // 对局 HUD 由 `BattleController::draw_frame` 维护，勿在此清空；仍预热字体。
            self.ensure_menu_assets();
            self.ensure_menu_text_assets();
            return;
        }
        // 启动闪屏禁止走菜单合成路径，更不能 clear 掉已上传的 GLSS/GLSL 画面。
        if self.screen == OriginalScreen::Splash {
            self.ensure_startup_splash_presented();
            return;
        }
        self.ensure_menu_assets();
        self.ensure_menu_text_assets();
        if matches!(
            self.screen,
            OriginalScreen::MainMenu
                | OriginalScreen::SinglePlayerMenu
                | OriginalScreen::Campaign
                | OriginalScreen::Options
                | OriginalScreen::ExitConfirm
                | OriginalScreen::SkirmishLobby
                | OriginalScreen::ChooseMap
                | OriginalScreen::LoadScreen
        ) {
            if matches!(self.screen, OriginalScreen::SkirmishLobby | OriginalScreen::ChooseMap) {
                self.ensure_lobby_maps();
                self.ensure_lobby_modes();
                self.ensure_lobby_preview();
            }
            if matches!(self.screen, OriginalScreen::Campaign | OriginalScreen::SkirmishLobby) {
                self.ensure_skirmish_chrome();
            }
            // 大厅预览并入 UI 页合成，避免与 `set_map_preview` 双通道抢相机。
            self.renderer.clear_preview();
            let load_allow_retry = self.load_allow_retry();
            let load_status = if self.screen == OriginalScreen::LoadScreen { Some(self.banner.clone()) } else { None };
            let load_progress = self.load_screen_progress();
            let wave_owned = self.current_wave_frames();
            let wave = wave_owned.as_ref().map(|(buttons, tiles)| {
                let animate_empty_tiles =
                    self.menu_frame_wave.as_ref().is_some_and(|w| w.direction() == WaveDirection::SlideOut);
                ui_compose::ShellWaveFrames {
                    buttons: buttons.as_slice(),
                    tiles: tiles.as_slice(),
                    animate_empty_tiles,
                }
            });
            if let Some(decoded) = self.ui_decode_cache.as_ref() {
                let movie = self.menu_movie.as_ref().and_then(|m| m.frame());
                let page = match self.screen {
                    OriginalScreen::MainMenu => ui_compose::compose_main_menu_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.status_line_visible(),
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        movie,
                        wave,
                        self.menu_panel_anim_frame,
                    ),
                    OriginalScreen::SinglePlayerMenu => ui_compose::compose_single_player_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.status_line_visible(),
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        movie,
                        wave,
                        self.menu_panel_anim_frame,
                    ),
                    OriginalScreen::Campaign => {
                        let track_thumb = self.skirmish_chrome.as_ref().and_then(|c| c.track_thumb.as_ref());
                        ui_compose::compose_campaign_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.status_line_visible(),
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            ui_compose::CampaignPaint {
                                selected_side: self.campaign_side,
                                difficulty: self.campaign_difficulty,
                                track_thumb,
                                side_anim_frame: self.campaign_side_anim_frame.max(1),
                            },
                            wave,
                            self.menu_panel_anim_frame,
                        )
                    }
                    OriginalScreen::Options => self.options_state.as_ref().and_then(|state| {
                        ui_compose::compose_options_page(
                            decoded,
                            state,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            movie,
                            self.menu_panel_anim_frame,
                        )
                    }),
                    OriginalScreen::ExitConfirm => ui_compose::compose_exit_confirm_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        movie,
                        self.menu_panel_anim_frame,
                    ),
                    OriginalScreen::SkirmishLobby => {
                        let selected = self
                            .selected_map
                            .as_ref()
                            .and_then(|name| self.lobby_maps.iter().find(|m| &m.file_name == name))
                            .or_else(|| self.lobby_maps.first());
                        let map_file = selected.map(|m| m.file_name.as_str()).unwrap_or("");
                        let map_csf = selected.map(|m| m.name_csf.as_str());
                        let map_name = resolve_caption(self.menu_csf.as_ref(), map_file, map_csf);
                        let country = self.skirmish.side.clone();
                        let ai_csf = ra_widgets::skirmish_setup::SkirmishBootRequest::ai_difficulty_csf_key(&self.skirmish.difficulty);
                        let ai_name = self
                            .menu_csf
                            .as_ref()
                            .and_then(|c| c.get(ai_csf).map(|s| s.to_string()))
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| self.skirmish.difficulty.clone());
                        let ai_rows = self.lobby_ai_rows();
                        let mode_csf = self
                            .selected_mode_id
                            .and_then(|id| self.lobby_modes.iter().find(|m| m.id == id))
                            .map(|m| m.name_csf.as_str())
                            .unwrap_or("GUI:Battle");
                        let game_type_name = resolve_caption(self.menu_csf.as_ref(), mode_csf, Some(mode_csf));
                        let paint = ui_compose::SkirmishLobbyPaint {
                            map_name: map_name.as_str(),
                            game_type_name: game_type_name.as_str(),
                            player_name: self.skirmish.player_name.as_str(),
                            country_name: country.as_str(),
                            color_rgb: self.skirmish.color_rgb(),
                            ai_name: ai_name.as_str(),
                            ai_country: self.skirmish.row_side(1),
                            ai_difficulty: self.skirmish.difficulty.as_str(),
                            ai_rows,
                            short_game: self.skirmish.short_game,
                            mcv_repacks: self.skirmish.mcv_repacks,
                            crates: self.skirmish.crates,
                            superweapons: self.skirmish.superweapons,
                            build_off_ally: self.skirmish.build_off_ally,
                            game_speed: self.skirmish.game_speed,
                            credits: self.skirmish.credits,
                            unit_count: self.skirmish.unit_count,
                            player_name_editing: self.skirmish.player_name_editing,
                            country_combo_open: self.skirmish.open_combo == Some(ra_widgets::skirmish_setup::SkirmishComboKind::Country),
                            color_combo_open: self.skirmish.open_combo == Some(ra_widgets::skirmish_setup::SkirmishComboKind::Color),
                            ai_combo_open: self.skirmish.open_combo == Some(ra_widgets::skirmish_setup::SkirmishComboKind::Ai),
                            combo_row: self.skirmish.combo_row,
                            row_side_indices: self.skirmish.row_sides,
                            row_color_indices: self.skirmish.row_colors,
                            chrome: self.skirmish_chrome.as_ref(),
                        };
                        ui_compose::compose_skirmish_lobby_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.status_line_visible(),
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            self.lobby_preview.as_ref(),
                            &paint,
                            wave,
                            0,
                        )
                    }
                    OriginalScreen::ChooseMap => {
                        let mode_labels: Vec<String> = self
                            .lobby_modes
                            .iter()
                            .map(|m| resolve_caption(self.menu_csf.as_ref(), &m.name_csf, Some(&m.name_csf)))
                            .collect();
                        let mode_names: Vec<&str> = mode_labels.iter().map(|s| s.as_str()).collect();
                        let selected_mode_index = self
                            .selected_mode_id
                            .and_then(|id| self.lobby_modes.iter().position(|m| m.id == id));
                        let visible_maps = self.maps_matching_selected_mode();
                        let map_labels: Vec<String> = visible_maps
                            .iter()
                            .map(|m| resolve_caption(self.menu_csf.as_ref(), &m.file_name, Some(&m.name_csf)))
                            .collect();
                        let map_names: Vec<&str> = map_labels.iter().map(|s| s.as_str()).collect();
                        let selected_map_index =
                            self.selected_map.as_ref().and_then(|sel| visible_maps.iter().position(|m| &m.file_name == sel));
                        ui_compose::compose_choose_map_page(
                            decoded,
                            self.window_width as u32,
                            self.window_height as u32,
                            self.menu_pressed_entry,
                            self.menu_hovered_entry,
                            self.status_line_visible(),
                            self.menu_font.as_ref(),
                            self.menu_csf.as_ref(),
                            self.lobby_preview.as_ref(),
                            &mode_names,
                            selected_mode_index,
                            &map_names,
                            selected_map_index,
                            wave,
                            0,
                        )
                    }
                    OriginalScreen::LoadScreen => ui_compose::compose_load_screen_page(
                        decoded,
                        self.window_width as u32,
                        self.window_height as u32,
                        self.menu_pressed_entry,
                        self.menu_hovered_entry,
                        self.menu_font.as_ref(),
                        self.menu_csf.as_ref(),
                        ui_compose::LoadScreenPaint {
                            side: self.skirmish.side.as_str(),
                            player_name: self.skirmish.player_name.as_str(),
                            side_flag: self.skirmish_chrome.as_ref().and_then(|c| c.row_flags[0].as_ref()),
                            status: load_status.as_deref().unwrap_or(""),
                            allow_retry: load_allow_retry,
                            progress: load_progress,
                            brief_csf_override: self.load_brief_csf.as_deref(),
                        },
                    ),
                    _ => None,
                };
                if let Some(page) = page {
                    tracing::debug!(screen = self.screen.as_str(), w = page.width(), h = page.height(), "壳层 chrome 已合成并上传 UI 页通道");
                    self.upload_ui_page(page);
                    if self.screen != OriginalScreen::LoadScreen && !self.banner.contains("chrome 已上传") {
                        self.banner = format!("{} · chrome 已上传", self.banner);
                        self.refresh_shell_title();
                    }
                    return;
                }
            }
            // 装载页即使 chrome 未解码也要画出可读状态，禁止纯色空窗。
            if self.screen == OriginalScreen::LoadScreen {
                let empty = ui_decode::PageDecodeReport {
                    background: None,
                    panels: Vec::new(),
                    button_normals: Vec::new(),
                    button_hovers: Vec::new(),
                    button_presseds: Vec::new(),
                    sdbtnanm_frames: Vec::new(),
                    errors: Vec::new(),
                };
                if let Some(page) = ui_compose::compose_load_screen_page(
                    &empty,
                    self.window_width as u32,
                    self.window_height as u32,
                    self.menu_pressed_entry,
                    self.menu_hovered_entry,
                    self.menu_font.as_ref(),
                    self.menu_csf.as_ref(),
                    ui_compose::LoadScreenPaint {
                        side: self.skirmish.side.as_str(),
                        player_name: self.skirmish.player_name.as_str(),
                        side_flag: self.skirmish_chrome.as_ref().and_then(|c| c.row_flags[0].as_ref()),
                        status: load_status.as_deref().unwrap_or(self.banner.as_str()),
                        allow_retry: load_allow_retry,
                        progress: load_progress,
                        brief_csf_override: self.load_brief_csf.as_deref(),
                    },
                ) {
                    self.upload_ui_page(page);
                    return;
                }
            }
            self.renderer.clear_ui_page();
            return;
        }

        self.renderer.clear_ui_page();
        self.renderer.clear_preview();
    }

    pub(super) fn refresh_shell_title(&mut self) {
        let Some(window) = &self.window
        else {
            return;
        };
        if matches!(self.screen, OriginalScreen::Battle | OriginalScreen::Results) {
            return;
        }
        let title = match self.screen {
            OriginalScreen::Splash => {
                format!("ra2 · 闪屏 · {} · Esc/Enter/点击跳过（预处理完成后进主菜单）· F12 截图", self.banner)
            }
            OriginalScreen::MainMenu => {
                format!("ra2 · 主菜单 · {} · Enter 单人 · N 网络 · O 选项 · Esc 确认退出 · F12 截图", self.banner)
            }
            OriginalScreen::SinglePlayerMenu => "ra2 · 单人游戏 · Enter/S 遭遇战 · Esc 返回 · F12 截图".into(),
            OriginalScreen::Campaign => {
                format!("ra2 · 战役 · {} · Esc 返回 · F12 截图", self.banner)
            }
            OriginalScreen::SkirmishLobby => {
                let detail = self
                    .selected_map
                    .as_ref()
                    .and_then(|name| self.lobby_maps.iter().find(|m| &m.file_name == name))
                    .map(|m| format!("{} {}x{} {}", m.file_name, m.width, m.height, m.theater.as_str()))
                    .unwrap_or_else(|| "（无可用图）".into());
                format!(
                    "ra2 · 遭遇战大厅 · {detail} · {}/{} · ←/→ 图 · Home/End · Q阵营 E难度 · Enter 开始 · Esc 返回 · F12 截图",
                    self.skirmish.side, self.skirmish.difficulty
                )
            }
            OriginalScreen::ChooseMap => {
                format!("ra2 · 选图 · {} · Esc 回大厅 · F12 截图", self.selected_map.as_deref().unwrap_or("（未选）"))
            }
            OriginalScreen::Network => "ra2 · 网络（占位禁用）· Esc 返回 · F12 截图".into(),
            OriginalScreen::LoadScreen => {
                if self.load_job.is_some() {
                    format!("ra2 · 加载 · {} · Esc/点取消 · F12 截图", self.banner)
                }
                else {
                    format!("ra2 · 加载 · {} · Enter/点重试 · Esc 回大厅 · F12 截图", self.banner)
                }
            }
            OriginalScreen::Options => {
                format!("ra2 · 选项 · {} · 视频循环分辨率 · Esc 返回 · F12 截图", self.banner)
            }
            OriginalScreen::ExitConfirm => {
                format!("ra2 · 确认退出 · {} · Enter 退出 · Esc 取消 · F12 截图", self.banner)
            }
            OriginalScreen::Battle | OriginalScreen::Results => unreachable!(),
        };
        if title != self.last_shell_title {
            window.set_title(&title);
            self.last_shell_title = title;
        }
    }

    pub(super) fn redraw(&mut self) {
        if self.screen.pumps_session() {
            if let Some(ctrl) = self.battle_controller.as_mut() {
                let prev = ctrl.take_pump_clock();
                let dt = Instant::now().duration_since(prev).as_secs_f64();
                let (nav, sim_dt) = ctrl.pump(dt);
                self.renderer.timings.simulation = Some(sim_dt);
                let assets = self.menu_assets.as_ref().and_then(|a| a.source.as_ref());
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str(), self.menu_font.as_ref(), assets);
                self.apply_nav(nav);
            }
        }
        else if self.screen.requires_session() {
            if let Some(ctrl) = self.battle_controller.as_mut() {
                let _ = ctrl.take_pump_clock();
                self.renderer.timings.simulation = None;
                let assets = self.menu_assets.as_ref().and_then(|a| a.source.as_ref());
                ctrl.draw_frame(&mut self.renderer, self.window.as_ref(), self.screen.as_str(), self.menu_font.as_ref(), assets);
            }
        }
        else {
            // 前置页：无色块菜单。原版 SHP 未接前仅标题 + 可选大厅地图预览。
            self.renderer.timings.simulation = None;
            self.renderer.timings.presentation_build = None;
            if self.screen == OriginalScreen::Splash {
                self.tick_splash();
            }
            if self.screen == OriginalScreen::LoadScreen {
                self.poll_load_job();
            }
            if matches!(
                self.screen,
                OriginalScreen::MainMenu
                    | OriginalScreen::SinglePlayerMenu
                    | OriginalScreen::Campaign
                    | OriginalScreen::Options
                    | OriginalScreen::ExitConfirm
                    | OriginalScreen::SkirmishLobby
                    | OriginalScreen::ChooseMap
            ) {
                let dt = self.menu_movie_clock.replace(Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0).min(0.25);
                let movie_advanced = self.menu_movie.as_mut().is_some_and(|m| m.tick(dt));
                let status_advanced = self.status_line.tick(dt);
                // WARNING 窗内动画：分类器间隔 100ms（10 FPS），勿与全局 15 FPS chrome 时钟混用。
                const WARN_FRAME_SECS: f64 = 0.1;
                let warn_pages = matches!(
                    self.screen,
                    OriginalScreen::MainMenu
                        | OriginalScreen::SinglePlayerMenu
                        | OriginalScreen::Campaign
                        | OriginalScreen::Options
                        | OriginalScreen::ExitConfirm
                );
                let mut panel_advanced = false;
                if warn_pages {
                    let panel_dt =
                        self.menu_panel_anim_clock.replace(Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0).min(0.25);
                    self.menu_panel_anim_accum += panel_dt;
                    while self.menu_panel_anim_accum >= WARN_FRAME_SECS {
                        self.menu_panel_anim_accum -= WARN_FRAME_SECS;
                        self.menu_panel_anim_frame = self.menu_panel_anim_frame.wrapping_add(1);
                        panel_advanced = true;
                    }
                }
                else {
                    self.menu_panel_anim_clock = None;
                }
                let mut side_advanced = false;
                if self.screen == OriginalScreen::Campaign {
                    let side_hot = matches!(self.menu_hovered_entry, Some("allied" | "tutorial" | "soviet")) || self.campaign_side.is_some();
                    if side_hot {
                        const SIDE_FRAME_SECS: f64 = 1.0 / 12.0;
                        let side_dt =
                            self.campaign_side_anim_clock.replace(Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0).min(0.25);
                        self.campaign_side_anim_accum += side_dt;
                        while self.campaign_side_anim_accum >= SIDE_FRAME_SECS {
                            self.campaign_side_anim_accum -= SIDE_FRAME_SECS;
                            self.campaign_side_anim_frame = self.campaign_side_anim_frame.wrapping_add(1).max(1);
                            side_advanced = true;
                        }
                    }
                    else {
                        self.campaign_side_anim_clock = None;
                    }
                }
                if movie_advanced || side_advanced || status_advanced || panel_advanced {
                    self.refresh_menu_backdrop();
                }
                else if let Some(reason) = self.menu_movie.as_ref().and_then(|m| m.stalled_reason()) {
                    if !self.banner.contains("影片失步") {
                        self.banner = format!("{} · 影片失步 · {reason}", self.banner);
                    }
                }
            }
            if self.screen == OriginalScreen::SkirmishLobby {
                let ready = self.poll_lobby_preview();
                if ready {
                    self.refresh_menu_backdrop();
                    let map = self.selected_map.as_deref().unwrap_or("?");
                    self.banner = format!("预览就绪 · {map}");
                }
                else if self.lobby_preview_job.is_some() {
                    let map = self.selected_map.as_deref().unwrap_or("?");
                    self.banner = format!("预览生成中… {map}");
                }
            }
            self.renderer.draw_frame(None);
            self.refresh_shell_title();
        }
        self.flush_pending_screenshot();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
