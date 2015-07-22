//! 集成测试：遭遇战大厅配置循环。

use ra_widgets::skirmish_setup::{
    LOBBY_DIFFICULTIES, LOBBY_SIDES, SkirmishBootRequest, load_screen_art_suffix, load_screen_background_shp, load_screen_preferred_pal,
    sidebar_chrome_mix,
};

#[test]
fn default_lobby_leaves_map_and_seed_open() {
    let lobby = SkirmishBootRequest::default_lobby();
    assert!(lobby.preferred_map.is_none());
    assert_eq!(lobby.match_seed, 0);
    assert_eq!(lobby.side, LOBBY_SIDES[0]);
    assert_eq!(lobby.row_side(1), "French");
}

#[test]
fn houses_to_ensure_follows_lobby_rows() {
    let req = SkirmishBootRequest::default_lobby();
    assert_eq!(req.houses_to_ensure(0), vec!["Americans"]);
    assert_eq!(req.houses_to_ensure(1), vec!["Americans", "French"]);
}

#[test]
fn cycle_side_wraps() {
    let mut req = SkirmishBootRequest::default_lobby();
    assert_eq!(req.side, LOBBY_SIDES[0]);
    for expected in LOBBY_SIDES.iter().skip(1) {
        req.cycle_side();
        assert_eq!(req.side, *expected);
    }
    req.cycle_side();
    assert_eq!(req.side, LOBBY_SIDES[0]);
}

#[test]
fn cycle_difficulty_advances() {
    let mut req = SkirmishBootRequest::default_lobby();
    assert_eq!(req.difficulty, LOBBY_DIFFICULTIES[1]);
    req.cycle_difficulty();
    assert_eq!(req.difficulty, LOBBY_DIFFICULTIES[2]);
    req.cycle_difficulty();
    assert_eq!(req.difficulty, LOBBY_DIFFICULTIES[0]);
}

#[test]
fn load_screen_maps_lobby_sides_to_country_art() {
    assert_eq!(load_screen_art_suffix("Americans"), "ustates");
    assert_eq!(load_screen_art_suffix("French"), "france");
    // 原版装载页共用 `mpls.pal`，不按国家换 `mplsf` 等。
    assert_eq!(load_screen_preferred_pal("French"), "mpls.pal");
    assert_eq!(load_screen_background_shp("Americans", 1024), "ls800ustates.shp");
    assert_eq!(load_screen_background_shp("British", 640), "ls640ukingdom.shp");
}

#[test]
fn sidebar_chrome_mix_splits_allied_and_soviet() {
    assert_eq!(sidebar_chrome_mix("Americans"), "sidec01.mix");
    assert_eq!(sidebar_chrome_mix("French"), "sidec01.mix");
    assert_eq!(sidebar_chrome_mix("Germans"), "sidec01.mix");
    assert_eq!(sidebar_chrome_mix("British"), "sidec01.mix");
    assert_eq!(sidebar_chrome_mix("Russians"), "sidec02.mix");
    assert_eq!(sidebar_chrome_mix("Iraq"), "sidec02.mix");
    assert_eq!(sidebar_chrome_mix("Yuri"), "sidec02.mix");
}
