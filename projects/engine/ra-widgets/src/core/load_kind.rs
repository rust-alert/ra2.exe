//! 进战斗前装载种类：共用 [`crate::OriginalScreen::LoadScreen`]，外观与回退页按种类区分。

use super::OriginalScreen;

/// 装载页内容与取消回退目标。
///
/// 遭遇战与战役在原版上视觉不同（国家 `ls*` 图 vs 任务简报），但进度条、
/// 取消 / 重试、最短展示与进对局的任务管线相同，因此只保留一个产品页，
/// 用本枚举驱动合成与回退，而不是再开第二个 `OriginalScreen`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadKind {
    /// 遭遇战：国家装载图 + `LOADBRIEF` / 玩家名进度条。
    #[default]
    Skirmish,
    /// 战役：任务简报装载（资源与开局接线后续再接；取消回战役选边）。
    Campaign,
}

impl LoadKind {
    /// 日志 / 标题短名。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skirmish => "skirmish",
            Self::Campaign => "campaign",
        }
    }

    /// 取消或装载失败后的回退产品页。
    pub fn cancel_screen(self) -> OriginalScreen {
        match self {
            Self::Skirmish => OriginalScreen::SkirmishLobby,
            Self::Campaign => OriginalScreen::Campaign,
        }
    }
}
