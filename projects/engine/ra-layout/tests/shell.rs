//! 壳层像素辅助与常量集成测试。

use ra_layout::*;

#[test]
fn main_menu_panel_sits_on_right_edge() {
    let snap = ra_layout::solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    let panel_top = rect_px_from_snapshot(&snap, "panel_top");
    let first = rect_px_from_snapshot(&snap, MAIN_MENU_BUTTON_IDS[0]);
    assert_eq!(panel_top.x + panel_top.w, 800);
    assert!(ra_layout::RightPanelChrome::shell_defaults().tile_count() > 0);
    assert_eq!(first.w, BUTTON_CELL_W);
    assert_eq!(first.y, RIGHT_PANEL_TOP_H);
}

#[test]
fn main_menu_bottom_cover_is_remainder_not_fixed_65() {
    let snap = ra_layout::solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    // 600 - 199 = 401；401/42 = 9 格；底盖 y=199+378=577，高=23。
    assert_eq!(ra_layout::RightPanelChrome::shell_defaults().tile_count(), 9);
    assert_eq!(
        rect_px_from_snapshot(&snap, "panel_bottom"),
        RectPx::new(632, 577, 168, 23)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "movie"),
        RectPx::new(0, 0, 632, 570)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "lower_strip"),
        RectPx::new(0, 568, 632, 32)
    );
}

#[test]
fn main_menu_title_and_tooltip_rects() {
    let snap = ra_layout::solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "title"),
        RectPx::new(635, 9, 163, 18)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "tooltip"),
        RectPx::new(10, 579, 455, 20)
    );
}

#[test]
fn main_menu_exit_sits_on_bottom_cover() {
    let snap = ra_layout::solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    let expected_y = [199, 241, 283, 325, 367];
    for (i, y) in expected_y.iter().enumerate() {
        assert_eq!(
            rect_px_from_snapshot(&snap, MAIN_MENU_BUTTON_IDS[i]),
            RectPx::new(644, *y, 156, 42)
        );
    }
    // Exit：底盖上沿一行 → y = 577 - 42 = 535。
    assert_eq!(
        rect_px_from_snapshot(&snap, MAIN_MENU_BUTTON_IDS[5]),
        RectPx::new(644, 535, 156, 42)
    );
}

#[test]
fn skirmish_lobby_matches_game_exe_dialog_0x102() {
    let snap = ra_layout::solve_skirmish_lobby();
    assert_eq!(SKIRMISH_LOBBY_BUTTON_IDS, ["start", "choose_map", "back"]);
    // 开始/选图：模板 y DLU 149/176 → 吸附到 tile 1/2。
    assert_eq!(
        rect_px_from_snapshot(&snap, "start"),
        RectPx::new(644, 241, 156, 42)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "choose_map"),
        RectPx::new(644, 283, 156, 42)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "back"),
        RectPx::new(644, 535, 156, 42)
    );
    // 预览 `0x468` DLU (324,23,96,69) → right_anchor。
    assert_eq!(
        rect_px_from_snapshot(&snap, "map_preview"),
        RectPx::new(644, 37, 144, 112)
    );
    // 标题 `0x694` DLU (318,1,108,10) → right_anchor。
    assert_eq!(
        rect_px_from_snapshot(&snap, "title"),
        RectPx::new(635, 2, 162, 16)
    );
    // 地图名底板 `sdmpbtn`：贴右缘，底边落在第一根 tile 下沿。
    assert_eq!(
        rect_px_from_snapshot(&snap, "map_name_plate"),
        RectPx::new(644, 157, 156, 84)
    );
    // 左栏表单：内容区居中（固有尺寸不变，原点随居中偏移）。
    let player_name = rect_px_from_snapshot(&snap, "player_name");
    assert_eq!((player_name.w, player_name.h), (150, 24));
    assert!(player_name.y >= 40, "top margin");
    let color = rect_px_from_snapshot(&snap, "color_face_0");
    let check4 = rect_px_from_snapshot(&snap, "checkbox_4");
    let right = (color.x + color.w).max(check4.x + check4.w);
    let mid = (player_name.x + right) / 2;
    assert!((mid - 316).abs() <= 4, "form mid {mid}");
    let checkbox_quick = rect_px_from_snapshot(&snap, "checkbox_quick");
    assert_eq!((checkbox_quick.w, checkbox_quick.h), (150, 16));
    let track_speed = rect_px_from_snapshot(&snap, "track_speed");
    assert_eq!((track_speed.w, track_speed.h), (128, 21));
    assert_eq!(checkbox_quick.y, track_speed.y);
    // 对话框页也画壳层底条；提示贴 tooltip 带。
    assert_eq!(
        rect_px_from_snapshot(&snap, "lower_strip"),
        RectPx::new(0, 568, 632, 32)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "status_help"),
        RectPx::new(15, 579, 455, 20)
    );
}

