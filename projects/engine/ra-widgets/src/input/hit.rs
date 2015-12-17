//! 前置菜单逻辑命中：仅命中框，不绘制色块或 SHP。
//!
//! 点击 / 悬停一律经 `window_to_shell_px` + 各页 `solve_*` snapshot。
//! [`hits_for`] 仍产出壳层像素 [`MenuHit`] 列表，供诊断与测试枚举入口。
//!
//! 选图 / 遭遇战大厅几何来自 `solve_choose_map` / `solve_skirmish_lobby`；
//! 主菜单 / 单人 / 选项右栏页来自 `solve_shell_page`；
//! 装载 / 战役 / 退出确认 / 网络占位来自对应 `solve_*`；闪屏与对局无菜单命中。

use crate::{menu_action::MenuAction, original_screen::OriginalScreen, skin::slots::slots_for};
use ra_layout::{
    CAMPAIGN_BUTTON_IDS, CAMPAIGN_SIDE_IDS, CHOOSE_MAP_BUTTON_IDS, CHOOSE_MAP_LIST_ROW_H,
    EXIT_CONFIRM_BUTTON_IDS, LOAD_SCREEN_BUTTON_IDS, MAIN_MENU_BUTTON_IDS, NETWORK_BUTTON_IDS,
    OPTIONS_BUTTON_IDS, SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_LOBBY_BUTTON_IDS, LayoutSnapshot, Point2,
    Rect, RectPx, choose_map_list_row_rect, choose_map_visible_rows, clamp_map_list_scroll,
    solve_campaign, solve_choose_map, solve_exit_confirm, solve_load_screen, solve_network_page,
    solve_options_page, solve_shell_page, solve_skirmish_lobby, window_to_shell_px,
};
use ra_map::BootMapCandidate;

/// 菜单上的一个可点区域（壳层设计像素，非窗口归一化）。
#[derive(Debug, Clone, Copy)]
pub struct MenuHit {
    /// 逻辑入口 id。
    pub entry_id: &'static str,
    /// 动作标识。
    pub action: MenuAction,
    /// 壳层像素矩形。
    pub rect: RectPx,
    /// 是否可点。
    pub enabled: bool,
}

/// 为当前页构建命中列表；对局/结算返回空。
///
/// 列表矩形为壳层 800×600 内容坐标；点击请走 [`hit_action`]。
/// `mode_count`：选图页游戏类型行数（其它页忽略）。
/// `map_list_scroll`：选图页地图列表滚动偏移（其它页忽略）。
/// `load_allow_retry`：加载页「重试」是否可点（装载进行中为 `false`）。
pub fn hits_for(
    screen: OriginalScreen,
    maps: &[BootMapCandidate],
    mode_count: usize,
    map_list_scroll: usize,
    load_allow_retry: bool,
) -> Vec<MenuHit> {
    match screen {
        OriginalScreen::MainMenu => hits_main_menu(),
        OriginalScreen::SinglePlayerMenu => hits_single_player(),
        OriginalScreen::Campaign => hits_campaign(),
        OriginalScreen::SkirmishLobby => hits_skirmish_lobby(maps),
        OriginalScreen::ChooseMap => hits_choose_map(maps, mode_count, map_list_scroll),
        OriginalScreen::Options => hits_options(),
        OriginalScreen::ExitConfirm => hits_exit_confirm(),
        OriginalScreen::LoadScreen => hits_load_screen(load_allow_retry),
        OriginalScreen::Network => hits_network(),
        OriginalScreen::Splash | OriginalScreen::Battle | OriginalScreen::Results => Vec::new(),
    }
}

