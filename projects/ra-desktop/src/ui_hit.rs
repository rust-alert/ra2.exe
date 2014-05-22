//! 前置菜单逻辑命中：仅命中框，不绘制色块或 SHP。
//!
//! 主菜单 / 单人页 / 遭遇战大厅命中对齐 [`crate::ui_layout`] 像素格（经 fit 相机）；
//! 其余页仍用 [`crate::ui_slots`] 归一化框。

use crate::{
    boot::BootMapCandidate,
    menu_action::MenuAction,
    screen::OriginalScreen,
    ui_layout::{
        LOBBY_MAP_ROW_MAX, MAIN_MENU_BUTTON_IDS, SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_LOBBY_BUTTON_IDS, main_menu_layout,
        single_player_layout, skirmish_lobby_layout, skirmish_map_row_rect, window_to_shell_px,
    },
    ui_slots::slots_for,
};

/// 菜单上的一个可点区域（窗口归一化坐标 0..1）。
#[derive(Debug, Clone, Copy)]
pub struct MenuHit {
    /// 逻辑入口 id。
    pub entry_id: &'static str,
    /// 动作标识。
    pub action: MenuAction,
    /// 左。
    pub x0: f32,
    /// 上。
    pub y0: f32,
    /// 右。
    pub x1: f32,
    /// 下。
    pub y1: f32,
    /// 是否可点。
    pub enabled: bool,
}

/// 为当前页构建命中列表；对局/结算返回空。
///
/// 主菜单列表的归一化框是 **800×600 内容坐标**（非窗口坐标）；点击请走 [`hit_action`]。
/// `load_allow_retry`：加载页「重试」是否可点（装载进行中为 `false`）。
pub fn hits_for(screen: OriginalScreen, maps: &[BootMapCandidate], load_allow_retry: bool) -> Vec<MenuHit> {
    match screen {
        OriginalScreen::MainMenu => hits_main_menu(),
        OriginalScreen::SinglePlayerMenu => hits_single_player(),
        OriginalScreen::SkirmishLobby => hits_skirmish_lobby(maps),
        OriginalScreen::LoadScreen => hits_load_screen(load_allow_retry),
        OriginalScreen::Match | OriginalScreen::Results => Vec::new(),
        other => hits_from_slots(other),
    }
}

