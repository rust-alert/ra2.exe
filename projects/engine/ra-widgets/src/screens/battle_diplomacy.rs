//! 对局外交子页（暂停层；只读同盟花名册）。

use ra_layout::{BATTLE_DIPLOMACY_BUTTON_IDS, BATTLE_DIPLOMACY_ROW_COUNT, LayoutSnapshot, RectPx, rect_px_from_snapshot, solve_battle_diplomacy_at};

/// 外交名单行（非本机、非氛围）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleDiplomacyRow {
    /// house 短名（规则 id）。
    pub house: String,
    /// 显示名（CSF / 回退）。
    pub display_name: String,
    /// 是否与本机同盟。
    pub allied: bool,
}

/// 外交命中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleDiplomacyHit {
    /// 关闭并恢复对局。
    Back,
}

impl BattleDiplomacyHit {
    /// 由入口 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "back" => Some(Self::Back),
            _ => None,
        }
    }

    /// 稳定入口 id。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Back => "back",
        }
    }
}

/// 外交 snapshot。
pub fn diplomacy_snapshot(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    solve_battle_diplomacy_at(viewport_w, viewport_h)
}

/// 窗口像素命中。
pub fn hit_at(viewport_w: u32, viewport_h: u32, x: i32, y: i32) -> Option<BattleDiplomacyHit> {
    let snap = diplomacy_snapshot(viewport_w, viewport_h);
    let hit = snap.hit_test(ra_layout::Point2 { x: x as f32, y: y as f32 })?;
    BattleDiplomacyHit::from_entry_id(hit.id.0.as_str())
}

/// Back 钮矩形。
pub fn button_rects(viewport_w: u32, viewport_h: u32) -> [RectPx; 1] {
    let snap = diplomacy_snapshot(viewport_w, viewport_h);
    [rect_px_from_snapshot(&snap, BATTLE_DIPLOMACY_BUTTON_IDS[0])]
}

/// 名单行槽位数。
pub fn row_slot_count() -> usize {
    BATTLE_DIPLOMACY_ROW_COUNT
}
