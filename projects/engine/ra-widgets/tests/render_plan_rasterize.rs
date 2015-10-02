//! `RenderPlan` 纯色占位栅格化。

use ra_layout::SHELL_BASE_H;
use ra_layout::SHELL_BASE_W;
use ra_widgets::RenderPlan;

#[test]
fn battle_pause_placeholders_rasterize_opaque_rail_pixel() {
    let plan = RenderPlan::battle_pause_placeholders();
    let page = plan
        .rasterize_solids(SHELL_BASE_W as u32, SHELL_BASE_H as u32)
        .expect("page");
    let resume = plan.rect_of("resume").expect("resume");
    let x = (resume.x as u32) + 4;
    let y = (resume.y as u32) + 4;
    let di = ((y * page.width() + x) * 4) as usize;
    let px = &page.as_raw()[di..di + 4];
    assert_eq!(px[3], 255, "resume placeholder should be opaque");
    assert!(px[0] > 0 || px[1] > 0 || px[2] > 0);
}

#[test]
fn excluding_ids_drops_named_commands() {
    let plan = RenderPlan::battle_pause_placeholders();
    assert!(plan.rect_of("background").is_some());
    let filtered = plan.excluding_ids(&["background", "movie"]);
    assert!(filtered.rect_of("background").is_none());
    assert!(filtered.rect_of("resume").is_some());
}

#[test]
fn retaining_ids_keeps_only_named_commands() {
    let plan = RenderPlan::battle_hud_placeholders(800, 600);
    let rail = plan.retaining_ids(&["sidebar", "bottom_strip"]);
    assert!(rail.rect_of("sidebar").is_some());
    assert!(rail.rect_of("bottom_strip").is_some());
    assert!(rail.rect_of("command_bar").is_none());
}
