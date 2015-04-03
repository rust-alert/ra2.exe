//! 集成测试：遭遇战大厅配置循环。

use ra_widgets::skirmish_setup::{
    ALPHA_AI_HOUSE, ALPHA_BOOT_MAP, ALPHA_HUMAN_HOUSE, ALPHA_MATCH_SEED, ALPHA_STARTING_CREDITS, LOBBY_DIFFICULTIES, LOBBY_SIDES,
    SkirmishBootRequest, load_screen_art_suffix, load_screen_background_shp, load_screen_preferred_pal,
};

#[test]
fn alpha_fixed_pins_map_houses_seed_and_credits() {
    let req = SkirmishBootRequest::alpha_fixed();
    assert_eq!(req.preferred_map.as_deref(), Some(ALPHA_BOOT_MAP));
    assert_eq!(req.side, ALPHA_HUMAN_HOUSE);
    assert_eq!(req.row_side(0), ALPHA_HUMAN_HOUSE);
    assert_eq!(req.row_side(1), ALPHA_AI_HOUSE);
    assert_eq!(req.match_seed, ALPHA_MATCH_SEED);
    assert_eq!(req.credits, ALPHA_STARTING_CREDITS);
    assert_eq!(req.alpha_houses(), (ALPHA_HUMAN_HOUSE, ALPHA_AI_HOUSE));
}

#[test]
fn default_lobby_matches_alpha_fixed() {
    assert_eq!(SkirmishBootRequest::default_lobby(), SkirmishBootRequest::alpha_fixed());
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
