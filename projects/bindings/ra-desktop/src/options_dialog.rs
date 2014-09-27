//! 选项对话框：左五区控件 + 右栏接受/取消/主菜单。
//!
//! 布局对齐原版选项板（显示 / 游戏 / 界面 / 音效），并增补质感呈现区；右栏动作为接受、取消、主菜单。
//! 本模块只持草稿状态与命中，不直接碰窗口或配置落盘。

use ra_types::{DisplayMode, PresentFeel, PresentMode};

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
    /// 质感伽马档（映射到 `PresentFeel.gamma`）。
    PresentGamma,
    /// 质感高光收敛档（映射到 `PresentFeel.highlight_roll_off`）。
    PresentRollOff,
}

impl OptionsTrackbar {
    /// 滑条最大值（含）。
    pub const fn max(self) -> u8 {
        match self {
            Self::Detail | Self::Difficulty => 2,
            Self::Scroll => 6,
            Self::Music | Self::Sound | Self::Voice => 10,
            Self::PresentGamma | Self::PresentRollOff => 20,
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
    /// 启用 16 位质感呈现。
    Present16bit,
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
    /// 壳层质感呈现草稿。
    pub present: PresentFeel,
    /// 分辨率下拉是否展开。
    pub resolution_open: bool,
    /// 正在拖动的滑条。
    pub dragging: Option<OptionsTrackbar>,
}

impl OptionsDialogState {
    /// 从当前壳层显示档、音量与质感构造草稿。
    pub fn from_shell(display_mode: DisplayMode, music_vol: f32, sfx_vol: f32, present: PresentFeel) -> Self {
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
            present: present.sanitized(),
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

    fn track_value_mut(&mut self, id: OptionsTrackbar) -> Option<&mut u8> {
        match id {
            OptionsTrackbar::Detail => Some(&mut self.detail),
            OptionsTrackbar::Difficulty => Some(&mut self.difficulty),
            OptionsTrackbar::Scroll => Some(&mut self.scroll),
            OptionsTrackbar::Music => Some(&mut self.music),
            OptionsTrackbar::Sound => Some(&mut self.sound),
            OptionsTrackbar::Voice => Some(&mut self.voice),
            OptionsTrackbar::PresentGamma | OptionsTrackbar::PresentRollOff => None,
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
            OptionsTrackbar::PresentGamma => gamma_to_pos(self.present.gamma),
            OptionsTrackbar::PresentRollOff => roll_to_pos(self.present.highlight_roll_off),
        }
    }

    /// 按壳层像素处理按下。
    pub fn on_press(&mut self, layout: &OptionsDialogLayout, x: i32, y: i32) -> Option<OptionsHit> {
        let hit = layout.hit_at(x, y, self.resolution_open)?;
        match hit {
            OptionsHit::Toggle(id) => match id {
                OptionsCheckbox::Tooltips => self.tooltips = !self.tooltips,
                OptionsCheckbox::Scanlines => self.scanlines = !self.scanlines,
                OptionsCheckbox::ShowDamage => self.show_damage = !self.show_damage,
                OptionsCheckbox::Present16bit => {
                    self.present.mode = if self.present.is_active() {
                        PresentMode::Off
                    } else {
                        PresentMode::Bit16
                    };
                }
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

    /// 拖动中更新滑条；档位变化时返回 `true`。
    pub fn on_drag(&mut self, layout: &OptionsDialogLayout, x: i32, _y: i32) -> bool {
        let Some(id) = self.dragging
        else {
            return false;
        };
        let before = self.track_value(id);
        self.apply_track_at(layout, id, x);
        self.track_value(id) != before
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
        let pos = pos.min(max);
        match id {
            OptionsTrackbar::PresentGamma => {
                self.present.gamma = gamma_from_pos(pos);
                self.present = self.present.sanitized();
            }
            OptionsTrackbar::PresentRollOff => {
                self.present.highlight_roll_off = roll_from_pos(pos);
                self.present = self.present.sanitized();
            }
            other => {
                if let Some(slot) = self.track_value_mut(other) {
                    *slot = pos;
                }
            }
        }
    }
}

fn vol_to_pos(v: f32) -> u8 {
    ((v.clamp(0.0, 1.0) * 10.0).round() as u8).min(10)
}

fn pos_to_vol(p: u8) -> f32 {
    f32::from(p.min(10)) / 10.0
}

/// 伽马滑条：`0..=20` → `0.50..=2.50`（步长 0.10）。
fn gamma_from_pos(pos: u8) -> f32 {
    0.5 + f32::from(pos.min(20)) * 0.1
}

fn gamma_to_pos(gamma: f32) -> u8 {
    (((gamma.clamp(0.5, 2.5) - 0.5) / 0.1).round() as u8).min(20)
}

/// 高光收敛滑条：`0..=20` → `0.00..=1.00`（步长 0.05）。
fn roll_from_pos(pos: u8) -> f32 {
    f32::from(pos.min(20)) * 0.05
}

fn roll_to_pos(roll: f32) -> u8 {
    ((roll.clamp(0.0, 1.0) / 0.05).round() as u8).min(20)
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
    /// 质感呈现区标题。
    pub sec_present: RectPx,
    /// 音效区标题。
    pub sec_audio: RectPx,
    /// 详细程度滑条。
    pub track_detail: RectPx,
    /// 分辨率下拉。
    pub resolution: RectPx,
    /// 难度滑条。
    pub track_difficulty: RectPx,
    /// 三勾选（提示 / 扫描线 / 损害）。
    pub checks: [RectPx; 3],
    /// 滚动滑条。
    pub track_scroll: RectPx,
    /// 16 位质感勾选。
    pub check_present: RectPx,
    /// 质感伽马滑条。
    pub track_present_gamma: RectPx,
    /// 质感高光收敛滑条。
    pub track_present_roll: RectPx,
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
        // 略压原版四区间距，腾出「质感」区（仍落在 800×600 内容板内）。
        let y0 = content.y + 12;
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
            track_detail: RectPx::new(left, y0 + 32, col_w, 22),
            resolution: RectPx::new(left + col_w + 16, y0 + 32, col_w, 28),
            sec_game: RectPx::new(left, y0 + 80, usable_w, 18),
            track_difficulty: RectPx::new(left, y0 + 112, usable_w - 40, 22),
            sec_ui: RectPx::new(left, y0 + 168, usable_w, 18),
            checks: [
                RectPx::new(left, y0 + 198, 220, 22),
                RectPx::new(left, y0 + 224, 220, 22),
                RectPx::new(left, y0 + 250, 220, 22),
            ],
            track_scroll: RectPx::new(left + col_w + 16, y0 + 198, col_w, 22),
            sec_present: RectPx::new(left, y0 + 290, usable_w, 18),
            check_present: RectPx::new(left, y0 + 318, 280, 22),
            track_present_gamma: RectPx::new(left, y0 + 348, usable_w - 40, 22),
            track_present_roll: RectPx::new(left, y0 + 378, usable_w - 40, 22),
            sec_audio: RectPx::new(left, y0 + 420, usable_w, 18),
            track_music: RectPx::new(left, y0 + 448, usable_w - 40, 22),
            track_sound: RectPx::new(left, y0 + 482, usable_w - 40, 22),
            track_voice: RectPx::new(left, y0 + 516, usable_w - 40, 22),
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
            OptionsTrackbar::PresentGamma => self.track_present_gamma,
            OptionsTrackbar::PresentRollOff => self.track_present_roll,
        }
    }

    /// 分辨率下拉展开后的行矩形。
    pub fn resolution_row(self, index: usize) -> RectPx {
        RectPx::new(self.resolution.x, self.resolution.y + self.resolution.h + index as i32 * 24, self.resolution.w, 24)
    }

    /// 壳层像素命中。`resolution_open` 为真时才命中下拉行。
    pub fn hit_at(self, x: i32, y: i32, resolution_open: bool) -> Option<OptionsHit> {
        for (i, id) in OPTIONS_RAIL_IDS.iter().enumerate() {
            if self.rail[i].contains(x, y) {
                return Some(match *id {
                    "accept" => OptionsHit::Accept,
                    "cancel" => OptionsHit::Cancel,
                    _ => OptionsHit::MainMenu,
                });
            }
        }
        if resolution_open {
            for i in 0..DisplayMode::ALL.len() {
                if self.resolution_row(i).contains(x, y) {
                    return Some(OptionsHit::ResolutionRow(i));
                }
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
        if self.check_present.contains(x, y) {
            return Some(OptionsHit::Toggle(OptionsCheckbox::Present16bit));
        }
        for id in [
            OptionsTrackbar::Detail,
            OptionsTrackbar::Difficulty,
            OptionsTrackbar::Scroll,
            OptionsTrackbar::Music,
            OptionsTrackbar::Sound,
            OptionsTrackbar::Voice,
            OptionsTrackbar::PresentGamma,
            OptionsTrackbar::PresentRollOff,
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
        let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.4, 0.7, PresentFeel::DEFAULT);
        let track = layout.track_music;
        state.on_press(&layout, track.x + 6, track.y + 4);
        assert_eq!(state.music, 0);
        state.on_press(&layout, track.x + track.w - 2, track.y + 4);
        assert_eq!(state.music, 10);
    }

    #[test]
    fn resolution_row_selects_mode() {
        let layout = OptionsDialogLayout::new();
        let mut state = OptionsDialogState::from_shell(DisplayMode::W640H480, 0.5, 0.5, PresentFeel::DEFAULT);
        state.resolution_open = true;
        let row = layout.resolution_row(2);
        state.on_press(&layout, row.x + 4, row.y + 4);
        assert_eq!(state.display_mode, DisplayMode::W1024H768);
        assert!(!state.resolution_open);
    }

    #[test]
    fn present_toggle_and_gamma_track() {
        let layout = OptionsDialogLayout::new();
        let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.5, 0.5, PresentFeel::DEFAULT);
        assert!(state.present.is_active());
        state.on_press(&layout, layout.check_present.x + 4, layout.check_present.y + 4);
        assert!(!state.present.is_active());
        state.on_press(&layout, layout.check_present.x + 4, layout.check_present.y + 4);
        assert!(state.present.is_active());

        let track = layout.track_present_gamma;
        state.on_press(&layout, track.x + 6, track.y + 4);
        assert!((state.present.gamma - 0.5).abs() < 1e-3);
        state.on_press(&layout, track.x + track.w - 2, track.y + 4);
        assert!((state.present.gamma - 2.5).abs() < 1e-3);

        let roll = layout.track_present_roll;
        state.on_press(&layout, roll.x + 6, roll.y + 4);
        assert!((state.present.highlight_roll_off - 0.0).abs() < 1e-3);
    }

    #[test]
    fn present_controls_fit_content() {
        let layout = OptionsDialogLayout::new();
        let bottom = layout.track_voice.y + layout.track_voice.h;
        assert!(bottom <= layout.content.y + layout.content.h);
        assert!(layout.sec_present.y < layout.sec_audio.y);
    }
}
