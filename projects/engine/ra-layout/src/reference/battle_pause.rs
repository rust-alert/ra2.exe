//! 对局暂停菜单：几何权威为 `solve_battle_pause` snapshot。

use crate::{BATTLE_PAUSE_MENU_BUTTON_IDS, reference::shell_chrome::solve_shell_page, snapshot::LayoutSnapshot};

/// 对局暂停菜单：`shell_page_layout_tree` → snapshot。
pub fn solve_battle_pause() -> LayoutSnapshot {
    solve_shell_page("battle_pause", &BATTLE_PAUSE_MENU_BUTTON_IDS[..3], Some(BATTLE_PAUSE_MENU_BUTTON_IDS[3]))
}
