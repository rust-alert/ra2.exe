//! 选项对话框：左四区控件 + 右栏接受/取消/主菜单。
//!
//! 布局对齐原版选项板（显示 / 游戏 / 界面 / 音效）；右栏动作为接受、取消、主菜单。
//! 本模块只持草稿状态与命中，不直接碰窗口或配置落盘。

use ra_types::DisplayMode;

use crate::ui_layout::{BUTTON_CELL_H, BUTTON_CELL_W, RIGHT_PANEL_W, RectPx, SHELL_BASE_H, SHELL_BASE_W, main_menu_layout};

/// 右栏按钮入口 id（与 [`crate::ui_slots`] 一致）。
pub const OPTIONS_RAIL_IDS: [&str; 3] = ["accept", "cancel", "main_menu"];

/// 滑条种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsTrackbar {
    /// 画面详细程度（0..=2）。
    Detail,
    /// 难度（0..=2）。
    Difficulty,
    /// 滚动速率（0..=6）。
    Scroll,
    /// 音乐音量（0..=10）。
    Music,
    /// 音效音量（0..=10）。
    Sound,
    /// 语音音量（0..=10）。
    Voice,
}

impl OptionsTrackbar {
    /// 滑条最大值（含）。
    pub const fn max(self) -> u8 {
        match self {
            Self::Detail | Self::Difficulty => 2,
            Self::Scroll => 6,
            Self::Music | Self::Sound | Self::Voice => 10,
        }
    }
}

/// 勾选框种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCheckbox {
    /// 工具提示。
    Tooltips,
    /// 扫描线。
    Scanlines,
    /// 显示损害特性。
    ShowDamage,
}

/// 选项页命中结果（含拖动起点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsHit {
    /// 右栏接受。
    Accept,
    /// 右栏取消。
    Cancel,
    /// 右栏主菜单。
    MainMenu,
    /// 勾选切换。
    Toggle(OptionsCheckbox),
    /// 点在滑条上（开始拖或跳档）。
    Track(OptionsTrackbar),
    /// 打开/关闭分辨率下拉。
    ResolutionCombo,
    /// 选中某一分辨率行。
    ResolutionRow(usize),
}

/// 选项草稿（进入页时从壳层快照，接受才提交）。
#[derive(Debug, Clone, PartialEq)]
pub struct OptionsDialogState {
    /// 画面详细程度。
    pub detail: u8,
    /// 客户区分辨率档。
    pub display_mode: DisplayMode,
    /// 难度。
    pub difficulty: u8,
    /// 工具提示。
    pub tooltips: bool,
    /// 扫描线。
    pub scanlines: bool,
    /// 显示损害特性。
    pub show_damage: bool,
    /// 滚动速率。
    pub scroll: u8,
    /// 音乐 0..=10。
    pub music: u8,
    /// 音效 0..=10。
    pub sound: u8,
    /// 语音 0..=10。
    pub voice: u8,
    /// 分辨率下拉是否展开。
    pub resolution_open: bool,
    /// 正在拖动的滑条。
    pub dragging: Option<OptionsTrackbar>,
}

impl OptionsDialogState {
    /// 从当前壳层显示档与音量构造草稿。
    pub fn from_shell(display_mode: DisplayMode, music_vol: f32, sfx_vol: f32) -> Self {
        Self {
            detail: 2,
            display_mode,
            difficulty: 2,
            tooltips: true,
            scanlines: false,
            show_damage: true,
            scroll: 6,
            music: vol_to_pos(music_vol),
            sound: vol_to_pos(sfx_vol),
            voice: vol_to_pos(sfx_vol),
            resolution_open: false,
            dragging: None,
        }
    }

    /// 音量滑条 → 0..1。
    pub fn music_volume_f32(&self) -> f32 {
        pos_to_vol(self.music)
    }

    /// 音效滑条 → 0..1。
    pub fn sound_volume_f32(&self) -> f32 {
        pos_to_vol(self.sound)
    }

    /// 语音滑条 → 0..1（暂与音效通道共用设备侧，仍独立存草稿）。
    pub fn voice_volume_f32(&self) -> f32 {
        pos_to_vol(self.voice)
    }

    fn track_value_mut(&mut self, id: OptionsTrackbar) -> &mut u8 {
        match id {
            OptionsTrackbar::Detail => &mut self.detail,
            OptionsTrackbar::Difficulty => &mut self.difficulty,
            OptionsTrackbar::Scroll => &mut self.scroll,
            OptionsTrackbar::Music => &mut self.music,
            OptionsTrackbar::Sound => &mut self.sound,
            OptionsTrackbar::Voice => &mut self.voice,
        }
    }

    /// 读取滑条档位（绘制拇指位置用）。
    pub fn track_value(&self, id: OptionsTrackbar) -> u8 {
        match id {
            OptionsTrackbar::Detail => self.detail,
            OptionsTrackbar::Difficulty => self.difficulty,
            OptionsTrackbar::Scroll => self.scroll,
            OptionsTrackbar::Music => self.music,
            OptionsTrackbar::Sound => self.sound,
            OptionsTrackbar::Voice => self.voice,
        }
    }

