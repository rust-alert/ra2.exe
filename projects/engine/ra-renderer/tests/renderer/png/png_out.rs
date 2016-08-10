//! 集成测试：原 `src/png_out.rs` 内联测试迁出。

use ra_renderer::*;

#[test]
fn encode_png_emits_signature() {
    let img = RgbaImage::from_raw(2, 2, vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255]).unwrap();
    let bytes = encode_png(&img).unwrap();
    assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
}