/// 窗口像素点击 → 动作。
pub fn hit_action(
    screen: OriginalScreen,
    maps: &[BootMapCandidate],
    mode_count: usize,
    _selected: Option<&str>,
    cursor: (f64, f64),
    win_w: f64,
    win_h: f64,
    map_list_scroll: usize,
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
        return hit_choose_map_at(maps, mode_count, map_list_scroll, cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::ExitConfirm {
        return hit_exit_confirm_at(cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    if screen == OriginalScreen::LoadScreen {
        return hit_load_screen_at(cursor.0, cursor.1, win_w, win_h, load_allow_retry)
            .map(|(_, action)| action);
    }
    if screen == OriginalScreen::Network {
        return hit_network_at(cursor.0, cursor.1, win_w, win_h).map(|(_, action)| action);
    }
    // 闪屏 / 对局 / 结算无前置菜单点击命中。
    None
}

/// 命中命中区下标。主菜单 / 单人页包含禁用项（用于悬停帧与底栏提示）；
/// 点击仍走 [`hit_action`]（仅 `enabled`）。
pub fn hover_index(
    screen: OriginalScreen,
    maps: &[BootMapCandidate],
    mode_count: usize,
    _selected: Option<&str>,
    cursor: (f64, f64),
    win_w: f64,
    win_h: f64,
    map_list_scroll: usize,
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
        return hit_choose_map_at(maps, mode_count, map_list_scroll, cursor.0, cursor.1, win_w, win_h).map(|(i, _)| i);
    }
    if screen == OriginalScreen::ExitConfirm {
        return hover_exit_confirm_at(cursor.0, cursor.1, win_w, win_h);
    }
    if screen == OriginalScreen::LoadScreen {
        return hit_load_screen_at(cursor.0, cursor.1, win_w, win_h, load_allow_retry).map(|(i, _)| i);
    }
    if screen == OriginalScreen::Network {
        return hover_network_at(cursor.0, cursor.1, win_w, win_h);
    }
    // 闪屏 / 对局 / 结算无前置菜单悬停命中。
    None
}

fn shell_point(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<Point2> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let (sx, sy) = window_to_shell_px(cursor_x, cursor_y, win_w, win_h);
    Some(Point2 {
        x: sx as f32,
        y: sy as f32,
    })
}

fn hits_from_slot_ids(
    screen: OriginalScreen,
    snap: &LayoutSnapshot,
    ids: &[&'static str],
) -> Vec<MenuHit> {
    let Some(page) = slots_for(screen) else {
        return Vec::new();
    };
    ids.iter()
        .filter_map(|id| {
            let btn = page.buttons.iter().find(|b| b.entry_id == *id)?;
            let el = snap.get(id)?;
            Some(menu_hit_from_rect(
                btn.entry_id,
                btn.action,
                el.layout.rect,
                btn.enabled,
            ))
        })
        .collect()
}

fn hit_enabled_slot_at(
    screen: OriginalScreen,
    snap: &LayoutSnapshot,
    ids: &[&'static str],
    point: Point2,
) -> Option<(usize, MenuAction)> {
    let page = slots_for(screen)?;
    for (i, id) in ids.iter().enumerate() {
        let Some(btn) = page.buttons.iter().find(|b| b.entry_id == *id) else {
            continue;
        };
        if !btn.enabled {
            continue;
        }
        let Some(el) = snap.get(id) else {
            continue;
        };
        if el.layout.rect.contains(point) {
            return Some((i, btn.action));
        }
    }
    None
}

fn hover_ids_at(snap: &LayoutSnapshot, ids: &[&'static str], point: Point2) -> Option<usize> {
    for (i, id) in ids.iter().enumerate() {
        if snap.get(id).is_some_and(|el| el.layout.rect.contains(point)) {
            return Some(i);
        }
    }
    None
}

fn hits_main_menu() -> Vec<MenuHit> {
    hits_from_slot_ids(
        OriginalScreen::MainMenu,
        &main_menu_snapshot(),
        &MAIN_MENU_BUTTON_IDS,
    )
}

fn hit_main_menu_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hit_enabled_slot_at(
        OriginalScreen::MainMenu,
        &main_menu_snapshot(),
        &MAIN_MENU_BUTTON_IDS,
        point,
    )
}

/// 悬停：含禁用钮（底栏提示仍可显示）。
fn hover_main_menu_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hover_ids_at(&main_menu_snapshot(), &MAIN_MENU_BUTTON_IDS, point)
}

fn main_menu_snapshot() -> LayoutSnapshot {
    solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    )
}

fn hits_single_player() -> Vec<MenuHit> {
    hits_from_slot_ids(
        OriginalScreen::SinglePlayerMenu,
        &single_player_snapshot(),
        &SINGLE_PLAYER_BUTTON_IDS,
    )
}

fn hit_single_player_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hit_enabled_slot_at(
        OriginalScreen::SinglePlayerMenu,
        &single_player_snapshot(),
        &SINGLE_PLAYER_BUTTON_IDS,
        point,
    )
}

/// 悬停：含禁用钮。
fn hover_single_player_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hover_ids_at(&single_player_snapshot(), &SINGLE_PLAYER_BUTTON_IDS, point)
}