#[test]
fn choose_map_matches_game_exe_dialog_0x6b() {
    let snap = ra_layout::solve_choose_map();
    assert_eq!(CHOOSE_MAP_BUTTON_IDS, ["use_map", "create_random", "cancel"]);
    // 使用地图 / 随机：与遭遇战「开始 / 选图」同格（DLU 149/176 → tile 1/2），落在 `map_name_plate` 下方，避免盖住挡板。
    assert_eq!(
        rect_px_from_snapshot(&snap, "use_map"),
        RectPx::new(644, 241, 156, 42)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "create_random"),
        RectPx::new(644, 283, 156, 42)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "cancel"),
        RectPx::new(644, 535, 156, 42)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "map_preview"),
        RectPx::new(644, 37, 144, 112)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "title"),
        RectPx::new(635, 2, 162, 16)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "map_name_plate"),
        RectPx::new(644, 157, 156, 84)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "game_type_list"),
        RectPx::new(30, 127, 195, 260)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "map_list"),
        RectPx::new(252, 127, 195, 260)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "label_engagement"),
        RectPx::new(35, 33, 386, 20)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "lower_strip"),
        RectPx::new(0, 568, 632, 32)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "status_help"),
        RectPx::new(15, 579, 455, 20)
    );
}

#[test]
fn campaign_matches_fsbkgdlg_side_origins() {
    use ra_layout::{rect_px_from_snapshot, solve_campaign, CAMPAIGN_BUTTON_IDS, CAMPAIGN_SIDE_IDS};

    let snap = solve_campaign();
    assert_eq!(CAMPAIGN_BUTTON_IDS, ["back"]);
    assert_eq!(CAMPAIGN_SIDE_IDS, ["allied", "tutorial", "soviet"]);
    // 仅「上一页」贴底盖。
    assert_eq!(
        rect_px_from_snapshot(&snap, "back"),
        RectPx::new(644, 535, 156, 42)
    );
    // 三侧图：相对 `fsbkgdlg` 的 SHP 原点与画布。
    assert_eq!(
        rect_px_from_snapshot(&snap, "allied"),
        RectPx::new(30, 26, 570, 135)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "tutorial"),
        RectPx::new(82, 187, 468, 108)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "soviet"),
        RectPx::new(98, 298, 444, 149)
    );
    // 难度：侧图下方固定槽位。
    assert_eq!(
        rect_px_from_snapshot(&snap, "difficulty_label"),
        RectPx::new(191, 454, 100, 20)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "difficulty_value"),
        RectPx::new(338, 454, 100, 20)
    );
    let track = rect_px_from_snapshot(&snap, "difficulty");
    assert_eq!(track, RectPx::new(191, 483, 247, 13));
    let soviet = rect_px_from_snapshot(&snap, "soviet");
    assert!(track.y >= soviet.y + soviet.h);
    assert_eq!(
        rect_px_from_snapshot(&snap, "title"),
        RectPx::new(635, 9, 163, 18)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, "tooltip"),
        RectPx::new(10, 579, 455, 20)
    );
}

