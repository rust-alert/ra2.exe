//! 对局放弃确认（暂停二级页）。

use ra_layout::{BATTLE_ABORT_CONFIRM_BUTTON_IDS, LayoutSnapshot, RectPx, rect_px_from_snapshot, solve_battle_abort_confirm_at};

/// 放弃确认命中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleAbortConfirmHit {
    /// 离开对局（不进结算）。
    Leave,
    /// 取消并恢复对局。
    Cancel,
}

impl BattleAbortConfirmHit {
    /// 由入口 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "leave" => Some(Self::Leave),
            "cancel" => Some(Self::Cancel),
            _ => None,
        }
    }

    /// 稳定入口 id。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Leave => "leave",
            Self::Cancel => "cancel",
        }
    }
}

/// 放弃确认 snapshot。
pub fn abort_snapshot(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    solve_battle_abort_confirm_at(viewport_w, viewport_h)
}

/// 全屏压暗矩形。
pub fn dim_rect(viewport_w: u32, viewport_h: u32) -> RectPx {
    rect_px_from_snapshot(&abort_snapshot(viewport_w, viewport_h), "dim")
}

/// 提示文案矩形。
pub fn prompt_rect(viewport_w: u32, viewport_h: u32) -> RectPx {
    rect_px_from_snapshot(&abort_snapshot(viewport_w, viewport_h), "prompt")
}

/// Leave / Cancel 钮矩形。
pub fn button_rects(viewport_w: u32, viewport_h: u32) -> [RectPx; 2] {
    let snap = abort_snapshot(viewport_w, viewport_h);
    [
        rect_px_from_snapshot(&snap, BATTLE_ABORT_CONFIRM_BUTTON_IDS[0]),
        rect_px_from_snapshot(&snap, BATTLE_ABORT_CONFIRM_BUTTON_IDS[1]),
    ]
}

/// 窗口像素命中。
pub fn hit_at(viewport_w: u32, viewport_h: u32, x: i32, y: i32) -> Option<BattleAbortConfirmHit> {
    let snap = abort_snapshot(viewport_w, viewport_h);
    let hit = snap.hit_test(ra_layout::Point2 { x: x as f32, y: y as f32 })?;
    BattleAbortConfirmHit::from_entry_id(hit.id.0.as_str())
}
