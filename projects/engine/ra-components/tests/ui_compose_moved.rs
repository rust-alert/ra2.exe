//! 集成测试：原 `src/ui_compose.rs` 内联测试迁出。

use ra_components::{
    options_dialog::{OptionsDialogLayout, OptionsDialogState},
    ui_compose::*,
};
use ra_renderer::RgbaImage;
use ra_types::{DisplayMode, PresentFeel};

#[test]
fn paint_options_draws_music_thumb() {
    let mut page = RgbaImage::from_raw(800, 600, vec![0u8; 800 * 600 * 4]).unwrap();
    let layout = OptionsDialogLayout::new();
    let state = OptionsDialogState::from_shell(DisplayMode::W800H600, 1.0, 0.0, PresentFeel::DEFAULT);
    paint_options_dialog_controls(&mut page, &layout, &state, None, None);
    let track = layout.track_music;
    let px = track.x + track.w - 8;
    let py = track.y + track.h / 2;
    let di = ((py as u32 * page.width() + px as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 3], &[220, 40, 40]);
}
