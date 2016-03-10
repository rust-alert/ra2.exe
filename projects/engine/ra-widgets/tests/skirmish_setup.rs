//! 集成测试：遭遇战大厅配置循环。

use ra_widgets::skirmish_setup::{
    LOBBY_DIFFICULTIES, SkirmishBootRequest, UiFactionChrome, load_screen_art_suffix, load_screen_background_shp, load_screen_background_shp_resolved,
    load_screen_brief_suffix, load_screen_palette, load_screen_palette_resolved, load_screen_preferred_pal, score_screen_background_shp,
    score_screen_palette, sidebar_chrome_mix, sidebar_chrome_mix_candidates, sidebar_chrome_mix_for_faction,
    sidebar_radar_pal, sidebar_radar_shp,
};

fn sample_sides() -> Vec<String> {
    ["Americans", "French", "Germans", "British", "Russians"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn lobby_with_sides() -> SkirmishBootRequest {
    let mut lobby = SkirmishBootRequest::default_lobby();
    lobby.set_lobby_sides(sample_sides());
    lobby
}

#[test]
fn default_lobby_leaves_map_and_seed_open() {
    let lobby = lobby_with_sides();
    assert!(lobby.preferred_map.is_none());
    assert_eq!(lobby.match_seed, 0);
    assert_eq!(lobby.side, "Americans");
    assert_eq!(lobby.row_side(1), "French");
}

#[test]
fn houses_to_ensure_follows_lobby_rows() {
    let req = lobby_with_sides();
    assert_eq!(req.houses_to_ensure(0), vec!["Americans".to_string()]);
    assert_eq!(req.houses_to_ensure(1), vec!["Americans".to_string(), "French".to_string()]);
}

#[test]
fn cycle_side_wraps() {
    let mut req = lobby_with_sides();
    assert_eq!(req.side, "Americans");
    for expected in sample_sides().into_iter().skip(1) {
        req.cycle_side();
        assert_eq!(req.side, expected);
    }
    req.cycle_side();
    assert_eq!(req.side, "Americans");
}

#[test]
fn set_lobby_sides_preserves_selected_names() {
    let mut req = lobby_with_sides();
    req.set_row_side(0, 4);
    assert_eq!(req.side, "Russians");
    req.set_lobby_sides(vec![
        "Americans".into(),
        "Russians".into(),
        "French".into(),
        "Korea".into(),
    ]);
    assert_eq!(req.side, "Russians");
    assert_eq!(req.row_side(0), "Russians");
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
    assert_eq!(load_screen_art_suffix("YuriCountry"), "yuri");
    // YR 国家盘优先；RA2 缺盘时由 `load_screen_palette` 回退 `mpls.pal`。
    assert_eq!(load_screen_preferred_pal("French"), "mplsf.pal");
    assert_eq!(load_screen_preferred_pal("YuriCountry"), "mpyls.pal");
    assert_eq!(load_screen_palette("French", |_| false), "mplsf.pal");
    assert_eq!(load_screen_palette("French", |n| n == "mpls.pal"), "mpls.pal");
    assert_eq!(load_screen_palette("YuriCountry", |n| n == "mpyls.pal"), "mpyls.pal");
    assert_eq!(load_screen_brief_suffix("YuriCountry"), "YuriCountry");
    assert_eq!(load_screen_brief_suffix("Americans"), "USA");
    assert_eq!(load_screen_background_shp("Americans", 1024), "ls800ustates.shp");
    assert_eq!(load_screen_background_shp("British", 640), "ls640ukingdom.shp");
    assert_eq!(load_screen_background_shp("YuriCountry", 800), "ls800yuri.shp");
    assert_eq!(
        load_screen_background_shp_resolved("Guild1", 800, Some("ls800haihead.shp")),
        "ls800haihead.shp"
    );
    assert_eq!(
        load_screen_palette_resolved("Guild1", Some("mplshh.pal"), |_| true),
        "mplshh.pal"
    );
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
    assert_eq!(sidebar_chrome_mix_for_faction("GDI"), "sidec01.mix");
    assert_eq!(sidebar_chrome_mix_for_faction("Nod"), "sidec02.mix");
    assert_eq!(sidebar_chrome_mix_for_faction("ThirdSide"), "sidec02.mix");
    assert_eq!(
        sidebar_chrome_mix_candidates("YuriCountry"),
        vec!["sidec02md.mix".to_string(), "sidec02.mix".to_string()]
    );
    assert_eq!(
        sidebar_chrome_mix_candidates("Americans"),
        vec!["sidec01md.mix".to_string(), "sidec01.mix".to_string()]
    );
    assert_eq!(sidebar_radar_shp("YuriCountry"), "radary.shp");
    assert_eq!(sidebar_radar_shp("Russians"), "radar.shp");
    assert_eq!(sidebar_radar_pal("YuriCountry"), "radaryuri.pal");
    assert_eq!(sidebar_radar_pal("Russians"), "sidebar.pal");
}

#[test]
fn ui_faction_chrome_is_open_by_mix_index() {
    assert_eq!(UiFactionChrome::from_country("Americans").mix_file_index, 1);
    assert_eq!(UiFactionChrome::from_country("Russians").mix_file_index, 2);
    assert!(UiFactionChrome::from_country("YuriCountry").yuri_file_names);
    assert!(UiFactionChrome::from_faction_id("ThirdSide").yuri_file_names);
    assert_eq!(UiFactionChrome::from_faction_id("Nod").mix_file_index, 2);
    assert_eq!(score_screen_background_shp("YuriCountry"), "mpyscrnl.shp");
    assert_eq!(score_screen_palette("YuriCountry"), "mpyscrn.pal");
    assert_eq!(score_screen_background_shp("Russians"), "mpsscrnl.shp");
    assert_eq!(score_screen_background_shp("Americans"), "mpascrnl.shp");
    // 模组未知国名：靠 Side= 库存默认；真正扩展族靠 MixFileIndex。
    let custom_psi = UiFactionChrome::resolve("CustomPsi", Some("ThirdSide"), None);
    assert_eq!(custom_psi.mix_file_index, 2);
    assert!(custom_psi.yuri_file_names);
    assert_eq!(UiFactionChrome::resolve("CustomNod", Some("Nod"), None).mix_file_index, 2);
    assert_eq!(UiFactionChrome::resolve("CustomGdi", Some("GDI"), None).mix_file_index, 1);
    assert_eq!(UiFactionChrome::resolve("TotallyUnknown", None, None).mix_file_index, 1);
    // 任意多阵营：index 5 / 6 直接生成 sidecNN，不写死族名。
    let fifth = UiFactionChrome::from_mix_index(5, false);
    assert_eq!(fifth.sidebar_mix(), "sidec05.mix");
    assert_eq!(
        fifth.sidebar_mix_candidates(),
        vec!["sidec05md.mix".to_string(), "sidec05.mix".to_string()]
    );
    let sixth = UiFactionChrome::from_side_keys(
        Some(6),
        true,
        Some("mpxscrnl.shp".into()),
        Some("mpxscrn.pal".into()),
    )
    .expect("index 6");
    assert_eq!(sixth.sidebar_mix(), "sidec06.mix");
    assert_eq!(sixth.radar_shp(), "radary.shp");
    assert_eq!(sixth.score_background_candidates()[0], "mpxscrnl.shp");
    assert_eq!(sixth.score_palette_candidates()[0], "mpxscrn.pal");
    let from_def = UiFactionChrome::from_side_chrome(&ra_assets::SideChromeDef {
        id: "FifthSide".into(),
        mix_file_index: Some(5),
        yuri_file_names: false,
        score_background: Some("mpxscrnl.shp".into()),
        score_palette: Some("mpxscrn.pal".into()),
    })
    .expect("side chrome");
    assert_eq!(from_def.mix_file_index, 5);
    assert_eq!(from_def.score_background_shp(), "mpxscrnl.shp");
}
