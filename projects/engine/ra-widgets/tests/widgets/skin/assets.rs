//! 菜单资源挂载与探测集成测试。

use ra_renderer::RgbaImage;
use ra_widgets::skin::assets::*;
#[test]
fn downscale_halves_dimensions() {
    let src = RgbaImage::from_raw(4, 2, vec![255u8; 4 * 2 * 4]).unwrap();
    let out = downscale_to_fit(&src, 2, 2).unwrap();
    assert_eq!((out.width(), out.height()), (2, 1));
}
