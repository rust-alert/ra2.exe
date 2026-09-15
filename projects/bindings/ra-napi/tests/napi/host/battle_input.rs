//! 自 `bindings/ra-napi/src/host/battle_input.rs` 迁出的单元测试（集成测试 crate）。

// 自 bindings/ra-napi/src/host/battle_input.rs :: tests
use ra_napi::host::battle_input::*;

#[test]
fn small_move_stays_maybe_click_and_releases_as_click() {
    let g = LeftGesture::begin(100.0, 100.0);
    let g = g.on_cursor_moved(103.0, 102.0);
    assert!(matches!(g, LeftGesture::MaybeClick { .. }));
    let (idle, action) = g.release();
    assert_eq!(idle, LeftGesture::Idle);
    assert_eq!(action, LeftReleaseAction::Click);
}

#[test]
fn large_move_enters_marquee_and_does_not_need_pan() {
    let g = LeftGesture::begin(10.0, 10.0);
    let g = g.on_cursor_moved(30.0, 40.0);
    match g {
        LeftGesture::Marquee { origin_x, origin_y, cur_x, cur_y } => {
            assert_eq!((origin_x, origin_y), (10.0, 10.0));
            assert_eq!((cur_x, cur_y), (30.0, 40.0));
        }
        other => panic!("expected Marquee, got {other:?}"),
    }
    let (_, action) = g.release();
    assert_eq!(action, LeftReleaseAction::Marquee(ScreenRect { x: 10.0, y: 10.0, w: 20.0, h: 30.0 }));
}

#[test]
fn screen_rect_intersects_entity_hitbox() {
    let drag = ScreenRect::from_drag(0.0, 0.0, 50.0, 50.0);
    let hit = ScreenRect::from_center_half(40.0, 40.0, MARQUEE_HIT_HALF_PX);
    assert!(drag.intersects(&hit));
    let miss = ScreenRect::from_center_half(200.0, 200.0, MARQUEE_HIT_HALF_PX);
    assert!(!drag.intersects(&miss));
}

#[test]
fn vehicle_marquee_hitbox_covers_body_above_feet() {
    // 脚点在 (100, 100)；框只罩住上方车身时，旧 12px 半宽会漏，载具半宽+上移应命中。
    let feet = (100.0_f32, 100.0_f32);
    let body_box = ScreenRect::from_drag(70.0, 40.0, 130.0, 85.0);
    let old = ScreenRect::from_center_half(feet.0, feet.1, 12.0);
    assert!(!body_box.intersects(&old));
    let vehicle = ScreenRect::from_center_half(feet.0, feet.1 - MARQUEE_VEHICLE_LIFT_PX, MARQUEE_HIT_HALF_VEHICLE_PX);
    assert!(body_box.intersects(&vehicle));
}

#[test]
fn distance_to_point_zero_inside() {
    let r = ScreenRect::from_drag(10.0, 10.0, 40.0, 40.0);
    assert_eq!(r.distance_to_point(20.0, 20.0), 0.0);
    assert!(r.contains_point(20.0, 20.0));
    assert!(r.distance_to_point(50.0, 25.0) > 0.0);
}

#[test]
fn edge_scroll_left_and_right_oppose() {
    let (dx_l, dy_l) = edge_scroll_screen_delta(5.0, 100.0, 800, 600, 16.0, 640.0, 0.1);
    assert!(dx_l > 0.0);
    assert_eq!(dy_l, 0.0);
    let (dx_r, _) = edge_scroll_screen_delta(790.0, 100.0, 800, 600, 16.0, 640.0, 0.1);
    assert!(dx_r < 0.0);
    let (dx_mid, dy_mid) = edge_scroll_screen_delta(400.0, 300.0, 800, 600, 16.0, 640.0, 0.1);
    assert_eq!((dx_mid, dy_mid), (0.0, 0.0));
}

#[test]
fn keyboard_pan_uses_dt_not_discrete_steps() {
    let keys = CameraPanKeys { left: true, ..CameraPanKeys::default() };
    let (dx, dy) = keyboard_pan_screen_delta(keys, 640.0, 1.0 / 60.0);
    assert!((dx - 640.0 / 60.0).abs() < 1e-3);
    assert_eq!(dy, 0.0);
    let both = CameraPanKeys { left: true, right: true, ..CameraPanKeys::default() };
    assert_eq!(keyboard_pan_screen_delta(both, 640.0, 0.1), (0.0, 0.0));
}

#[test]
fn edge_scroll_works_on_sidebar_and_command_bar() {
    // 右栏内侧靠窗右缘：应向东滚。
    let (dx, dy) = edge_scroll_screen_delta(1270.0, 200.0, 1280, 720, 16.0, 640.0, 0.1);
    assert!(dx < 0.0, "sidebar right edge must scroll");
    assert_eq!(dy, 0.0);
    // 底边命令条：应向南滚。
    let (dx2, dy2) = edge_scroll_screen_delta(400.0, 710.0, 1280, 720, 16.0, 640.0, 0.1);
    assert_eq!(dx2, 0.0);
    assert!(dy2 < 0.0, "command bar bottom edge must scroll");
}