fn single_player_snapshot() -> LayoutSnapshot {
    solve_shell_page(
        "single_player",
        &SINGLE_PLAYER_BUTTON_IDS[..3],
        Some(SINGLE_PLAYER_BUTTON_IDS[3]),
    )
}

/// 战役页悬停入口 id（三侧 / 难度轨 / 右栏钮）。
pub fn campaign_entry_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<&'static str> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    let snap = campaign_snapshot();
    for id in CAMPAIGN_SIDE_IDS {
        if snap.get(id).is_some_and(|el| el.layout.rect.contains(point)) {
            return Some(id);
        }
    }
    if snap
        .get("difficulty")
        .is_some_and(|el| el.layout.rect.contains(point))
        || snap
            .get("difficulty_label")
            .is_some_and(|el| el.layout.rect.contains(point))
        || snap
            .get("difficulty_value")
            .is_some_and(|el| el.layout.rect.contains(point))
    {
        return Some("difficulty");
    }
    for id in CAMPAIGN_BUTTON_IDS {
        if snap.get(id).is_some_and(|el| el.layout.rect.contains(point)) {
            return Some(id);
        }
    }
    None
}

fn hits_campaign() -> Vec<MenuHit> {
    let snap = campaign_snapshot();
    let mut out = Vec::new();
    let side_actions = [
        (CAMPAIGN_SIDE_IDS[0], MenuAction::SelectCampaignAllied),
        (CAMPAIGN_SIDE_IDS[1], MenuAction::SelectCampaignTutorial),
        (CAMPAIGN_SIDE_IDS[2], MenuAction::SelectCampaignSoviet),
    ];
    for (id, action) in side_actions {
        let Some(el) = snap.get(id) else {
            continue;
        };
        out.push(menu_hit_from_rect(id, action, el.layout.rect, true));
    }
    if let Some(el) = snap.get("difficulty") {
        out.push(menu_hit_from_rect(
            "difficulty",
            MenuAction::CycleCampaignDifficulty,
            el.layout.rect,
            true,
        ));
    }
    out.extend(hits_from_slot_ids(
        OriginalScreen::Campaign,
        &snap,
        &CAMPAIGN_BUTTON_IDS,
    ));
    out
}

fn hit_campaign_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    let snap = campaign_snapshot();
    let side_actions = [
        (CAMPAIGN_SIDE_IDS[0], MenuAction::SelectCampaignAllied),
        (CAMPAIGN_SIDE_IDS[1], MenuAction::SelectCampaignTutorial),
        (CAMPAIGN_SIDE_IDS[2], MenuAction::SelectCampaignSoviet),
    ];
    for (i, (id, action)) in side_actions.iter().enumerate() {
        if snap.get(id).is_some_and(|el| el.layout.rect.contains(point)) {
            return Some((i, *action));
        }
    }
    if snap
        .get("difficulty")
        .is_some_and(|el| el.layout.rect.contains(point))
    {
        return Some((3, MenuAction::CycleCampaignDifficulty));
    }
    // 侧三 + 难度轨占 0..3；右栏钮从 4 起与 `hits_campaign` / hover 序一致。
    hit_enabled_slot_at(OriginalScreen::Campaign, &snap, &CAMPAIGN_BUTTON_IDS, point)
        .map(|(i, action)| (4 + i, action))
}

fn hover_campaign_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    let order = [
        CAMPAIGN_SIDE_IDS[0],
        CAMPAIGN_SIDE_IDS[1],
        CAMPAIGN_SIDE_IDS[2],
        "difficulty",
        CAMPAIGN_BUTTON_IDS[0],
    ];
    hover_ids_at(&campaign_snapshot(), &order, point)
}

fn campaign_snapshot() -> LayoutSnapshot {
    solve_campaign()
}

fn hits_options() -> Vec<MenuHit> {
    hits_from_slot_ids(
        OriginalScreen::Options,
        &options_snapshot(),
        &OPTIONS_BUTTON_IDS,
    )
}

fn hit_options_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hit_enabled_slot_at(
        OriginalScreen::Options,
        &options_snapshot(),
        &OPTIONS_BUTTON_IDS,
        point,
    )
}

