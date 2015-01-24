//! 集成测试：原 `src/ui_layout.rs` 内联测试迁出。

use ra_layout::ui_layout::*;

#[test]
fn main_menu_panel_sits_on_right_edge() {
    let layout = main_menu_layout(1024, 768);
    assert_eq!(layout.canvas.w, 800);
    assert_eq!(layout.panel_top.x + layout.panel_top.w, 800);
    assert!(layout.panel_tile_count > 0);
    assert_eq!(layout.buttons[0].w, BUTTON_CELL_W);
    assert_eq!(layout.buttons[0].y, RIGHT_PANEL_TOP_H);
    assert_eq!(MAIN_MENU_BUTTON_IDS.len(), layout.buttons.len());
}

#[test]
fn main_menu_bottom_cover_is_remainder_not_fixed_65() {
    let layout = main_menu_layout(800, 600);
    // 600 - 199 = 401；401/42 = 9 格；底盖 y=199+378=577，高=23。
    assert_eq!(layout.panel_tile_count, 9);
    assert_eq!(layout.panel_bottom, RectPx::new(632, 577, 168, 23));
    assert_eq!(layout.movie, RectPx::new(0, 0, 632, 570));
    assert_eq!(layout.lower_strip, RectPx::new(0, 568, 632, 32));
}

#[test]
fn main_menu_title_and_tooltip_rects() {
    let layout = main_menu_layout(800, 600);
    assert_eq!(layout.title, RectPx::new(635, 9, 163, 18));
    assert_eq!(layout.tooltip, RectPx::new(10, 579, 455, 20));
}

#[test]
fn main_menu_exit_sits_on_bottom_cover() {
    let layout = main_menu_layout(800, 600);
    let expected_y = [199, 241, 283, 325, 367];
    for (i, y) in expected_y.iter().enumerate() {
        assert_eq!(layout.buttons[i], RectPx::new(644, *y, 156, 42));
    }
    // Exit：底盖上沿一行 → y = 577 - 42 = 535。
    assert_eq!(layout.buttons[5], RectPx::new(644, 535, 156, 42));
}

#[test]
fn skirmish_lobby_matches_game_exe_dialog_0x102() {
    let layout = skirmish_lobby_layout(800, 600);
    assert_eq!(SKIRMISH_LOBBY_BUTTON_IDS, ["start", "choose_map", "back"]);
    // 开始/选图：模板 y DLU 149/176 → 吸附到 tile 1/2。
    assert_eq!(layout.shell.buttons[0], RectPx::new(644, 241, 156, 42));
    assert_eq!(layout.shell.buttons[1], RectPx::new(644, 283, 156, 42));
    assert_eq!(layout.shell.buttons[2], RectPx::new(644, 535, 156, 42));
    // 预览 `0x468` DLU (324,23,96,69) → right_anchor。
    assert_eq!(layout.map_preview, RectPx::new(644, 37, 144, 112));
    // 标题 `0x694` DLU (318,1,108,10) → right_anchor。
    assert_eq!(layout.title, RectPx::new(635, 2, 162, 16));
    // 地图名底板 `sdmpbtn`：贴右缘，底边落在第一根 tile 下沿。
    assert_eq!(layout.map_name_plate, RectPx::new(644, 157, 156, 84));
    // 玩家名 `0x6A0` DLU (35,11,100,12)。
    assert_eq!(layout.player_name, RectPx::new(53, 18, 150, 20));
    // 快速游戏 `0x54E` DLU (35,145,100,10)。
    assert_eq!(layout.checkboxes[0], RectPx::new(53, 236, 150, 16));
    // 速度滑条 `0x529` DLU (214,145,85,13)。
    assert_eq!(layout.track_speed, RectPx::new(321, 236, 128, 21));
    assert_eq!(layout.label_speed, RectPx::new(219, 236, 90, 16));
    assert_eq!(layout.shell.lower_strip, RectPx::new(0, 0, 0, 0));
}

