//! 遭遇战大厅与选图。

use ra_layout;
use ra_map::map_matches_game_mode_filter;
use ra_renderer::RgbaImage;
use ra_types::AssetSource;
use ra_widgets::fs_source::GameAssetSource;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::skirmish_setup::{self, hover_entry_at, side_flag_pcx_candidates};
use ra_widgets::compose::{self, SkirmishChromeSprites};
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::host::boot::{self, BootMapCandidate};
use crate::host::preview_job::PreviewJob;

use super::Shell;

impl Shell {
    /// 惰性加载遭遇战勾选 / 滑条拇指 / 旗标 PCX。
    pub(super) fn ensure_skirmish_chrome(&mut self) {
        self.ensure_menu_assets();
        let flag_key = (0..ra_layout::SKIRMISH_ROW_COUNT)
            .map(|i| self.skirmish.row_side(i).to_string())
            .collect::<Vec<_>>()
            .join(",");
        let need_flag = self.skirmish_chrome_side.as_deref() != Some(flag_key.as_str());
        let need_base = self
            .skirmish_chrome
            .as_ref()
            .map(|c| c.checkbox_off.is_none() || c.track_cap_l.is_none() || c.combo_arrow.is_none())
            .unwrap_or(true);
        if !need_base && !need_flag {
            return;
        }
        let Some(source) = self.menu_assets.as_ref().and_then(|a| a.source.as_ref())
        else {
            return;
        };
        let mut chrome = self.skirmish_chrome.take().unwrap_or_default();
        if need_base {
            chrome.checkbox_off = Self::load_pcx_rgba(source, "cue_i.pcx");
            chrome.checkbox_on = Self::load_pcx_rgba(source, "cce_i.pcx");
            chrome.track_thumb = Self::load_pcx_rgba(source, "trakgrip.pcx");
            chrome.track_cap_l = Self::load_pcx_rgba(source, "trofl.pcx");
            chrome.track_cap_m = Self::load_pcx_rgba(source, "trofm.pcx");
            chrome.track_cap_r = Self::load_pcx_rgba(source, "trofr.pcx");
            chrome.combo_arrow = Self::load_pcx_rgba(source, "dnarrowr.pcx");
            chrome.combo_arrow_pressed = Self::load_pcx_rgba(source, "dnarrowp.pcx");
        }
        if need_flag {
            for i in 0..ra_layout::SKIRMISH_ROW_COUNT {
                let side = self.skirmish.row_side(i);
                let prefix = self
                    .lobby_countries
                    .iter()
                    .find(|c| c.id.eq_ignore_ascii_case(side))
                    .map(|c| c.prefix.as_str())
                    .unwrap_or("");
                // 已知 id 映射优先；再试 `{prefix}i.pcx`（如 `USA`→`usai.pcx`）。
                let prefix_flag = if prefix.len() >= 3 {
                    Some(format!("{}i.pcx", prefix[..3].to_ascii_lowercase()))
                } else {
                    None
                };
                chrome.row_flags[i] = side_flag_pcx_candidates(side)
                    .iter()
                    .copied()
                    .chain(prefix_flag.as_deref())
                    .find_map(|name| Self::load_pcx_rgba(source, name));
            }
            chrome.flag = chrome.row_flags[0].clone();
            chrome.ai_flag = chrome.row_flags[1].clone();
            self.skirmish_chrome_side = Some(flag_key);
        }
        self.skirmish_chrome = Some(chrome);
    }

    /// 遭遇战左栏按下：勾选 / 滑条优先于右栏按钮。
    pub(super) fn handle_skirmish_press(&mut self) -> bool {
        let (x, y) = self.shell_cursor_px();
        let ai_rows = self.lobby_ai_rows();
        if self.skirmish.on_press(x, y, ai_rows).is_none() {
            return false;
        }
        self.skirmish_pointer_consumed = true;
        self.play_menu_click();
        self.refresh_menu_backdrop();
        true
    }

