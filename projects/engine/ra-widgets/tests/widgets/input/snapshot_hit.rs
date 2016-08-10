//! snapshot 命中与选图页控件 id 对齐。

use ra_layout::{LayoutSnapshot, solve_choose_map};
use ra_widgets::input::hit_id_at;

fn center(snap: &LayoutSnapshot, id: &str) -> (f32, f32) {
    let r = snap.get(id).unwrap_or_else(|| panic!("missing id {id}")).layout.rect;
    (r.x + r.width * 0.5, r.y + r.height * 0.5)
}

#[test]
fn choose_map_buttons_hit_via_snapshot() {
    let snap = solve_choose_map();
    // 左栏居中后列表离开旧 DLU 像素，点击点必须取自当前 snapshot。
    let (ux, uy) = center(&snap, "use_map");
    let (rx, ry) = center(&snap, "create_random");
    let (cx, cy) = center(&snap, "cancel");
    let (gx, gy) = center(&snap, "game_type_list");
    assert_eq!(hit_id_at(&snap, ux, uy), Some("use_map"));
    assert_eq!(hit_id_at(&snap, rx, ry), Some("create_random"));
    assert_eq!(hit_id_at(&snap, cx, cy), Some("cancel"));
    assert_eq!(hit_id_at(&snap, gx, gy), Some("game_type_list"));
}
