//! snapshot 命中与选图页控件 id 对齐。

use ra_layout::solve_choose_map;
use ra_widgets::input::hit_id_at;

#[test]
fn choose_map_buttons_hit_via_snapshot() {
    let snap = solve_choose_map();
    assert_eq!(hit_id_at(&snap, 650.0, 260.0), Some("use_map"));
    assert_eq!(hit_id_at(&snap, 650.0, 300.0), Some("create_random"));
    assert_eq!(hit_id_at(&snap, 650.0, 540.0), Some("cancel"));
    assert_eq!(hit_id_at(&snap, 100.0, 200.0), Some("game_type_list"));
}
