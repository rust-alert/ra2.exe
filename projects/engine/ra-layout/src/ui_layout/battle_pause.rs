//! 对局暂停菜单布局。

use super::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePauseMenuLayout {
    /// 合成画布。
    pub canvas: RectPx,
    /// 左侧战术区（压暗罩）。
    pub dim: RectPx,
    /// 右侧栏整体。
    pub sidebar: RectPx,
    /// 六钮：前五连续平铺，末项贴底盖。
    pub buttons: [RectPx; 6],
}

/// 对局暂停菜单布局（右栏几何与壳层主菜单钮格同构）。
///
/// 按钮几何来自 `right_rail_buttons_layout_tree`；侧栏/压暗仍投影自壳层 chrome。
pub fn battle_pause_menu_layout(_viewport_w: u32, _viewport_h: u32) -> BattlePauseMenuLayout {
    let shell = shell_chrome_base_layout();
    let panel_x = shell.panel_top.x;
    let rail = right_rail_buttons(
        "battle_pause",
        &BATTLE_PAUSE_MENU_BUTTON_IDS[..5],
        Some(BATTLE_PAUSE_MENU_BUTTON_IDS[5]),
    );
    let buttons = [rail[0], rail[1], rail[2], rail[3], rail[4], rail[5]];
    BattlePauseMenuLayout {
        canvas: shell.canvas,
        dim: RectPx::new(0, 0, panel_x, SHELL_BASE_H),
        sidebar: RectPx::new(panel_x, 0, RIGHT_PANEL_W, SHELL_BASE_H),
        buttons,
    }
}

impl BattlePauseMenuLayout {
    /// 壳层像素命中入口 id。
    pub fn hit_entry_id(self, x: i32, y: i32) -> Option<&'static str> {
        for (i, id) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().enumerate() {
            if self.buttons[i].contains(x, y) {
                return Some(*id);
            }
        }
        None
    }
}
