//! 前置菜单逻辑命中：仅命中框，不绘制色块或 SHP。
//!
//! 主菜单 / 单人页 / 遭遇战大厅命中对齐 [`ra_layout::ui_layout`] 像素格（经 fit 相机）；
//! 其余页仍用 [`crate::ui_slots`] 归一化框。

use crate::{menu_action::MenuAction, original_screen::OriginalScreen, ui_slots::slots_for};
use ra_layout::{
    CAMPAIGN_BUTTON_IDS, CAMPAIGN_SIDE_IDS, CHOOSE_MAP_BUTTON_IDS, EXIT_CONFIRM_BUTTON_IDS, MAIN_MENU_BUTTON_IDS, OPTIONS_BUTTON_IDS,
    SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_LOBBY_BUTTON_IDS, campaign_layout, choose_map_layout, exit_confirm_layout, main_menu_layout,
    options_layout, single_player_layout, skirmish_lobby_layout, window_to_shell_px,
};
use ra_map::BootMapCandidate;

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
        OriginalScreen::Campaign => hits_campaign(),
        OriginalScreen::SkirmishLobby => hits_skirmish_lobby(maps),
        OriginalScreen::ChooseMap => hits_choose_map(maps),
        OriginalScreen::Options => hits_options(),
        OriginalScreen::ExitConfirm => hits_exit_confirm(),
        OriginalScreen::LoadScreen => hits_load_screen(load_allow_retry),
        OriginalScreen::Battle | OriginalScreen::Results => Vec::new(),
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
    if screen == OriginalScreen::Campaign {
        return hit_campaign_at(cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::Options {
        return hit_options_at(cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::SkirmishLobby {
        return hit_skirmish_lobby_at(maps, cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::ChooseMap {
        return hit_choose_map_at(maps, cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::ExitConfirm {
        return hit_exit_confirm_at(cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    hit_at(&hits_for(screen, maps, load_allow_retry), cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action)
}

/// 命中命中区下标。主菜单 / 单人页包含禁用项（用于悬停帧与底栏提示）；
/// 点击仍走 [`hit_action`]（仅 `enabled`）。
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
        return hover_main_menu_at(cursor.0, cursor.1, win_w, win_h);
    }
    if screen == OriginalScreen::SinglePlayerMenu {
        return hover_single_player_at(cursor.0, cursor.1, win_w, win_h);
    }
    if screen == OriginalScreen::Campaign {
        return hover_campaign_at(cursor.0, cursor.1, win_w, win_h);
    }
    if screen == OriginalScreen::Options {
        return hover_options_at(cursor.0, cursor.1, win_w, win_h);
    }
    if screen == OriginalScreen::SkirmishLobby {
        return hit_skirmish_lobby_at(maps, cursor.0, cursor.1, win_w, win_h).map(|(i, _)| i);
    }
    if screen == OriginalScreen::ChooseMap {
        return hit_choose_map_at(maps, cursor.0, cursor.1, win_w, win_h).map(|(i, _)| i);
    }
    if screen == OriginalScreen::ExitConfirm {
        return hover_exit_confirm_at(cursor.0, cursor.1, win_w, win_h);
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

/// 悬停：含禁用钮（底栏提示仍可显示）。
fn hover_main_menu_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = main_menu_layout(0, 0);
    for (i, _) in MAIN_MENU_BUTTON_IDS.iter().enumerate() {
        if layout.buttons[i].contains(sx, sy) {
            return Some(i);
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

/// 悬停：含禁用钮。
fn hover_single_player_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = single_player_layout(0, 0);
    for (i, _) in SINGLE_PLAYER_BUTTON_IDS.iter().enumerate() {
        if layout.buttons[i].contains(sx, sy) {
            return Some(i);
        }
    }
    None
}

/// 战役页悬停入口 id（三侧 / 难度轨 / 右栏钮）。
pub fn campaign_entry_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<&'static str> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = campaign_layout(0, 0);
    let sides = [(CAMPAIGN_SIDE_IDS[0], layout.allied), (CAMPAIGN_SIDE_IDS[1], layout.tutorial), (CAMPAIGN_SIDE_IDS[2], layout.soviet)];
    for (id, rect) in sides {
        if rect.contains(sx, sy) {
            return Some(id);
        }
    }
    if layout.difficulty_track.contains(sx, sy) || layout.difficulty_label.contains(sx, sy) || layout.difficulty_value.contains(sx, sy) {
        return Some("difficulty");
    }
    for (i, id) in CAMPAIGN_BUTTON_IDS.iter().enumerate() {
        if layout.shell.buttons[i].contains(sx, sy) {
            return Some(*id);
        }
    }
    None
}

fn hits_campaign() -> Vec<MenuHit> {
    let layout = campaign_layout(0, 0);
    let bw = layout.shell.canvas.w as f32;
    let bh = layout.shell.canvas.h as f32;
    let mut out = Vec::new();
    let sides = [
        (CAMPAIGN_SIDE_IDS[0], MenuAction::SelectCampaignAllied, layout.allied),
        (CAMPAIGN_SIDE_IDS[1], MenuAction::SelectCampaignTutorial, layout.tutorial),
        (CAMPAIGN_SIDE_IDS[2], MenuAction::SelectCampaignSoviet, layout.soviet),
    ];
    for (id, action, cell) in sides {
        out.push(MenuHit {
            entry_id: id,
            action,
            x0: cell.x as f32 / bw,
            y0: cell.y as f32 / bh,
            x1: (cell.x + cell.w) as f32 / bw,
            y1: (cell.y + cell.h) as f32 / bh,
            enabled: true,
        });
    }
    let track = layout.difficulty_track;
    out.push(MenuHit {
        entry_id: "difficulty",
        action: MenuAction::CycleCampaignDifficulty,
        x0: track.x as f32 / bw,
        y0: track.y as f32 / bh,
        x1: (track.x + track.w) as f32 / bw,
        y1: (track.y + track.h) as f32 / bh,
        enabled: true,
    });
    if let Some(page) = slots_for(OriginalScreen::Campaign) {
        for (i, id) in CAMPAIGN_BUTTON_IDS.iter().enumerate() {
            let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
            else {
                continue;
            };
            let cell = layout.shell.buttons[i];
            out.push(MenuHit {
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
    out
}

fn hit_campaign_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    let hits = hits_campaign();
    hit_at(&hits, cursor_x, cursor_y, win_w, win_h)
}

fn hover_campaign_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = campaign_layout(0, 0);
    let rects = [layout.allied, layout.tutorial, layout.soviet, layout.difficulty_track, layout.shell.buttons[0], layout.shell.buttons[1]];
    for (i, rect) in rects.iter().enumerate() {
        if rect.contains(sx, sy) {
            return Some(i);
        }
    }
    None
}

fn hits_options() -> Vec<MenuHit> {
    let Some(page) = slots_for(OriginalScreen::Options)
    else {
        return Vec::new();
    };
    let layout = options_layout(0, 0);
    let bw = layout.canvas.w as f32;
    let bh = layout.canvas.h as f32;
    OPTIONS_BUTTON_IDS
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

fn hit_options_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = options_layout(0, 0);
    let Some(page) = slots_for(OriginalScreen::Options)
    else {
        return None;
    };
    for (i, id) in OPTIONS_BUTTON_IDS.iter().enumerate() {
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

/// 悬停：含禁用钮。
fn hover_options_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = options_layout(0, 0);
    for (i, _) in OPTIONS_BUTTON_IDS.iter().enumerate() {
        if layout.buttons[i].contains(sx, sy) {
            return Some(i);
        }
    }
    None
}

fn hits_exit_confirm() -> Vec<MenuHit> {
    let Some(page) = slots_for(OriginalScreen::ExitConfirm)
    else {
        return Vec::new();
    };
    let shell = main_menu_layout(0, 0);
    let dlg = exit_confirm_layout(0, 0);
    let bw = shell.canvas.w as f32;
    let bh = shell.canvas.h as f32;
    EXIT_CONFIRM_BUTTON_IDS
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let btn = page.buttons.iter().find(|b| b.entry_id == *id)?;
            let cell = dlg.buttons[i];
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

fn hit_exit_confirm_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let dlg = exit_confirm_layout(0, 0);
    let Some(page) = slots_for(OriginalScreen::ExitConfirm)
    else {
        return None;
    };
    for (i, id) in EXIT_CONFIRM_BUTTON_IDS.iter().enumerate() {
        let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
        else {
            continue;
        };
        if !btn.enabled {
            continue;
        }
        if dlg.buttons[i].contains(sx, sy) {
            return Some((i, btn.action));
        }
    }
    None
}

fn hover_exit_confirm_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let dlg = exit_confirm_layout(0, 0);
    for (i, _) in EXIT_CONFIRM_BUTTON_IDS.iter().enumerate() {
        if dlg.buttons[i].contains(sx, sy) {
            return Some(i);
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
    // 装载中不提供鼠标钮（Esc 仍取消）；失败后才露出重试/取消。
    if !allow_retry {
        return Vec::new();
    }
    let Some(page) = slots_for(OriginalScreen::LoadScreen)
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

fn hits_skirmish_lobby(_maps: &[BootMapCandidate]) -> Vec<MenuHit> {
    let layout = skirmish_lobby_layout(0, 0);
    let bw = layout.shell.canvas.w as f32;
    let bh = layout.shell.canvas.h as f32;
    let mut hits = Vec::new();
    // 地图列表不在本页左侧；选图走右栏 `choose_map`（完整模态后续接）。
    if let Some(page) = slots_for(OriginalScreen::SkirmishLobby) {
        for (i, id) in SKIRMISH_LOBBY_BUTTON_IDS.iter().enumerate() {
            let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
            else {
                continue;
            };
            let cell = layout.shell.buttons[i];
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

fn hit_skirmish_lobby_at(_maps: &[BootMapCandidate], cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = skirmish_lobby_layout(0, 0);
    let page = slots_for(OriginalScreen::SkirmishLobby)?;
    for (i, id) in SKIRMISH_LOBBY_BUTTON_IDS.iter().enumerate() {
        let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
        else {
            continue;
        };
        if !btn.enabled {
            continue;
        }
        if layout.shell.buttons[i].contains(sx, sy) {
            return Some((i, btn.action));
        }
    }
    None
}

const CHOOSE_MAP_LIST_ROW_H: i32 = 16;

fn hits_choose_map(maps: &[BootMapCandidate]) -> Vec<MenuHit> {
    let layout = choose_map_layout(0, 0);
    let bw = layout.shell.canvas.w as f32;
    let bh = layout.shell.canvas.h as f32;
    let mut hits = Vec::new();
    if let Some(page) = slots_for(OriginalScreen::ChooseMap) {
        for (i, id) in CHOOSE_MAP_BUTTON_IDS.iter().enumerate() {
            let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
            else {
                continue;
            };
            let cell = layout.shell.buttons[i];
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
    let visible = (layout.map_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (i, _) in maps.iter().take(visible).enumerate() {
        let row_y = layout.map_list.y + (i as i32) * CHOOSE_MAP_LIST_ROW_H;
        hits.push(MenuHit {
            entry_id: "map_row",
            action: MenuAction::SelectMap(i),
            x0: layout.map_list.x as f32 / bw,
            y0: row_y as f32 / bh,
            x1: (layout.map_list.x + layout.map_list.w) as f32 / bw,
            y1: (row_y + CHOOSE_MAP_LIST_ROW_H) as f32 / bh,
            enabled: true,
        });
    }
    hits
}

fn hit_choose_map_at(maps: &[BootMapCandidate], cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    let layout = choose_map_layout(0, 0);
    let page = slots_for(OriginalScreen::ChooseMap)?;
    for (i, id) in CHOOSE_MAP_BUTTON_IDS.iter().enumerate() {
        let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id)
        else {
            continue;
        };
        if !btn.enabled {
            continue;
        }
        if layout.shell.buttons[i].contains(sx, sy) {
            return Some((i, btn.action));
        }
    }
    if layout.map_list.contains(sx, sy) {
        let row = ((sy - layout.map_list.y) / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
        let visible = (layout.map_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
        if row < maps.len().min(visible) {
            // 按钮之后的列表下标，供悬停映射用。
            return Some((CHOOSE_MAP_BUTTON_IDS.len() + row, MenuAction::SelectMap(row)));
        }
    }
    None
}