#[test]
fn edge_scroll_ignores_cursor_outside_window() {
    let (dx, dy) = edge_scroll_screen_delta(900.0, 100.0, 800, 600, 16.0, 640.0, 0.1);
    assert_eq!((dx, dy), (0.0, 0.0));
}

#[test]
fn south_blocked_when_cannot_scroll_south() {
    let cur = edge_scroll_cursor_for(false, false, false, true, true, true, true, false);
    assert_eq!(cur, EdgeScrollCursor::Blocked(EdgeScrollDir::South));
    let cur_ok = edge_scroll_cursor_for(false, false, false, true, true, true, true, true);
    assert_eq!(cur_ok, EdgeScrollCursor::Scroll(EdgeScrollDir::South));
}

#[test]
fn edge_scroll_overrides_context_pointer() {
    assert_eq!(BattlePointer::resolve(EdgeScrollCursor::Default, BattlePointer::Deploy), BattlePointer::Deploy);
    assert_eq!(
        BattlePointer::resolve(EdgeScrollCursor::Scroll(EdgeScrollDir::East), BattlePointer::Deploy),
        BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::East))
    );
    assert_eq!(BattlePointer::resolve(EdgeScrollCursor::Default, BattlePointer::Attack), BattlePointer::Attack);
}

#[test]
fn right_click_map_cancels_tools_or_deselects_not_stop() {
    assert_eq!(classify_right_click_map(true), RightClickMapOutcome::CancelToolModes);
    assert_eq!(classify_right_click_map(false), RightClickMapOutcome::Deselect);
}

#[test]
fn order_click_modifier_ctrl_beats_alt() {
    assert_eq!(OrderClickModifier::from_keys(false, false), OrderClickModifier::None);
    assert_eq!(OrderClickModifier::from_keys(true, false), OrderClickModifier::ForceAttack);
    assert_eq!(OrderClickModifier::from_keys(false, true), OrderClickModifier::ForceMove);
    assert_eq!(OrderClickModifier::from_keys(true, true), OrderClickModifier::ForceAttack);
}

#[test]
fn selected_mobile_blocks_cell_neighbor_friendly_pick() {
    assert!(!allow_cell_neighbor_friendly_pick(true));
    assert!(allow_cell_neighbor_friendly_pick(false));
}

#[test]
fn selected_mobile_blocks_friendly_image_soft_pick() {
    // 有机动选中时禁止车身软命中，否则点邻矿被吞成重选。
    assert!(!allow_friendly_image_soft_pick(true));
    assert!(allow_friendly_image_soft_pick(false));
}

#[test]
fn soft_hit_selected_harvester_on_neighbor_ore_orders_not_reselects() {
    // 矿车在 (5,5)，左键点邻格矿 (6,5)：车身软命中仍摸到单位，但必须下移动 / 采集令。
    assert!(friendly_soft_hit_should_order_not_reselect(true, Some((6, 5)), Some((5, 5))));
    // 落点就是单位格：点选 / 部署，不改下令。
    assert!(!friendly_soft_hit_should_order_not_reselect(true, Some((5, 5)), Some((5, 5))));
    // 无机动选中：走点选。
    assert!(!friendly_soft_hit_should_order_not_reselect(false, Some((6, 5)), Some((5, 5))));
}

#[test]
fn interaction_mode_is_mutually_exclusive() {
    let place = BattleInteractionMode::PlaceBuilding { type_id: "GACNST".into() };
    assert_eq!(place.place_type_id(), Some("GACNST"));
    assert!(place.is_tool());
    assert!(!place.is_repair());
    assert!(BattleInteractionMode::Repair.is_repair());
    assert!(BattleInteractionMode::Sell.is_sell());
    assert!(BattleInteractionMode::Planning.is_planning());
    assert!(BattleInteractionMode::AttackMove.is_attack_move());
    assert!(BattleInteractionMode::Follow.is_follow());
    assert!(!BattleInteractionMode::Normal.is_tool());
}

#[test]
fn surface_metrics_cursor_scales_with_factor() {
    let metrics =
        BattleSurfaceMetrics { logical_width: 800, logical_height: 600, physical_width: 1600, physical_height: 1200, scale_factor: 2.0 };
    let (x, y) = metrics.cursor_from_physical(winit::dpi::PhysicalPosition::new(200.0, 100.0));
    assert!((x - 100.0).abs() < 1e-6);
    assert!((y - 50.0).abs() < 1e-6);
    let (px, py, pw, ph) = metrics.layout_rect_to_physical(10, 20, 100, 50);
    assert_eq!((px, py, pw, ph), (20, 40, 200, 100));
}

#[test]
fn resolved_hover_carries_pointer_and_cell() {
    let hover = ResolvedBattleHover { recommended_pointer: BattlePointer::Move, cell: Some((12, 34)) };
    assert_eq!(hover.recommended_pointer, BattlePointer::Move);
    assert_eq!(hover.cell, Some((12, 34)));
    let with_edge = BattlePointer::resolve(EdgeScrollCursor::Scroll(EdgeScrollDir::East), hover.recommended_pointer);
    assert_eq!(with_edge, BattlePointer::Edge(EdgeScrollCursor::Scroll(EdgeScrollDir::East)));
}