/// 悬停：含禁用钮。
fn hover_options_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hover_ids_at(&options_snapshot(), &OPTIONS_BUTTON_IDS, point)
}

fn options_snapshot() -> LayoutSnapshot {
    solve_options_page()
}

fn hits_exit_confirm() -> Vec<MenuHit> {
    hits_from_slot_ids(
        OriginalScreen::ExitConfirm,
        &exit_confirm_snapshot(),
        &EXIT_CONFIRM_BUTTON_IDS,
    )
}

fn hit_exit_confirm_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hit_enabled_slot_at(
        OriginalScreen::ExitConfirm,
        &exit_confirm_snapshot(),
        &EXIT_CONFIRM_BUTTON_IDS,
        point,
    )
}

fn hover_exit_confirm_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hover_ids_at(&exit_confirm_snapshot(), &EXIT_CONFIRM_BUTTON_IDS, point)
}

fn exit_confirm_snapshot() -> LayoutSnapshot {
    solve_exit_confirm()
}

fn hits_network() -> Vec<MenuHit> {
    hits_from_slot_ids(
        OriginalScreen::Network,
        &network_snapshot(),
        &NETWORK_BUTTON_IDS,
    )
}

fn hit_network_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hit_enabled_slot_at(
        OriginalScreen::Network,
        &network_snapshot(),
        &NETWORK_BUTTON_IDS,
        point,
    )
}

fn hover_network_at(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hover_ids_at(&network_snapshot(), &NETWORK_BUTTON_IDS, point)
}

fn network_snapshot() -> LayoutSnapshot {
    solve_network_page()
}

fn hits_load_screen(allow_retry: bool) -> Vec<MenuHit> {
    // 装载中不提供鼠标钮（Esc 仍取消）；失败后才露出重试/取消。
    if !allow_retry {
        return Vec::new();
    }
    hits_from_slot_ids(
        OriginalScreen::LoadScreen,
        &load_screen_snapshot(),
        &LOAD_SCREEN_BUTTON_IDS,
    )
}

fn hit_load_screen_at(
    cursor_x: f64,
    cursor_y: f64,
    win_w: f64,
    win_h: f64,
    allow_retry: bool,
) -> Option<(usize, MenuAction)> {
    if !allow_retry {
        return None;
    }
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hit_enabled_slot_at(
        OriginalScreen::LoadScreen,
        &load_screen_snapshot(),
        &LOAD_SCREEN_BUTTON_IDS,
        point,
    )
}

fn load_screen_snapshot() -> LayoutSnapshot {
    solve_load_screen()
}

fn hits_skirmish_lobby(_maps: &[BootMapCandidate]) -> Vec<MenuHit> {
    // 地图列表不在本页左侧；选图走右栏 `choose_map`（完整模态后续接）。
    hits_from_slot_ids(
        OriginalScreen::SkirmishLobby,
        &skirmish_lobby_snapshot(),
        &SKIRMISH_LOBBY_BUTTON_IDS,
    )
}

fn hit_skirmish_lobby_at(
    _maps: &[BootMapCandidate],
    cursor_x: f64,
    cursor_y: f64,
    win_w: f64,
    win_h: f64,
) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    hit_enabled_slot_at(
        OriginalScreen::SkirmishLobby,
        &skirmish_lobby_snapshot(),
        &SKIRMISH_LOBBY_BUTTON_IDS,
        point,
    )
}

/// 遭遇战大厅几何权威：`solve_skirmish_lobby` → `LayoutSnapshot`。
fn skirmish_lobby_snapshot() -> LayoutSnapshot {
    solve_skirmish_lobby()
}

/// 选图页几何权威：`solve_choose_map` → `LayoutSnapshot`。
fn choose_map_snapshot() -> LayoutSnapshot {
    solve_choose_map()
}

fn menu_hit_from_rect(
    entry_id: &'static str,
    action: MenuAction,
    rect: Rect,
    enabled: bool,
) -> MenuHit {
    menu_hit_from_rect_px(entry_id, action, rect_px_from_layout_rect(rect), enabled)
}

fn menu_hit_from_rect_px(
    entry_id: &'static str,
    action: MenuAction,
    rect: RectPx,
    enabled: bool,
) -> MenuHit {
    MenuHit {
        entry_id,
        action,
        rect,
        enabled,
    }
}

