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
pub fn battle_pause_menu_layout(viewport_w: u32, viewport_h: u32) -> BattlePauseMenuLayout {
    let _ = (viewport_w, viewport_h);
    let shell = main_menu_layout(0, 0);
    let panel_x = shell.panel_top.x;
    let tile_y = shell.panel_tile.y;
    let resume_y = shell.panel_bottom.y - BUTTON_CELL_H;
    let buttons = [
        button_cell(panel_x, tile_y),
        button_cell(panel_x, tile_y + BUTTON_CELL_H),
        button_cell(panel_x, tile_y + 2 * BUTTON_CELL_H),
        button_cell(panel_x, tile_y + 3 * BUTTON_CELL_H),
        button_cell(panel_x, tile_y + 4 * BUTTON_CELL_H),
        button_cell(panel_x, resume_y),
    ];
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
