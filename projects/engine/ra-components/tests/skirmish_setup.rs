//! 集成测试：遭遇战大厅配置循环。

use ra_components::skirmish_setup::{
    LOBBY_DIFFICULTIES, LOBBY_SIDES, SkirmishBootRequest, load_screen_art_suffix, load_screen_background_shp, load_screen_preferred_pal,
};

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
    assert_eq!(load_screen_preferred_pal("French"), "mplsf.pal");
    assert_eq!(load_screen_background_shp("Americans", 1024), "ls800ustates.shp");
    assert_eq!(load_screen_background_shp("British", 640), "ls640ukingdom.shp");
}