    /// 按壳层像素处理按下。
    pub fn on_press(&mut self, layout: &OptionsDialogLayout, x: i32, y: i32) -> Option<OptionsHit> {
        let hit = layout.hit_at(x, y)?;
        match hit {
            OptionsHit::Toggle(id) => match id {
                OptionsCheckbox::Tooltips => self.tooltips = !self.tooltips,
                OptionsCheckbox::Scanlines => self.scanlines = !self.scanlines,
                OptionsCheckbox::ShowDamage => self.show_damage = !self.show_damage,
            },
            OptionsHit::Track(id) => {
                self.dragging = Some(id);
                self.apply_track_at(layout, id, x);
            }
            OptionsHit::ResolutionCombo => {
                self.resolution_open = !self.resolution_open;
            }
            OptionsHit::ResolutionRow(i) => {
                if let Some(mode) = DisplayMode::ALL.get(i).copied() {
                    self.display_mode = mode;
                }
                self.resolution_open = false;
            }
            OptionsHit::Accept | OptionsHit::Cancel | OptionsHit::MainMenu => {}
        }
        Some(hit)
    }

    /// 拖动中更新滑条。
    pub fn on_drag(&mut self, layout: &OptionsDialogLayout, x: i32, _y: i32) -> bool {
        let Some(id) = self.dragging
        else {
            return false;
        };
        self.apply_track_at(layout, id, x);
        true
    }

    /// 松开；返回是否点在右栏导航钮上（由壳层消费）。
    pub fn on_release(&mut self) -> Option<OptionsHit> {
        self.dragging = None;
        None
    }

    fn apply_track_at(&mut self, layout: &OptionsDialogLayout, id: OptionsTrackbar, x: i32) {
        let track = layout.trackbar_rect(id);
        let max = id.max();
        let inner = (track.w - 12).max(1);
        let rel = (x - track.x - 6).clamp(0, inner);
        let pos = if max == 0 {
            0
        } else {
            ((rel as u32 * u32::from(max) + (inner as u32 / 2)) / inner as u32) as u8
        };
        *self.track_value_mut(id) = pos.min(max);
    }
}

fn vol_to_pos(v: f32) -> u8 {
    ((v.clamp(0.0, 1.0) * 10.0).round() as u8).min(10)
}

fn pos_to_vol(p: u8) -> f32 {
    f32::from(p.min(10)) / 10.0
}

/// 选项对话框一帧几何（800×600 内容坐标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionsDialogLayout {
    /// 画布。
    pub canvas: RectPx,
    /// 左侧大内容板。
    pub content: RectPx,
    /// 右栏顶盖。
    pub panel_top: RectPx,
    /// 右栏平铺起点。
    pub panel_tile: RectPx,
    /// 平铺条数。
    pub panel_tile_count: i32,
    /// 右栏底盖。
    pub panel_bottom: RectPx,
    /// 右栏标题。
    pub title: RectPx,
    /// 接受 / 取消 / 主菜单。
    pub rail: [RectPx; 3],
    /// 显示区标题。
    pub sec_display: RectPx,
    /// 游戏区标题。
    pub sec_game: RectPx,
    /// 界面区标题。
    pub sec_ui: RectPx,
    /// 音效区标题。
    pub sec_audio: RectPx,
    /// 详细程度滑条。
    pub track_detail: RectPx,
    /// 分辨率下拉。
    pub resolution: RectPx,
    /// 难度滑条。
    pub track_difficulty: RectPx,
    /// 三勾选。
    pub checks: [RectPx; 3],
    /// 滚动滑条。
    pub track_scroll: RectPx,
    /// 音乐 / 音效 / 语音。
    pub track_music: RectPx,
    pub track_sound: RectPx,
    pub track_voice: RectPx,
}