#[test]
fn exit_confirm_centers_pudlgbgn_panel() {
    let snap = ra_layout::solve_exit_confirm();
    // 画布 451×326；居中 ((800-451)+1)/2=175，((600-326)+1)/2=137。
    assert_eq!(
        rect_px_from_snapshot(&snap, "dialog"),
        RectPx::new(175, 137, EXIT_CONFIRM_DIALOG_W, EXIT_CONFIRM_DIALOG_H)
    );
    // 正文左上锚点区；按钮原点取 DLU，尺寸取 `mnbttn` 126×25。
    assert_eq!(
        rect_px_from_snapshot(&snap, "prompt"),
        RectPx::new(235, 202, 330, 81)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, EXIT_CONFIRM_BUTTON_IDS[0]),
        RectPx::new(486, 356, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H)
    );
    assert_eq!(
        rect_px_from_snapshot(&snap, EXIT_CONFIRM_BUTTON_IDS[1]),
        RectPx::new(486, 421, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H)
    );
}

#[test]
fn shell_page_layouts_ignore_viewport_size() {
    // 壳层页内容落在 800×600 设计画布；窗口适配由相机负责，禁止页面再算第二套几何。
    for (w, h) in [(640u32, 480u32), (800, 600), (1280, 720), (2560, 1440)] {
        let _ = (w, h);
        let main = ra_layout::solve_shell_page(
            "main_menu",
            &MAIN_MENU_BUTTON_IDS[..5],
            Some(MAIN_MENU_BUTTON_IDS[5]),
        );
        assert_eq!(
            rect_px_from_snapshot(&main, "exit"),
            RectPx::new(644, 535, 156, 42),
            "{w}x{h} exit"
        );

        let lobby = ra_layout::solve_skirmish_lobby();
        assert_eq!(
            rect_px_from_snapshot(&lobby, "start"),
            RectPx::new(644, 241, 156, 42),
            "{w}x{h} start"
        );

        let maps = ra_layout::solve_choose_map();
        assert_eq!(
            rect_px_from_snapshot(&maps, "map_list"),
            RectPx::new(252, 127, 195, 260),
            "{w}x{h} map_list"
        );

        let campaign = ra_layout::solve_campaign();
        assert_eq!(
            rect_px_from_snapshot(&campaign, "allied"),
            RectPx::new(30, 26, 570, 135),
            "{w}x{h} allied"
        );

        let exit = ra_layout::solve_exit_confirm();
        assert_eq!(
            rect_px_from_snapshot(&exit, "dialog"),
            RectPx::new(175, 137, EXIT_CONFIRM_DIALOG_W, EXIT_CONFIRM_DIALOG_H),
            "{w}x{h} exit dialog"
        );
    }
}

#[test]
fn window_to_shell_px_matches_fit_camera() {
    assert_eq!(window_to_shell_px(400.0, 300.0, 800.0, 600.0), (400, 300));
    assert_eq!(window_to_shell_px(0.0, 0.0, 800.0, 600.0), (0, 0));
    // 2× 等比放大。
    assert_eq!(window_to_shell_px(800.0, 600.0, 1600.0, 1200.0), (400, 300));
    // 1280×720：zoom=1.2，左右黑边；内容左上角落在屏幕 x=160。
    assert_eq!(window_to_shell_px(160.0, 0.0, 1280.0, 720.0), (0, 0));
    assert_eq!(window_to_shell_px(640.0, 360.0, 1280.0, 720.0), (400, 300));
    // 点在左侧黑边 → 负壳层 x。
    let (sx, _) = window_to_shell_px(0.0, 360.0, 1280.0, 720.0);
    assert!(sx < 0, "pillarbox maps outside design canvas, got {sx}");
}

#[test]
fn shell_content_rect_in_window_matches_fit_camera() {
    assert_eq!(shell_content_rect_in_window(800, 600), RectPx::new(0, 0, 800, 600));
    assert_eq!(shell_content_rect_in_window(1600, 1200), RectPx::new(0, 0, 1600, 1200));
    // 1280×720：zoom=1.2，左右 pillarbox 各 160。
    assert_eq!(shell_content_rect_in_window(1280, 720), RectPx::new(160, 0, 960, 720));
}
