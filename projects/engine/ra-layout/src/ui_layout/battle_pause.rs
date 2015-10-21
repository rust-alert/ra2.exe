//! 对局暂停菜单：几何权威为 `solve_battle_pause` snapshot。

use crate::{solve_shell_page, LayoutSnapshot, BATTLE_PAUSE_MENU_BUTTON_IDS};

/// 对局暂停菜单：`shell_page_layout_tree` → snapshot。
pub fn solve_battle_pause() -> LayoutSnapshot {
    solve_shell_page(
        "battle_pause",
        &BATTLE_PAUSE_MENU_BUTTON_IDS[..5],
        Some(BATTLE_PAUSE_MENU_BUTTON_IDS[5]),
    )
}
