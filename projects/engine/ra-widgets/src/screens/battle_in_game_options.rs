//! 局内选项 `0xBBB`（暂停二级页）。

use ra_layout::{
    BATTLE_IN_GAME_OPTIONS_BUTTON_IDS, LayoutSnapshot, RectPx, rect_px_from_snapshot, solve_battle_in_game_options_at,
};

/// 局内选项草稿（Back 时写回宿主；本切片最小可玩）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleInGameOptionsState {
    /// 游戏速度滑条 0..=6。
    pub game_speed: u8,
    /// 滚屏速率滑条 0..=6。
    pub scroll_rate: u8,
    /// 目标线。
    pub target_lines: bool,
    /// 显示隐藏物。
    pub show_hidden: bool,
    /// 提示。
    pub tooltips: bool,
    /// 正在拖动的滑条 id。
    pub drag_track: Option<&'static str>,
}

impl Default for BattleInGameOptionsState {
    fn default() -> Self {
        Self {
            game_speed: 4,
            scroll_rate: 4,
            target_lines: true,
            show_hidden: false,
            tooltips: true,
            drag_track: None,
        }
    }
}

/// 局内选项命中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleInGameOptionsHit {
    /// 返回暂停菜单。
    Back,
    /// Sound 子页（占位）。
    Sound,
    /// Keyboard 子页（占位）。
    Keyboard,
    /// 游戏速度滑条。
    TrackGameSpeed,
    /// 滚屏滑条。
    TrackScrollRate,
    /// 目标线勾选。
    CheckTargetLines,
    /// 显示隐藏勾选。
    CheckShowHidden,
    /// 提示勾选。
    CheckTooltips,
}

impl BattleInGameOptionsHit {
    /// 由入口 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "back" => Some(Self::Back),
            "sound" => Some(Self::Sound),
            "keyboard" => Some(Self::Keyboard),
            "track_game_speed" => Some(Self::TrackGameSpeed),
            "track_scroll_rate" => Some(Self::TrackScrollRate),
            "check_target_lines" => Some(Self::CheckTargetLines),
            "check_show_hidden" => Some(Self::CheckShowHidden),
            "check_tooltips" => Some(Self::CheckTooltips),
            _ => None,
        }
    }

    /// 稳定入口 id。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Back => "back",
            Self::Sound => "sound",
            Self::Keyboard => "keyboard",
            Self::TrackGameSpeed => "track_game_speed",
            Self::TrackScrollRate => "track_scroll_rate",
            Self::CheckTargetLines => "check_target_lines",
            Self::CheckShowHidden => "check_show_hidden",
            Self::CheckTooltips => "check_tooltips",
        }
    }
}

/// 局内选项 snapshot。
pub fn options_snapshot(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    solve_battle_in_game_options_at(viewport_w, viewport_h)
}

/// 窗口像素命中。
pub fn hit_at(viewport_w: u32, viewport_h: u32, x: i32, y: i32) -> Option<BattleInGameOptionsHit> {
    let snap = options_snapshot(viewport_w, viewport_h);
    let hit = snap.hit_test(ra_layout::Point2 { x: x as f32, y: y as f32 })?;
    BattleInGameOptionsHit::from_entry_id(hit.id.0.as_str())
}

/// 右栏三钮矩形。
pub fn button_rects(viewport_w: u32, viewport_h: u32) -> [RectPx; 3] {
    let snap = options_snapshot(viewport_w, viewport_h);
    [
        rect_px_from_snapshot(&snap, BATTLE_IN_GAME_OPTIONS_BUTTON_IDS[0]),
        rect_px_from_snapshot(&snap, BATTLE_IN_GAME_OPTIONS_BUTTON_IDS[1]),
        rect_px_from_snapshot(&snap, BATTLE_IN_GAME_OPTIONS_BUTTON_IDS[2]),
    ]
}

/// 按滑条矩形与指针 x 估算 0..=max。
pub fn track_value_at(track: RectPx, x: i32, max: u8) -> u8 {
    if track.w <= 1 || max == 0 {
        return 0;
    }
    let t = ((x - track.x) as f32 / track.w as f32).clamp(0.0, 1.0);
    (t * f32::from(max) + 0.5).floor() as u8
}
