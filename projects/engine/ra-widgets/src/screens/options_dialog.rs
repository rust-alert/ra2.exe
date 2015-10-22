//! 选项对话框：左五区控件 + 右栏接受/取消/主菜单。
//!
//! 几何权威为 `solve_options_page` snapshot。本模块只持草稿状态与命中，不碰窗口或配置落盘。

use ra_types::{DisplayMode, PresentFeel, PresentMode};

use ra_layout::{
    popup_row_below, rect_px_from_snapshot, solve_options_page, LayoutSnapshot, RectPx,
    OPTIONS_RESOLUTION_ROW_H,
};

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

    /// snapshot 控件 id。
    pub const fn layout_id(self) -> &'static str {
        match self {
            Self::Detail => "track_detail",
            Self::Difficulty => "track_difficulty",
            Self::Scroll => "track_scroll",
            Self::Music => "track_music",
            Self::Sound => "track_sound",
            Self::Voice => "track_voice",
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

impl OptionsCheckbox {
    /// snapshot 控件 id。
    pub const fn layout_id(self) -> &'static str {
        match self {
            Self::Tooltips => "check_tooltips",
            Self::Scanlines => "check_scanlines",
            Self::ShowDamage => "check_damage",
            Self::Present16bit => "check_present",
        }
    }
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

    /// 按壳层像素处理按下（几何来自 `solve_options_page`）。
    pub fn on_press(&mut self, x: i32, y: i32) -> Option<OptionsHit> {
        let snap = solve_options_page();
        let hit = hit_at(&snap, x, y, self.resolution_open)?;
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
                self.apply_track_at(&snap, id, x);
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
    pub fn on_drag(&mut self, x: i32, _y: i32) -> bool {
        let Some(id) = self.dragging else {
            return false;
        };
        let snap = solve_options_page();
        let before = self.track_value(id);
        self.apply_track_at(&snap, id, x);
        self.track_value(id) != before
    }

    /// 松开；返回是否点在右栏导航钮上（由壳层消费）。
    pub fn on_release(&mut self) -> Option<OptionsHit> {
        self.dragging = None;
        None
    }

    fn apply_track_at(&mut self, snap: &LayoutSnapshot, id: OptionsTrackbar, x: i32) {
        let track = rect_px_from_snapshot(snap, id.layout_id());
        let max = id.max();
        let inner = (track.w - 12).max(1);
        let rel = (x - track.x - 6).clamp(0, inner);
        let pos = if max == 0 {
            0
        } else {
            ((rel as u32 * u32::from(max) + (inner as u32 / 2)) / inner as u32) as u8
        };
        let pos = pos.min(max);
        *self.track_value_mut(id) = pos;
    }
}

/// 分辨率下拉展开后的行矩形。
pub fn resolution_row_rect(snap: &LayoutSnapshot, index: usize) -> RectPx {
    popup_row_below(
        rect_px_from_snapshot(snap, "resolution"),
        OPTIONS_RESOLUTION_ROW_H,
        index,
    )
}

/// 壳层像素命中。`resolution_open` 为真时才命中下拉行。
pub fn hit_at(snap: &LayoutSnapshot, x: i32, y: i32, resolution_open: bool) -> Option<OptionsHit> {
    for id in OPTIONS_RAIL_IDS {
        if rect_px_from_snapshot(snap, id).contains(x, y) {
            return Some(match id {
                "accept" => OptionsHit::Accept,
                "cancel" => OptionsHit::Cancel,
                _ => OptionsHit::MainMenu,
            });
        }
    }
    if resolution_open {
        for i in 0..DisplayMode::ALL.len() {
            if resolution_row_rect(snap, i).contains(x, y) {
                return Some(OptionsHit::ResolutionRow(i));
            }
        }
    }
    if rect_px_from_snapshot(snap, "resolution").contains(x, y) {
        return Some(OptionsHit::ResolutionCombo);
    }
    for id in [
        OptionsCheckbox::Tooltips,
        OptionsCheckbox::Scanlines,
        OptionsCheckbox::ShowDamage,
        OptionsCheckbox::Present16bit,
    ] {
        if rect_px_from_snapshot(snap, id.layout_id()).contains(x, y) {
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
        if rect_px_from_snapshot(snap, id.layout_id()).contains(x, y) {
            return Some(OptionsHit::Track(id));
        }
    }
    None
}

fn vol_to_pos(v: f32) -> u8 {
    ((v.clamp(0.0, 1.0) * 10.0).round() as u8).min(10)
}

fn pos_to_vol(p: u8) -> f32 {
    f32::from(p.min(10)) / 10.0
}