/// 窗口像素点击 → 动作。
pub fn hit_action(
    screen: OriginalScreen,
    maps: &[BootMapCandidate],
    _selected: Option<&str>,
    cursor: (f64, f64),
    win_w: f64,
    win_h: f64,
    load_allow_retry: bool,
) -> Option<MenuAction> {
    if screen == OriginalScreen::MainMenu {
        return hit_main_menu_at(cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::SinglePlayerMenu {
        return hit_single_player_at(cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::SkirmishLobby {
        return hit_skirmish_lobby_at(maps, cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    hit_at(&hits_for(screen, maps, load_allow_retry), cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action)
}

/// 命中命中区下标（仅 `enabled`）。
pub fn hover_index(
    screen: OriginalScreen,
    maps: &[BootMapCandidate],
    _selected: Option<&str>,
    cursor: (f64, f64),
    win_w: f64,
    win_h: f64,
    load_allow_retry: bool,
) -> Option<usize> {
    if screen == OriginalScreen::MainMenu {
        return hit_main_menu_at(cursor.0, cursor.1, win_w, win_h).map(|(i, _)| i);
    }
    if screen == OriginalScreen::SinglePlayerMenu {
        return hit_single_player_at(cursor.0, cursor.1, win_w, win_h).map(|(i, _)| i);
    }
    if screen == OriginalScreen::SkirmishLobby {
        return hit_skirmish_lobby_at(maps, cursor.0, cursor.1, win_w, win_h).map(|(i, _)| i);
    }
    hit_at(&hits_for(screen, maps, load_allow_retry), cursor.0, cursor.1, win_w, win_h).map(|(i, _)| i)
}

fn hit_at(hits: &[MenuHit], cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let nx = (cursor_x / win_w) as f32;
    let ny = (cursor_y / win_h) as f32;
    for (i, h) in hits.iter().enumerate() {
        if !h.enabled {
            continue;
        }
        if nx >= h.x0 && nx <= h.x1 && ny >= h.y0 && ny <= h.y1 {
            return Some((i, h.action));
        }
    }
    None
}

fn hits_main_menu() -> Vec<MenuHit> {
    let Some(page) = slots_for(OriginalScreen::MainMenu)
    else {
        return Vec::new();
    };
    let layout = main_menu_layout(0, 0);
    let bw = layout.canvas.w as f32;
    let bh = layout.canvas.h as f32;
    MAIN_MENU_BUTTON_IDS
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let btn = page.buttons.iter().find(|b| b.entry_id == *id)?;
            let cell = layout.buttons[i];
            Some(MenuHit {
                entry_id: btn.entry_id,
                action: btn.action,
                x0: cell.x as f32 / bw,
                y0: cell.y as f32 / bh,
                x1: (cell.x + cell.w) as f32 / bw,
                y1: (cell.y + cell.h) as f32 / bh,
                enabled: btn.enabled,
            })
        })
        .collect()
}

fn hit_main_menu_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = main_menu_layout(0, 0);
    let Some(page) = slots_for(OriginalScreen::MainMenu)
    else {
        return None;
    };
    for (i, id) in MAIN_MENU_BUTTON_IDS.iter().enumerate() {
        let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
        else {
            continue;
        };
        if !btn.enabled {
            continue;
        }
        if layout.buttons[i].contains(sx, sy) {
            return Some((i, btn.action));
        }
    }
    None
}

fn hits_single_player() -> Vec<MenuHit> {
    let Some(page) = slots_for(OriginalScreen::SinglePlayerMenu)
    else {
        return Vec::new();
    };
    let layout = single_player_layout(0, 0);
    let bw = layout.canvas.w as f32;
    let bh = layout.canvas.h as f32;
    SINGLE_PLAYER_BUTTON_IDS
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let btn = page.buttons.iter().find(|b| b.entry_id == *id)?;
            let cell = layout.buttons[i];
            Some(MenuHit {
                entry_id: btn.entry_id,
                action: btn.action,
                x0: cell.x as f32 / bw,
                y0: cell.y as f32 / bh,
                x1: (cell.x + cell.w) as f32 / bw,
                y1: (cell.y + cell.h) as f32 / bh,
                enabled: btn.enabled,
            })
        })
        .collect()
}

fn hit_single_player_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = single_player_layout(0, 0);
    let Some(page) = slots_for(OriginalScreen::SinglePlayerMenu)
    else {
        return None;
    };
    for (i, id) in SINGLE_PLAYER_BUTTON_IDS.iter().enumerate() {
        let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
        else {
            continue;
        };
        if !btn.enabled {
            continue;
        }
        if layout.buttons[i].contains(sx, sy) {
            return Some((i, btn.action));
        }
    }
    None
}

fn hits_from_slots(screen: OriginalScreen) -> Vec<MenuHit> {
    let Some(page) = slots_for(screen)
    else {
        return Vec::new();
    };
    page.buttons
        .iter()
        .map(|btn| {
            let (x0, y0, x1, y1) = btn.hit;
            MenuHit { entry_id: btn.entry_id, action: btn.action, x0, y0, x1, y1, enabled: btn.enabled }
        })
        .collect()
}

fn hits_load_screen(allow_retry: bool) -> Vec<MenuHit> {
    let Some(page) = slots_for(OriginalScreen::LoadScreen)
    else {
        return Vec::new();
    };
    page.buttons
        .iter()
        .map(|btn| {
            let (x0, y0, x1, y1) = btn.hit;
            let enabled = if btn.entry_id == "retry" { allow_retry } else { btn.enabled };
            MenuHit { entry_id: btn.entry_id, action: btn.action, x0, y0, x1, y1, enabled }
        })
        .collect()
}

