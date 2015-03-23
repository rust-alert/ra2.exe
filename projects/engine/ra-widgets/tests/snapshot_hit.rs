//! snapshot 命中与 `0x6B` 控件 id 对齐。

use ra_adaptor::dialog_template_0x6b;
use ra_layout::{dialog_layout_tree, LayoutEngine, RightPanelChrome, Viewport};
use ra_widgets::input::hit_id_at;

#[test]
fn choose_map_buttons_hit_via_snapshot() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: ra_layout::shell_design_size(chrome),
            ..Viewport::default()
        },
        &dialog_layout_tree("dialog_0x6b", &dialog_template_0x6b(), chrome),
    );
    assert_eq!(hit_id_at(&snap, 650.0, 210.0), Some("use_map"));
    assert_eq!(hit_id_at(&snap, 650.0, 250.0), Some("create_random"));
    assert_eq!(hit_id_at(&snap, 650.0, 540.0), Some("cancel"));
    assert_eq!(hit_id_at(&snap, 100.0, 200.0), Some("game_type_list"));
}