impl OptionsDialogLayout {
    /// 构造与主菜单同右栏几何的选项板。
    pub fn new() -> Self {
        let shell = main_menu_layout(0, 0);
        let panel_x = SHELL_BASE_W - RIGHT_PANEL_W;
        let content = RectPx::new(16, 16, panel_x - 24, SHELL_BASE_H - 32);
        let rail = [
            button_cell(panel_x, shell.panel_tile.y),
            button_cell(panel_x, shell.panel_tile.y + BUTTON_CELL_H),
            button_cell(panel_x, shell.panel_bottom.y - BUTTON_CELL_H),
        ];
        let left = content.x + 16;
        let usable_w = content.w - 32;
        let col_w = usable_w / 2 - 8;
        let y0 = content.y + 20;
        Self {
            canvas: RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H),
            content,
            panel_top: shell.panel_top,
            panel_tile: shell.panel_tile,
            panel_tile_count: shell.panel_tile_count,
            panel_bottom: shell.panel_bottom,
            title: shell.title,
            rail,
            sec_display: RectPx::new(left, y0, usable_w, 18),
            track_detail: RectPx::new(left, y0 + 36, col_w, 22),
            resolution: RectPx::new(left + col_w + 16, y0 + 36, col_w, 28),
            sec_game: RectPx::new(left, y0 + 100, usable_w, 18),
            track_difficulty: RectPx::new(left, y0 + 136, usable_w - 40, 22),
            sec_ui: RectPx::new(left, y0 + 200, usable_w, 18),
            checks: [
                RectPx::new(left, y0 + 236, 220, 22),
                RectPx::new(left, y0 + 266, 220, 22),
                RectPx::new(left, y0 + 296, 220, 22),
            ],
            track_scroll: RectPx::new(left + col_w + 16, y0 + 236, col_w, 22),
            sec_audio: RectPx::new(left, y0 + 360, usable_w, 18),
            track_music: RectPx::new(left, y0 + 396, usable_w - 40, 22),
            track_sound: RectPx::new(left, y0 + 440, usable_w - 40, 22),
            track_voice: RectPx::new(left, y0 + 484, usable_w - 40, 22),
        }
    }

    fn trackbar_rect(self, id: OptionsTrackbar) -> RectPx {
        match id {
            OptionsTrackbar::Detail => self.track_detail,
            OptionsTrackbar::Difficulty => self.track_difficulty,
            OptionsTrackbar::Scroll => self.track_scroll,
            OptionsTrackbar::Music => self.track_music,
            OptionsTrackbar::Sound => self.track_sound,
            OptionsTrackbar::Voice => self.track_voice,
        }
    }

    /// 分辨率下拉展开后的行矩形。
    pub fn resolution_row(self, index: usize) -> RectPx {
        RectPx::new(self.resolution.x, self.resolution.y + self.resolution.h + index as i32 * 24, self.resolution.w, 24)
    }

    /// 壳层像素命中。
    pub fn hit_at(self, x: i32, y: i32) -> Option<OptionsHit> {
        for (i, id) in OPTIONS_RAIL_IDS.iter().enumerate() {
            if self.rail[i].contains(x, y) {
                return Some(match *id {
                    "accept" => OptionsHit::Accept,
                    "cancel" => OptionsHit::Cancel,
                    _ => OptionsHit::MainMenu,
                });
            }
        }
        // 下拉展开时优先点选行。
        // 行命中由调用方结合 `resolution_open` 判断；此处若点在行上仍返回行。
        for i in 0..DisplayMode::ALL.len() {
            if self.resolution_row(i).contains(x, y) {
                return Some(OptionsHit::ResolutionRow(i));
            }
        }
        if self.resolution.contains(x, y) {
            return Some(OptionsHit::ResolutionCombo);
        }
        for (i, rect) in self.checks.iter().enumerate() {
            if rect.contains(x, y) {
                let id = match i {
                    0 => OptionsCheckbox::Tooltips,
                    1 => OptionsCheckbox::Scanlines,
                    _ => OptionsCheckbox::ShowDamage,
                };
                return Some(OptionsHit::Toggle(id));
            }
        }
        for id in [
            OptionsTrackbar::Detail,
            OptionsTrackbar::Difficulty,
            OptionsTrackbar::Scroll,
            OptionsTrackbar::Music,
            OptionsTrackbar::Sound,
            OptionsTrackbar::Voice,
        ] {
            if self.trackbar_rect(id).contains(x, y) {
                return Some(OptionsHit::Track(id));
            }
        }
        None
    }

    /// 悬停右栏下标（含全部三钮）。
    pub fn hover_rail_index(self, x: i32, y: i32) -> Option<usize> {
        self.rail.iter().position(|r| r.contains(x, y))
    }
}

fn button_cell(panel_x: i32, y: i32) -> RectPx {
    let x = panel_x + (RIGHT_PANEL_W - BUTTON_CELL_W);
    RectPx::new(x, y, BUTTON_CELL_W, BUTTON_CELL_H)
}

impl Default for OptionsDialogLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rail_accept_is_top_tile_cell() {
        let layout = OptionsDialogLayout::new();
        let shell = main_menu_layout(0, 0);
        assert_eq!(layout.rail[0].y, shell.panel_tile.y);
        assert!(layout.rail[2].y < shell.panel_bottom.y);
    }

    #[test]
    fn track_drag_maps_edges() {
        let layout = OptionsDialogLayout::new();
        let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.4, 0.7);
        let track = layout.track_music;
        state.on_press(&layout, track.x + 6, track.y + 4);
        assert_eq!(state.music, 0);
        state.on_press(&layout, track.x + track.w - 2, track.y + 4);
        assert_eq!(state.music, 10);
    }

    #[test]
    fn resolution_row_selects_mode() {
        let layout = OptionsDialogLayout::new();
        let mut state = OptionsDialogState::from_shell(DisplayMode::W640H480, 0.5, 0.5);
        let row = layout.resolution_row(2);
        state.on_press(&layout, row.x + 4, row.y + 4);
        assert_eq!(state.display_mode, DisplayMode::W1024H768);
        assert!(!state.resolution_open);
    }
}
