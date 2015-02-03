//! 集成测试：原 `src/ui_assets.rs` 内联测试迁出。

use ra_widgets::ui_assets::*;
use ra_renderer::RgbaImage;
#[test]
fn downscale_halves_dimensions() {
    let src = RgbaImage::from_raw(4, 2, vec![255u8; 4 * 2 * 4]).unwrap();
    let out = downscale_to_fit(&src, 2, 2).unwrap();
    assert_eq!((out.width(), out.height()), (2, 1));
}