fn hits_skirmish_lobby(maps: &[BootMapCandidate]) -> Vec<MenuHit> {
    let layout = skirmish_lobby_layout(0, 0);
    let bw = layout.canvas.w as f32;
    let bh = layout.canvas.h as f32;
    let mut hits = Vec::new();
    let n = maps.len().min(LOBBY_MAP_ROW_MAX as usize);
    for i in 0..n {
        let row = skirmish_map_row_rect(&layout, i);
        hits.push(MenuHit {
            entry_id: "map",
            action: MenuAction::SelectMap(i),
            x0: row.x as f32 / bw,
            y0: row.y as f32 / bh,
            x1: (row.x + row.w) as f32 / bw,
            y1: (row.y + row.h) as f32 / bh,
            enabled: true,
        });
    }
    if let Some(page) = slots_for(OriginalScreen::SkirmishLobby) {
        for (i, id) in SKIRMISH_LOBBY_BUTTON_IDS.iter().enumerate() {
            let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
            else {
                continue;
            };
            let cell = layout.buttons[i];
            hits.push(MenuHit {
                entry_id: btn.entry_id,
                action: btn.action,
                x0: cell.x as f32 / bw,
                y0: cell.y as f32 / bh,
                x1: (cell.x + cell.w) as f32 / bw,
                y1: (cell.y + cell.h) as f32 / bh,
                enabled: btn.enabled,
            });
        }
    }
    hits
}

fn hit_skirmish_lobby_at(
    maps: &[BootMapCandidate],
    cursor_x: f64,
    cursor_y: f64,
    win_w: f64,
    win_h: f64,
) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = skirmish_lobby_layout(0, 0);
    let n = maps.len().min(LOBBY_MAP_ROW_MAX as usize);
    for i in 0..n {
        if skirmish_map_row_rect(&layout, i).contains(sx, sy) {
            return Some((i, MenuAction::SelectMap(i)));
        }
    }
    let page = slots_for(OriginalScreen::SkirmishLobby)?;
    for (i, id) in SKIRMISH_LOBBY_BUTTON_IDS.iter().enumerate() {
        let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
        else {
            continue;
        };
        if !btn.enabled {
            continue;
        }
        if layout.buttons[i].contains(sx, sy) {
            return Some((n + i, btn.action));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_map::Theater;

    #[test]
    fn main_menu_hit_single_player() {
        // 右侧首钮格中心：壳层约 (722, 220) → 1024×768 fit 后约 (924, 282)
        let action = hit_action(OriginalScreen::MainMenu, &[], None, (924.0, 282.0), 1024.0, 768.0, false);
        assert_eq!(action, Some(MenuAction::OpenSinglePlayer));
    }

    #[test]
    fn load_screen_disables_retry_while_loading() {
        let loading = hits_for(OriginalScreen::LoadScreen, &[], false);
        let retry = loading.iter().find(|h| h.entry_id == "retry").expect("retry");
        assert!(!retry.enabled);
        let failed = hits_for(OriginalScreen::LoadScreen, &[], true);
        let retry = failed.iter().find(|h| h.entry_id == "retry").expect("retry");
        assert!(retry.enabled);
    }

    #[test]
    fn disabled_network_not_hit() {
        // 网络钮格中心约壳层 (722, 262) → 窗口约 (924, 335)，禁用。
        let action = hit_action(OriginalScreen::MainMenu, &[], None, (924.0, 335.0), 1024.0, 768.0, false);
        assert_eq!(action, None);
    }

    #[test]
    fn lobby_map_row_is_selectable() {
        let maps =
            vec![BootMapCandidate { file_name: "mp03t4.map".into(), width: 50, height: 50, theater: Theater::Temperate }];
        let layout = skirmish_lobby_layout(0, 0);
        let row = skirmish_map_row_rect(&layout, 0);
        let cx = row.x + row.w / 2;
        let cy = row.y + row.h / 2;
        let cam = crate::ui_layout::shell_fit_camera(1024, 768);
        let sx = (cx as f32 - cam.center_x) * cam.zoom + 1024.0 * 0.5;
        let sy = (cy as f32 - cam.center_y) * cam.zoom + 768.0 * 0.5;
        let action =
            hit_action(OriginalScreen::SkirmishLobby, &maps, Some("mp03t4.map"), (sx as f64, sy as f64), 1024.0, 768.0, false);
        assert_eq!(action, Some(MenuAction::SelectMap(0)));
    }

    #[test]
    fn hover_index_tracks_enabled_button() {
        assert_eq!(hover_index(OriginalScreen::MainMenu, &[], None, (924.0, 282.0), 1024.0, 768.0, false,), Some(0));
        assert_eq!(hover_index(OriginalScreen::MainMenu, &[], None, (924.0, 335.0), 1024.0, 768.0, false,), None);
    }
}
