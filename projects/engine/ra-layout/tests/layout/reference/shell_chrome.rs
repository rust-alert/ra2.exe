//! 自 `engine/ra-layout/src/reference/shell_chrome.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/shell_chrome.rs :: tests
use ra_layout::{
    LayoutEngine, RightPanelChrome, Viewport,
    reference::{from_template::shell_design_size, shell_chrome::*},
    shell::{RectPx, rect_px_from_snapshot},
};

#[test]
fn shell_chrome_matches_golden_slots() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &shell_chrome_layout_tree(chrome));
    assert_eq!(rect_px_from_snapshot(&snap, "panel_top"), RectPx::new(632, 0, 168, 199));
    assert_eq!(rect_px_from_snapshot(&snap, "panel_tile"), RectPx::new(632, 199, 168, 42));
    assert_eq!(rect_px_from_snapshot(&snap, "panel_bottom"), RectPx::new(632, 577, 168, 23));
    assert_eq!(rect_px_from_snapshot(&snap, "background"), RectPx::new(0, 0, 632, 570));
    assert_eq!(rect_px_from_snapshot(&snap, "movie"), RectPx::new(0, 0, 632, 570));
    assert_eq!(rect_px_from_snapshot(&snap, "lower_strip"), RectPx::new(0, 568, 632, 32));
    assert_eq!(rect_px_from_snapshot(&snap, "title"), RectPx::new(635, 9, 163, 18));
    assert_eq!(rect_px_from_snapshot(&snap, "tooltip"), RectPx::new(10, 579, 455, 20));
    assert_eq!(chrome.tile_count(), 9);
}

#[test]
fn main_menu_rail_buttons_match_golden_cells() {
    let chrome = RightPanelChrome::shell_defaults();
    let root =
        right_rail_buttons_layout_tree("main_menu", &["single_player", "ww_online", "network", "movies", "options"], Some("exit"), chrome);
    let snap = LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &root);
    let expected = [
        ("single_player", RectPx::new(644, 199, 156, 42)),
        ("ww_online", RectPx::new(644, 241, 156, 42)),
        ("network", RectPx::new(644, 283, 156, 42)),
        ("movies", RectPx::new(644, 325, 156, 42)),
        ("options", RectPx::new(644, 367, 156, 42)),
        ("exit", RectPx::new(644, 535, 156, 42)),
    ];
    for (id, cell) in expected {
        assert_eq!(rect_px_from_snapshot(&snap, id), cell, "{id}");
    }
}

#[test]
fn shell_page_tree_matches_chrome_and_buttons() {
    let chrome = RightPanelChrome::shell_defaults();
    let ids = ["single_player", "ww_online", "network", "movies", "options", "exit"];
    let snap = LayoutEngine.solve(
        Viewport { size: shell_design_size(chrome), ..Viewport::default() },
        &shell_page_layout_tree("main_menu", &ids[..5], Some(ids[5]), chrome),
    );
    assert_eq!(rect_px_from_snapshot(&snap, "title"), RectPx::new(635, 9, 163, 18));
    assert_eq!(rect_px_from_snapshot(&snap, "exit"), RectPx::new(644, 535, 156, 42));
}
