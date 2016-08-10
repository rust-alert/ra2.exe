//! 选项页控件合成集成测试。

use ra_layout::{rect_px_from_snapshot, solve_options_page};
use ra_renderer::RgbaImage;
use ra_types::{DisplayMode, PresentFeel};
use ra_widgets::{compose::*, options_dialog::OptionsDialogState};

#[test]
fn paint_options_draws_music_thumb() {
    let mut page = RgbaImage::from_raw(800, 600, vec![0u8; 800 * 600 * 4]).unwrap();
    let snap = solve_options_page();
    let state = OptionsDialogState::from_shell(DisplayMode::W800H600, 1.0, 0.0, PresentFeel::DEFAULT);
    paint_options_dialog_controls(&mut page, &snap, &state, None, None);
    let track = rect_px_from_snapshot(&snap, "track_music");
    let px = track.x + track.w - 8;
    let py = track.y + track.h / 2;
    let di = ((py as u32 * page.width() + px as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 3], &[220, 40, 40]);
}
