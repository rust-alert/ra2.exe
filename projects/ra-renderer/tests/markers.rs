//! 集成测试：原 `src/markers.rs` 内联测试迁出。

use ra_renderer::ndc_visible;

#[test]
fn ndc_margin_rejects_far_points() {
    assert!(ndc_visible([0.0, 0.0], 1.15));
    assert!(ndc_visible([1.1, -1.1], 1.15));
    assert!(!ndc_visible([2.0, 0.0], 1.15));
    assert!(!ndc_visible([0.0, -3.0], 1.15));
}
