//! 自 `engine/ra-layout/src/reference/battle_hud.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/battle_hud.rs :: tests
use ra_layout::{reference::battle_hud::*, shell::rect_px_from_snapshot};

#[test]
fn battle_hud_snap_matches_computed_rects() {
    let metrics = BattleHudChromeMetrics::sidec01();
    for (vw, vh) in [(640u32, 480u32), (800, 600), (1280, 720), (2560, 1440)] {
        let snap = solve_battle_hud_with_metrics(vw, vh, metrics);
        let expected = compute_battle_hud_rects(vw, vh, metrics);
        for (id, cell) in [
            ("sidebar", expected.sidebar),
            ("credits", expected.credits),
            ("top", expected.top),
            ("radar", expected.radar),
            ("side1", expected.side1),
            ("cameo_band", expected.cameo_band),
            ("side3", expected.side3),
            ("addon", expected.addon),
            ("repair", expected.repair),
            ("sell", expected.sell),
            ("tab00", expected.tabs[0]),
            ("tab01", expected.tabs[1]),
            ("tab02", expected.tabs[2]),
            ("tab03", expected.tabs[3]),
            ("bottom_strip", expected.bottom_strip),
            ("command_bar", expected.command_bar),
            ("lendcap", expected.lendcap),
            ("rendcap", expected.rendcap),
            ("opt_btn", expected.opt_btn),
            ("diplo_btn", expected.diplo_btn),
        ] {
            let got = snap.get(id).expect(id).layout.rect;
            assert_eq!(got.x as i32, cell.x as i32, "{vw}x{vh} {id} x");
            assert_eq!(got.y as i32, cell.y as i32, "{vw}x{vh} {id} y");
            assert_eq!(got.width as i32, cell.width as i32, "{vw}x{vh} {id} w");
            assert_eq!(got.height as i32, cell.height as i32, "{vw}x{vh} {id} h");
        }
        for (id, cell) in COMMAND_BAR_BUTTON_IDS.iter().zip(expected.cmd_buttons.iter()) {
            let got = snap.get(id).expect(id).layout.rect;
            assert_eq!(got.x as i32, cell.x as i32, "{vw}x{vh} {id} x");
            assert_eq!(got.width as i32, cell.width as i32, "{vw}x{vh} {id} w");
        }
        let world = battle_hud_world_viewport(&snap);
        assert_eq!(world.w, expected.sidebar.x as i32);
        assert_eq!(world.h, expected.command_bar.y as i32);
    }
}

#[test]
fn battle_hud_command_bar_spans_tactical_bottom() {
    let sidec01 = BattleHudChromeMetrics::sidec01();
    let r = compute_battle_hud_rects(800, 600, sidec01);
    assert_eq!(r.sidebar.x as i32, 800 - 168);
    assert_eq!(r.sidebar.width as i32, 168);
    assert_eq!(r.sidebar.height as i32, 600);
    // 右栏底脚仍只在侧栏内。
    assert_eq!(r.bottom_strip.x as i32, r.sidebar.x as i32);
    assert_eq!(r.bottom_strip.width as i32, 168);
    // 命令条横贯战术区底边，右缘贴侧栏左缘。
    assert_eq!(r.command_bar.x as i32, 0);
    assert_eq!(r.command_bar.y as i32, 600 - COMMAND_BAR_H);
    assert_eq!(r.command_bar.width as i32, r.sidebar.x as i32);
    assert_eq!(r.command_bar.height as i32, COMMAND_BAR_H);
    assert_eq!(r.lendcap.width as i32, COMMAND_LENDCAP_W);
    assert_eq!(r.rendcap.width as i32, COMMAND_RENDCAP_W);
    assert_eq!(r.cmd_buttons[0].x as i32, COMMAND_LENDCAP_W);
    assert_eq!(r.cmd_buttons[0].width as i32, COMMAND_BUTTON_W);
    assert_eq!(r.cmd_buttons[1].x as i32, COMMAND_LENDCAP_W + COMMAND_BUTTON_W);
    // 选项 / 外交在资金条下的顶栏双槽，不在战术区左下、也不在底脚。
    assert!(r.opt_btn.x as i32 >= r.sidebar.x as i32);
    assert!(r.diplo_btn.x as i32 >= r.sidebar.x as i32);
    assert!(r.opt_btn.y as i32 >= r.top.y as i32);
    assert!(r.diplo_btn.y as i32 >= r.top.y as i32);
    assert!(r.opt_btn.y as i32 + r.opt_btn.height as i32 <= r.top.y as i32 + r.top.height as i32);
    assert!(r.diplo_btn.y as i32 + r.diplo_btn.height as i32 <= r.top.y as i32 + r.top.height as i32);
    // 修理 / 出售在 side1 带内。
    assert!(r.repair.y as i32 >= r.side1.y as i32);
    assert!(r.sell.y as i32 >= r.side1.y as i32);
    assert_eq!(r.credits.height as i32, CREDITS_H);
    assert_eq!(r.top.height as i32, TOP_H);
    assert_eq!(r.radar.height as i32, RADAR_H);
    assert_eq!(r.side1.height as i32, SIDE1_H);
    assert_eq!(r.side3.height as i32, SIDE3_H);
    assert_eq!(r.addon.height as i32, ADDON_H);
    assert_eq!(r.opt_btn.width as i32, sidec01.top_btn_w);
    assert_eq!(r.opt_btn.height as i32, sidec01.top_btn_h);
    assert_eq!(r.diplo_btn.width as i32, sidec01.top_btn_w);
    assert_eq!(r.repair.width as i32, sidec01.repair_sell_w);
    assert_eq!(r.repair.height as i32, sidec01.repair_sell_h);
    assert_eq!(r.tabs[0].width as i32, sidec01.tab_w);
    assert_eq!(r.tabs[0].height as i32, sidec01.tab_h);
}

