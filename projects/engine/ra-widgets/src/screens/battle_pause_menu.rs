//! 对局内暂停菜单（原版 Esc 菜单）：右栏六钮，左战术区压暗。
//!
//! 本模块只持入口 id、命中与布局入口；壳层导航 / 暂停仿真由宿主另接。

use ra_layout::{
    battle_pause_menu_layout, solve_battle_pause, LayoutSnapshot, Point2, BATTLE_PAUSE_MENU_BUTTON_IDS,
};

pub use ra_layout::ui_layout::{BATTLE_PAUSE_MENU_BUTTON_IDS as BUTTON_IDS, BattlePauseMenuLayout};

/// 暂停菜单命中结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePauseMenuHit {
    /// 打开选项。
    Options,
    /// 载入游戏。
    Load,
    /// 保存游戏。
    Save,
    /// 重新开始。
    Restart,
    /// 放弃任务。
    Abort,
    /// 回到游戏。
    Resume,
}

impl BattlePauseMenuHit {
    /// 由入口 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "options" => Some(Self::Options),
            "load" => Some(Self::Load),
            "save" => Some(Self::Save),
            "restart" => Some(Self::Restart),
            "abort" => Some(Self::Abort),
            "resume" => Some(Self::Resume),
            _ => None,
        }
    }

    /// 稳定入口 id（与 [`BUTTON_IDS`] 一致）。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Options => "options",
            Self::Load => "load",
            Self::Save => "save",
            Self::Restart => "restart",
            Self::Abort => "abort",
            Self::Resume => "resume",
        }
    }
}

/// 构造 800×600 内容坐标下的暂停菜单布局。
pub fn layout() -> BattlePauseMenuLayout {
    battle_pause_menu_layout(0, 0)
}

fn battle_pause_snapshot() -> LayoutSnapshot {
    solve_battle_pause()
}

/// 在布局上命中（几何权威为 `shell_page_layout_tree` snapshot；`layout` 仅保留 API 兼容）。
pub fn hit_at(_layout: BattlePauseMenuLayout, x: i32, y: i32) -> Option<BattlePauseMenuHit> {
    let snap = battle_pause_snapshot();
    let point = Point2 {
        x: x as f32,
        y: y as f32,
    };
    for id in BATTLE_PAUSE_MENU_BUTTON_IDS {
        if snap
            .get(id)
            .is_some_and(|el| el.layout.rect.contains(point))
        {
            return BattlePauseMenuHit::from_entry_id(id);
        }
    }
    None
}

/// 入口表（测试 / 诊断用）。
pub fn button_ids() -> &'static [&'static str; 6] {
    &BATTLE_PAUSE_MENU_BUTTON_IDS
}