    /// 当前选中地图对应的 AI 行数。
    pub(super) fn lobby_ai_rows(&self) -> usize {
        let slots = self
            .selected_map
            .as_ref()
            .and_then(|sel| self.lobby_maps.iter().find(|m| &m.file_name == sel))
            .or_else(|| self.lobby_maps.first())
            .map(|m| m.start_slots)
            .unwrap_or(4);
        boot::skirmish_ai_row_count(slots)
    }

    /// 遭遇战滑条拖动。
    pub(super) fn handle_skirmish_drag(&mut self) -> bool {
        if self.skirmish.dragging.is_none() {
            return false;
        }
        let (x, y) = self.shell_cursor_px();
        if !self.skirmish.on_drag(x, y) {
            return false;
        }
        self.refresh_menu_backdrop();
        true
    }

    /// 玩家名编辑中的键盘输入。返回 `true` 表示已消费（勿再走大厅快捷键）。
    pub(super) fn handle_skirmish_name_key(&mut self, key_ev: &winit::event::KeyEvent) -> bool {
        if self.screen != OriginalScreen::SkirmishLobby || !self.skirmish.player_name_editing {
            return false;
        }
        match key_ev.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => {
                if self.skirmish.backspace_name() {
                    self.refresh_menu_backdrop();
                }
            }
            PhysicalKey::Code(KeyCode::Escape) | PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                self.skirmish.end_name_edit();
                self.refresh_menu_backdrop();
            }
            _ => {
                if let Some(text) = key_ev.text.as_deref() {
                    if self.skirmish.append_name_text(text) {
                        self.refresh_menu_backdrop();
                    }
                }
            }
        }
        true
    }

    pub(super) fn ensure_lobby_maps(&mut self) {
        if !self.lobby_maps.is_empty() {
            return;
        }
        self.lobby_maps = boot::list_install_boot_maps();
        if self.selected_map.is_none() {
            self.selected_map = self.lobby_maps.first().map(|m| m.file_name.clone());
        }
        if self.skirmish.preferred_map.is_none() {
            self.skirmish.preferred_map = self.selected_map.clone();
        }
        tracing::info!(
            count = self.lobby_maps.len(),
            selected = ?self.selected_map,
            "遭遇战地图列表已刷新"
        );
    }

    /// 惰性装载离线遭遇战可选模式（`mpmodes.ini` 可见子集）。
    pub(super) fn ensure_lobby_modes(&mut self) {
        if !self.lobby_modes.is_empty() {
            return;
        }
        self.lobby_modes = boot::list_install_skirmish_modes();
        if self.selected_mode_id.is_none() {
            self.selected_mode_id = self.lobby_modes.first().map(|m| m.id);
        } else if let Some(id) = self.selected_mode_id {
            if !self.lobby_modes.iter().any(|m| m.id == id) {
                self.selected_mode_id = self.lobby_modes.first().map(|m| m.id);
            }
        }
        tracing::info!(
            count = self.lobby_modes.len(),
            selected = ?self.selected_mode_id,
            "遭遇战模式列表已刷新"
        );
    }

    /// 惰性装载遭遇战可选国家 / 势力（rules `[Countries]` / `[Sides]`）。
    pub(super) fn ensure_lobby_sides(&mut self) {
        if !self.lobby_countries.is_empty() {
            return;
        }
        let (countries, sides) = boot::list_install_skirmish_countries();
        self.lobby_side_groups = sides;
        self.lobby_countries = countries;
        let ids: Vec<String> = self.lobby_countries.iter().map(|c| c.id.clone()).collect();
        self.skirmish.set_lobby_sides(ids);
        tracing::info!(
            countries = self.lobby_countries.len(),
            side_groups = self.lobby_side_groups.len(),
            selected = %self.skirmish.side,
            "遭遇战国家 / 势力列表已刷新"
        );
    }

    /// 当前选中模式的地图过滤标签；无选中时回退 `standard`。
    pub(super) fn selected_mode_map_filter(&self) -> &str {
        self.selected_mode_id
            .and_then(|id| self.lobby_modes.iter().find(|m| m.id == id))
            .map(|m| m.map_filter.as_str())
            .unwrap_or("standard")
    }

    /// 匹配当前模式 `map_filter` 的大厅地图（保序）。
    pub(super) fn maps_matching_selected_mode(&self) -> Vec<BootMapCandidate> {
        let filter = self.selected_mode_map_filter();
        self.lobby_maps
            .iter()
            .filter(|m| map_matches_game_mode_filter(&m.game_modes, filter))
            .cloned()
            .collect()
    }

    /// 菜单命中用的地图列表：选图页按模式过滤，其它页用完整大厅表。
    pub(super) fn maps_for_menu_hit(&self) -> Vec<BootMapCandidate> {
        if self.screen == OriginalScreen::ChooseMap {
            self.maps_matching_selected_mode()
        } else {
            self.lobby_maps.clone()
        }
    }

    /// 若当前选中地图不匹配模式过滤，改选第一张匹配图（可能清空）。
    pub(super) fn clamp_selected_map_to_mode_filter(&mut self) {
        let filter = self.selected_mode_map_filter().to_string();
        let still_ok = self
            .selected_map
            .as_ref()
            .and_then(|sel| self.lobby_maps.iter().find(|m| &m.file_name == sel))
            .is_some_and(|m| map_matches_game_mode_filter(&m.game_modes, &filter));
        if still_ok {
            return;
        }
        self.selected_map = self
            .lobby_maps
            .iter()
            .find(|m| map_matches_game_mode_filter(&m.game_modes, &filter))
            .map(|m| m.file_name.clone());
        if self.screen == OriginalScreen::ChooseMap {
            self.ensure_lobby_preview();
        }
    }

    pub(super) fn ensure_lobby_preview(&mut self) {
        let Some(name) = self.selected_map.clone()
        else {
            self.lobby_preview = None;
            self.lobby_preview_for = None;
            self.lobby_preview_job = None;
            return;
        };
        if self.lobby_preview_for.as_deref() == Some(name.as_str()) {
            return;
        }
        if self.lobby_preview_job.as_ref().is_some_and(|j| j.map_name() == name) {
            return;
        }
        self.lobby_preview_job = Some(PreviewJob::start(name));
    }

    pub(super) fn poll_lobby_preview(&mut self) -> bool {
        let Some(job) = self.lobby_preview_job.as_ref()
        else {
            return false;
        };
        match job.try_take() {
            Ok(Some(result)) => {
                self.lobby_preview_job = None;
                let still_selected = self.selected_map.as_deref() == Some(result.map_name.as_str());
                if !still_selected {
                    return false;
                }
                match result.image {
                    Some(thumb) => {
                        tracing::info!(
                            map = %result.map_name,
                            w = thumb.width(),
                            h = thumb.height(),
                            "{}",
                            result.note
                        );
                        self.lobby_preview = Some(thumb);
                        self.lobby_preview_for = Some(result.map_name);
                    }
                    None => {
                        tracing::warn!(map = %result.map_name, "遭遇战大厅地图预览失败");
                        self.lobby_preview = None;
                        self.lobby_preview_for = Some(result.map_name);
                    }
                }
                true
            }
            Ok(None) => false,
            Err(()) => {
                self.lobby_preview_job = None;
                false
            }
        }
    }

    pub(super) fn cycle_lobby_map(&mut self, delta: isize) {
        self.ensure_lobby_maps();
        if self.lobby_maps.is_empty() {
            self.selected_map = None;
            self.skirmish.preferred_map = None;
            return;
        }
        let cur = self.selected_map.as_ref().and_then(|name| self.lobby_maps.iter().position(|m| &m.file_name == name)).unwrap_or(0);
        let n = self.lobby_maps.len() as isize;
        let next = ((cur as isize + delta).rem_euclid(n)) as usize;
        self.select_lobby_map_index(next);
    }

    /// 跳到大厅地图列表首项或末项（空列表时清空选中）。
    pub(super) fn jump_lobby_map_edge(&mut self, to_end: bool) {
        self.ensure_lobby_maps();
        if self.lobby_maps.is_empty() {
            self.selected_map = None;
            self.skirmish.preferred_map = None;
            return;
        }
        let idx = if to_end { self.lobby_maps.len() - 1 } else { 0 };
        self.select_lobby_map_index(idx);
    }

    pub(super) fn select_lobby_map_index(&mut self, index: usize) {
        let Some(map) = self.lobby_maps.get(index)
        else {
            return;
        };
        self.selected_map = Some(map.file_name.clone());
        self.skirmish.preferred_map = self.selected_map.clone();
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    /// 选中选图页游戏类型列表中的一项。
    pub(super) fn select_lobby_mode_index(&mut self, index: usize) {
        let Some(mode) = self.lobby_modes.get(index)
        else {
            return;
        };
        self.selected_mode_id = Some(mode.id);
        self.clamp_selected_map_to_mode_filter();
        self.sync_map_list_scroll_to_selection();
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    /// 进入选图页并快照当前优选地图（取消时还原）。
    pub(super) fn open_choose_map_page(&mut self) {
        self.ensure_lobby_maps();
        self.ensure_lobby_modes();
        self.choose_map_revert = Some(self.skirmish.preferred_map.clone());
        if self.selected_map.is_none() {
            self.selected_map = self.skirmish.preferred_map.clone().or_else(|| self.lobby_maps.first().map(|m| m.file_name.clone()));
        }
        self.clamp_selected_map_to_mode_filter();
        self.map_list_scroll = 0;
        self.sync_map_list_scroll_to_selection();
        self.set_screen(OriginalScreen::ChooseMap);
        self.banner = "选图".into();
        self.refresh_shell_title();
    }

    /// 使用当前选中地图并返回遭遇战大厅。
    pub(super) fn confirm_choose_map(&mut self) {
        self.choose_map_revert = None;
        if let Some(name) = self.selected_map.clone() {
            self.skirmish.preferred_map = Some(name);
        }
        self.set_screen(OriginalScreen::SkirmishLobby);
        self.banner = "已选用地图".into();
        self.refresh_shell_title();
    }

    /// 取消选图：还原进入页前的优选地图并回大厅。
    pub(super) fn cancel_choose_map(&mut self) {
        if let Some(prev) = self.choose_map_revert.take() {
            self.skirmish.preferred_map = prev.clone();
            self.selected_map = prev.or_else(|| self.lobby_maps.first().map(|m| m.file_name.clone()));
            self.ensure_lobby_preview();
        }
        self.set_screen(OriginalScreen::SkirmishLobby);
        self.banner = "已取消选图".into();
        self.refresh_shell_title();
    }

    /// 选图页地图列表可视行数。
    pub(super) fn choose_map_visible_row_count(&self) -> usize {
        let list = ra_layout::rect_px_from_snapshot(&ra_layout::solve_choose_map(), "map_list");
        ra_layout::choose_map_visible_rows(list.h)
    }

    /// 使当前选中地图落在选图列表可视窗内。
    pub(super) fn sync_map_list_scroll_to_selection(&mut self) {
        let maps = self.maps_matching_selected_mode();
        let visible = self.choose_map_visible_row_count();
        let index = self
            .selected_map
            .as_ref()
            .and_then(|sel| maps.iter().position(|m| &m.file_name == sel))
            .unwrap_or(0);
        self.map_list_scroll =
            ra_layout::scroll_map_list_to_reveal(self.map_list_scroll, index, maps.len(), visible);
    }

    /// 选图页滚轮 / 快捷键微调列表偏移。
    pub(super) fn nudge_map_list_scroll(&mut self, delta_rows: isize) {
        let maps = self.maps_matching_selected_mode();
        let visible = self.choose_map_visible_row_count();
        let max = maps.len().saturating_sub(visible);
        let next = (self.map_list_scroll as isize + delta_rows).clamp(0, max as isize) as usize;
        if next != self.map_list_scroll {
            self.map_list_scroll = next;
            self.refresh_menu_backdrop();
        }
    }
}
