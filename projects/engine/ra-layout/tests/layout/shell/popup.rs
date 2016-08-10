//! 自 `engine/ra-layout/src/shell/popup.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/shell/popup.rs :: tests
use ra_layout::shell::{RectPx, popup::*};

#[test]
fn popup_geometry_sits_below_anchor() {
    let face = RectPx::new(10, 20, 40, 16);
    assert_eq!(popup_row_below(face, 24, 0), RectPx::new(10, 36, 40, 24));
    assert_eq!(popup_row_below(face, 24, 2), RectPx::new(10, 84, 40, 24));
    assert_eq!(popup_list_below(face, 16, 3), RectPx::new(10, 36, 40, 48));
    assert_eq!(popup_list_below_min_w(face, 16, 2, 28), RectPx::new(10, 36, 40, 32));
    assert_eq!(popup_list_below_min_w(RectPx::new(0, 0, 10, 8), 16, 1, 28), RectPx::new(0, 8, 28, 16));
}