#[test]
fn sidec01_and_sidec02_chrome_metrics_differ() {
    let sidec01 = BattleHudChromeMetrics::sidec01();
    let sidec02 = BattleHudChromeMetrics::sidec02();
    assert_ne!(sidec01.repair_sell_w, sidec02.repair_sell_w);
    assert_ne!(sidec01.repair_sell_h, sidec02.repair_sell_h);
    assert_ne!(sidec01.repair_x, sidec02.repair_x);
    // 盟军 / 苏军页签画布同为 32×28，差在修理钮与顶栏等。
    assert_eq!(sidec01.tab_w, sidec02.tab_w);
    assert_eq!(sidec01.tab_h, sidec02.tab_h);
    assert_ne!(sidec01.top_btn_h, sidec02.top_btn_h);
    assert_ne!(sidec01.power_w, sidec02.power_w);

    let a = compute_battle_hud_rects(800, 600, sidec01);
    let s = compute_battle_hud_rects(800, 600, sidec02);
    assert_eq!(a.repair.width as i32, 64);
    assert_eq!(a.repair.height as i32, 31);
    assert_eq!(s.repair.width as i32, 52);
    assert_eq!(s.repair.height as i32, 32);
    assert_eq!(a.repair.x as i32 - a.sidebar.x as i32, sidec01.repair_x);
    assert_eq!(s.repair.x as i32 - s.sidebar.x as i32, sidec02.repair_x);
    assert_eq!(a.tabs[0].width as i32, 32);
    assert_eq!(a.tabs[0].height as i32, 28);
    assert_eq!(s.tabs[0].width as i32, 32);
    assert_eq!(s.tabs[0].height as i32, 28);
    assert_eq!(a.opt_btn.height as i32, 18);
    assert_eq!(s.opt_btn.height as i32, 22);
    assert_eq!(a.tabs[0].x as i32 - a.sidebar.x as i32, sidec01.tab_x);
    assert_eq!(s.tabs[0].x as i32 - s.sidebar.x as i32, sidec02.tab_x);
    // 四页签并排不越出侧栏。
    let a_last = a.tabs[3].x as i32 + a.tabs[3].width as i32;
    assert!(a_last <= a.sidebar.x as i32 + a.sidebar.width as i32);

    assert_eq!(BattleHudChromeMetrics::for_mix("sidec01.mix"), sidec01);
    assert_eq!(BattleHudChromeMetrics::for_mix("sidec02.mix"), sidec02);
    assert_eq!(BattleHudChromeMetrics::for_mix("SIDEC02.MIX"), sidec02);
    assert_eq!(BattleHudChromeMetrics::for_mix_index(4), sidec01);
}

#[test]
fn cameo_grid_two_columns_inside_band() {
    let metrics = BattleHudChromeMetrics::sidec01();
    let snap = solve_battle_hud_with_metrics(800, 600, metrics);
    let band = rect_px_from_snapshot(&snap, "cameo_band");
    let visible = cameo_visible_slot_count(band.h);
    assert!(visible >= 2, "至少两格 cameo · h={}", band.h);
    assert_eq!(visible % 2, 0);
    let a = cameo_slot_rect(band, metrics.power_w, 0).expect("slot0");
    let b = cameo_slot_rect(band, metrics.power_w, 1).expect("slot1");
    assert_eq!(a.w, CAMEO_CELL_W);
    assert_eq!(a.h, CAMEO_CELL_H);
    assert_eq!(b.x, a.x + CAMEO_CELL_W);
    assert_eq!(b.y, a.y);
    assert_eq!(hit_cameo_slot(band, metrics.power_w, a.x + 1, a.y + 1), Some(0));
    assert_eq!(hit_cameo_slot(band, metrics.power_w, b.x + 1, b.y + 1), Some(1));
}