#[test]
fn choose_map_matches_game_exe_dialog_0x6b() {
    let layout = choose_map_layout(800, 600);
    assert_eq!(CHOOSE_MAP_BUTTON_IDS, ["use_map", "create_random", "cancel"]);
    // 使用地图 `0x6C5` DLU y 122 → 吸附 tile 0；随机 y 149 → tile 1；取消贴底盖。
    assert_eq!(layout.shell.buttons[0], RectPx::new(644, 199, 156, 42));
    assert_eq!(layout.shell.buttons[1], RectPx::new(644, 241, 156, 42));
    assert_eq!(layout.shell.buttons[2], RectPx::new(644, 535, 156, 42));
    assert_eq!(layout.map_preview, RectPx::new(644, 37, 144, 112));
    assert_eq!(layout.title, RectPx::new(635, 2, 162, 16));
    assert_eq!(layout.map_name_plate, RectPx::new(644, 157, 156, 84));
    assert_eq!(layout.game_type_list, RectPx::new(30, 127, 195, 260));
    assert_eq!(layout.map_list, RectPx::new(252, 127, 195, 260));
    assert_eq!(layout.label_engagement, RectPx::new(35, 33, 386, 20));
    assert_eq!(layout.shell.lower_strip, RectPx::new(0, 0, 0, 0));
}

#[test]
fn campaign_matches_fsbkgdlg_side_origins() {
    use ra_layout::ui_layout::{CAMPAIGN_BUTTON_IDS, CAMPAIGN_SIDE_IDS, campaign_layout};
    let layout = campaign_layout(800, 600);
    assert_eq!(CAMPAIGN_BUTTON_IDS, ["back"]);
    assert_eq!(CAMPAIGN_SIDE_IDS, ["allied", "tutorial", "soviet"]);
    // 仅「上一页」贴底盖。
    assert_eq!(layout.shell.buttons[0], RectPx::new(644, 535, 156, 42));
    // 三侧图：相对 `fsbkgdlg` 的 SHP 原点与画布。
    assert_eq!(layout.allied, RectPx::new(30, 26, 570, 135));
    assert_eq!(layout.tutorial, RectPx::new(82, 187, 468, 108));
    assert_eq!(layout.soviet, RectPx::new(98, 298, 444, 149));
    // 难度：原版截图映到壳层坐标。
    assert_eq!(layout.difficulty_label, RectPx::new(191, 454, 100, 20));
    assert_eq!(layout.difficulty_value, RectPx::new(338, 454, 100, 20));
    assert_eq!(layout.difficulty_track, RectPx::new(191, 483, 247, 13));
    assert!(layout.difficulty_track.y >= layout.soviet.y + layout.soviet.h);
    // 标题 / 底栏提示与主菜单壳层 chrome 同格。
    assert_eq!(layout.title, layout.shell.title);
    assert_eq!(layout.status_help, layout.shell.tooltip);
    assert_eq!(layout.title, RectPx::new(635, 9, 163, 18));
    assert_eq!(layout.status_help, RectPx::new(10, 579, 455, 20));
}

#[test]
fn exit_confirm_centers_pudlgbgn_panel() {
    let dlg = exit_confirm_layout(800, 600);
    // 画布 451×326；居中 ((800-451)+1)/2=175，((600-326)+1)/2=137。
    assert_eq!(dlg.dialog, RectPx::new(175, 137, EXIT_CONFIRM_DIALOG_W, EXIT_CONFIRM_DIALOG_H));
    // 正文左上锚点区；按钮原点取 DLU，尺寸取 `mnbttn` 126×25。
    assert_eq!(dlg.prompt, RectPx::new(235, 202, 330, 81));
    assert_eq!(dlg.buttons[0], RectPx::new(486, 356, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H));
    assert_eq!(dlg.buttons[1], RectPx::new(486, 421, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H));
}
