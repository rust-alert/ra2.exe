//! 遭遇战大厅与选图。

use ra_layout::ui_layout;
use ra_renderer::RgbaImage;
use ra_types::AssetSource;
use ra_widgets::fs_source::GameAssetSource;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::skirmish_setup::{self, hover_entry_at, side_flag_pcx};
use ra_widgets::ui_compose::SkirmishChromeSprites;
use winit::event::KeyEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::host::boot;
use crate::host::preview_job::PreviewJob;

use super::Shell;

impl Shell {
    /// 惰性加载遭遇战勾选 / 滑条拇指 / 旗标 PCX。
    pub(super) fn ensure_skirmish_chrome(&mut self) {
        self.ensure_menu_assets();
        let flag_key = self
            .skirmish
            .row_sides
            .iter()
            .map(|i| ra_widgets::skirmish_setup::LOBBY_SIDES[(*i as usize) % ra_widgets::skirmish_setup::LOBBY_SIDES.len()])
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
            for i in 0..ui_layout::SKIRMISH_ROW_COUNT {
                let side = self.skirmish.row_side(i);
                chrome.row_flags[i] = Self::load_pcx_rgba(source, side_flag_pcx(side));
            }
            chrome.flag = chrome.row_flags[0].clone();
            chrome.ai_flag = chrome.row_flags[1].clone();
            self.skirmish_chrome_side = Some(flag_key);
        }
        self.skirmish_chrome = Some(chrome);
    }

    /// 遭遇战左栏按下：勾选 / 滑条优先于右栏按钮。
    pub(super) fn handle_skirmish_press(&mut self) -> bool {
        let layout = ui_layout::skirmish_lobby_layout(0, 0);
        let (x, y) = self.shell_cursor_px();
        let ai_rows = self.lobby_ai_rows();
        if self.skirmish.on_press(&layout, x, y, ai_rows).is_none() {
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
        let layout = ui_layout::skirmish_lobby_layout(0, 0);
        let (x, y) = self.shell_cursor_px();
        if !self.skirmish.on_drag(&layout, x, y) {
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

    /// 进入选图页并快照当前优选地图（取消时还原）。
    pub(super) fn open_choose_map_page(&mut self) {
        self.ensure_lobby_maps();
        self.ensure_lobby_modes();
        self.choose_map_revert = Some(self.skirmish.preferred_map.clone());
        if self.selected_map.is_none() {
            self.selected_map = self.skirmish.preferred_map.clone().or_else(|| self.lobby_maps.first().map(|m| m.file_name.clone()));
        }
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
}
