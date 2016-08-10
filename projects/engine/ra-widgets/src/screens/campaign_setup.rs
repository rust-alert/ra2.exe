//! 战役选边：与 `battle.ini` 战役 id、装载阵营的映射。

use super::skirmish_setup::LOBBY_DIFFICULTIES;

/// 选边入口 id（对齐 `CAMPAIGN_SIDE_IDS`）→ `battle.ini` 战役 id。
pub fn campaign_side_battle_id(side: &str) -> Option<&'static str> {
    match side {
        "allied" => Some("ALL1"),
        "tutorial" => Some("TUT1"),
        "soviet" => Some("SOV1"),
        _ => None,
    }
}

/// 选边入口 → 遭遇战装载用的本地 house 名（驱动 `ls*` 图与开局阵营）。
pub fn campaign_side_lobby_house(side: &str) -> Option<&'static str> {
    match side {
        "allied" | "tutorial" => Some("Americans"),
        "soviet" => Some("Russians"),
        _ => None,
    }
}

/// 战役难度滑条档位（0..=2）→ 装载难度标签。
pub fn campaign_difficulty_label(index: u8) -> &'static str {
    LOBBY_DIFFICULTIES[(index as usize) % LOBBY_DIFFICULTIES.len()]
}