fn rect_px_from_layout_rect(rect: Rect) -> RectPx {
    RectPx::new(
        rect.x as i32,
        rect.y as i32,
        rect.width as i32,
        rect.height as i32,
    )
}

fn hits_choose_map(maps: &[BootMapCandidate], mode_count: usize, map_list_scroll: usize) -> Vec<MenuHit> {
    let snap = choose_map_snapshot();
    let mut hits = hits_from_slot_ids(OriginalScreen::ChooseMap, &snap, &CHOOSE_MAP_BUTTON_IDS);
    if let Some(list) = snap.get("game_type_list") {
        let list = rect_px_from_layout_rect(list.layout.rect);
        let visible = choose_map_visible_rows(list.h);
        for i in 0..mode_count.min(visible) {
            let row = choose_map_list_row_rect(list, i, list.w);
            hits.push(menu_hit_from_rect_px("mode_row", MenuAction::SelectMode(i), row, true));
        }
    }
    let Some(list) = snap.get("map_list") else {
        return hits;
    };
    let list = rect_px_from_layout_rect(list.layout.rect);
    let visible = choose_map_visible_rows(list.h);
    let scroll = clamp_map_list_scroll(map_list_scroll, maps.len(), visible);
    for (row_i, _) in maps.iter().skip(scroll).take(visible).enumerate() {
        let abs_i = scroll + row_i;
        let row = choose_map_list_row_rect(list, row_i, list.w);
        hits.push(menu_hit_from_rect_px("map_row", MenuAction::SelectMap(abs_i), row, true));
    }
    if let Some(preview) = snap.get("map_preview") {
        let r = rect_px_from_layout_rect(preview.layout.rect);
        hits.push(menu_hit_from_rect_px("map_preview", MenuAction::Noop, r, true));
    }
    // 右栏信息槽：与遭遇战一致，可悬停出底栏提示。
    for id in ["game_type", "map_label"] {
        if let Some(slot) = snap.get(id) {
            let r = rect_px_from_layout_rect(slot.layout.rect);
            hits.push(menu_hit_from_rect_px(id, MenuAction::Noop, r, true));
        }
    }
    hits
}

fn hit_choose_map_at(
    maps: &[BootMapCandidate],
    mode_count: usize,
    map_list_scroll: usize,
    cursor_x: f64,
    cursor_y: f64,
    win_w: f64,
    win_h: f64,
) -> Option<(usize, MenuAction)> {
    let point = shell_point(cursor_x, cursor_y, win_w, win_h)?;
    let snap = choose_map_snapshot();
    if let Some(hit) =
        hit_enabled_slot_at(OriginalScreen::ChooseMap, &snap, &CHOOSE_MAP_BUTTON_IDS, point)
    {
        return Some(hit);
    }
    let mode_base = CHOOSE_MAP_BUTTON_IDS.len();
    if let Some(list) = snap.get("game_type_list") {
        let list = list.layout.rect;
        if list.contains(point) {
            let row = ((point.y - list.y) / CHOOSE_MAP_LIST_ROW_H as f32).floor().max(0.0) as usize;
            let visible = choose_map_visible_rows(list.height as i32);
            if row < mode_count.min(visible) {
                return Some((mode_base + row, MenuAction::SelectMode(row)));
            }
        }
    }
    let map_base = mode_base + mode_count;
    if let Some(list) = snap.get("map_list") {
        let list = list.layout.rect;
        if list.contains(point) {
            let row = ((point.y - list.y) / CHOOSE_MAP_LIST_ROW_H as f32).floor().max(0.0) as usize;
            let visible = choose_map_visible_rows(list.height as i32);
            let scroll = clamp_map_list_scroll(map_list_scroll, maps.len(), visible);
            let window = maps.len().saturating_sub(scroll).min(visible);
            if row < window {
                return Some((map_base + row, MenuAction::SelectMap(scroll + row)));
            }
        }
    }
    if let Some(preview) = snap.get("map_preview") {
        if preview.layout.rect.contains(point) {
            return Some((map_base + maps.len(), MenuAction::Noop));
        }
    }
    let info_base = map_base + maps.len() + 1;
    for (i, id) in ["game_type", "map_label"].into_iter().enumerate() {
        if let Some(slot) = snap.get(id) {
            if slot.layout.rect.contains(point) {
                return Some((info_base + i, MenuAction::Noop));
            }
        }
    }
    None
}
