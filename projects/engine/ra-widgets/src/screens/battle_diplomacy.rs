//! 对局外交子页（暂停层；复用 pause hub / `bkgd*` / 底栏，内容区画花名册）。

use ra_layout::{BattleHudChromeMetrics, LayoutSnapshot, RectPx};

use crate::battle_pause_menu::{button_rects_with_metrics, pause_snapshot_with_metrics};

/// 外交名单行（含本机；跳过氛围 house）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleDiplomacyRow {
    /// 显示名（本机名 / 「电脑」等）。
    pub display_name: String,
    /// 玩家色 RGB。
    pub color_rgb: [u8; 3],
    /// 队伍号（`0` = 无队）。
    pub team: u8,
    /// 击毁数。
    pub kills: u32,
}

/// 外交命中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleDiplomacyHit {
    /// 关闭并恢复对局（侧栏「继续」= 暂停 `resume` 槽）。
    Continue,
}

impl BattleDiplomacyHit {
    /// 由入口 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "continue" | "resume" | "back" => Some(Self::Continue),
            _ => None,
        }
    }

    /// 稳定入口 id。
    pub fn entry_id(self) -> &'static str {
        "continue"
    }
}

/// 外交 snapshot（与暂停同树，侧栏钮用 `resume` 槽）。
pub fn diplomacy_snapshot(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    diplomacy_snapshot_with_metrics(viewport_w, viewport_h, BattleHudChromeMetrics::sidec01())
}

/// 可注入侧栏度量的 snapshot。
pub fn diplomacy_snapshot_with_metrics(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> LayoutSnapshot {
    pause_snapshot_with_metrics(viewport_w, viewport_h, metrics)
}

/// 窗口像素命中（仅侧栏最底「继续」）。
pub fn hit_at(viewport_w: u32, viewport_h: u32, x: i32, y: i32) -> Option<BattleDiplomacyHit> {
    hit_at_with_metrics(viewport_w, viewport_h, BattleHudChromeMetrics::sidec01(), x, y)
}

/// 可注入侧栏度量的命中。
pub fn hit_at_with_metrics(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics, x: i32, y: i32) -> Option<BattleDiplomacyHit> {
    let cell = button_rects_with_metrics(viewport_w, viewport_h, metrics).last().copied()?;
    if x >= cell.x && x < cell.x + cell.w && y >= cell.y && y < cell.y + cell.h { Some(BattleDiplomacyHit::Continue) } else { None }
}

/// 「继续」钮矩形（暂停 `resume` 槽）。
pub fn button_rects(viewport_w: u32, viewport_h: u32) -> [RectPx; 1] {
    button_rects_with_metrics_arr(viewport_w, viewport_h, BattleHudChromeMetrics::sidec01())
}

fn button_rects_with_metrics_arr(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> [RectPx; 1] {
    let rects = button_rects_with_metrics(viewport_w, viewport_h, metrics);
    [rects.last().copied().unwrap_or(RectPx::new(0, 0, 0, 0))]
}

/// 名单最大行数。
pub fn row_slot_count() -> usize {
    8
}
