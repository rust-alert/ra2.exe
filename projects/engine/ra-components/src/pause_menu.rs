//! 对局内暂停菜单（原版 Esc 菜单）：右栏六钮，左战术区压暗。
//!
//! 本模块只持入口 id、命中与布局入口；壳层导航 / 暂停仿真由宿主另接。

use ra_layout::ui_layout::{PAUSE_MENU_BUTTON_IDS, pause_menu_layout};

pub use ra_layout::ui_layout::{PAUSE_MENU_BUTTON_IDS as BUTTON_IDS, PauseMenuLayout};

/// 暂停菜单命中结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuHit {
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

impl PauseMenuHit {
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
pub fn layout() -> PauseMenuLayout {
    pause_menu_layout(0, 0)
}

/// 在布局上命中。
pub fn hit_at(layout: PauseMenuLayout, x: i32, y: i32) -> Option<PauseMenuHit> {
    PauseMenuHit::from_entry_id(layout.hit_entry_id(x, y)?)
}

/// 入口表（测试 / 诊断用）。
pub fn button_ids() -> &'static [&'static str; 6] {
    &PAUSE_MENU_BUTTON_IDS
}
