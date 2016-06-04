//! 集成测试：遭遇战大厅配置循环。

use ra_widgets::skirmish_setup::{
    LOBBY_DIFFICULTIES, SkirmishBootRequest, UiFactionChrome, eva_fallback_sample_names, eva_voice_stem_prefix,
    load_screen_background_shp_resolved, load_screen_brief_csf_key, load_screen_palette_resolved, pick_side_flag_pcx,
    score_screen_background_shp, score_screen_palette,
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
fn load_screen_uses_explicit_names_only() {
    assert_eq!(
        load_screen_background_shp_resolved(800, Some("ls800haihead.shp")).as_deref(),
        Some("ls800haihead.shp")
    );
    assert_eq!(
        load_screen_background_shp_resolved(640, Some("ls800haihead.shp")).as_deref(),
        Some("ls640haihead.shp")
    );
    assert_eq!(load_screen_background_shp_resolved(800, None), None);
    assert_eq!(
        load_screen_palette_resolved(Some("mplshh.pal"), |_| true).as_deref(),
        Some("mplshh.pal")
    );
    assert_eq!(
        load_screen_palette_resolved(None, |n| n == "mpls.pal").as_deref(),
        Some("mpls.pal")
    );
    assert_eq!(
        load_screen_brief_csf_key("Guild1", Some("STT:PlayerSideHaihead")),
        "STT:PlayerSideHaihead"
    );
    assert_eq!(load_screen_brief_csf_key("Americans", None), "LOADBRIEF:Americans");
    assert_eq!(pick_side_flag_pcx(&["haih.pcx"], |_| Some(1)), Some("haih.pcx"));
    assert_eq!(pick_side_flag_pcx(&[], |_| Some(1)), None);
}

#[test]
fn ui_faction_chrome_is_open_by_mix_index_only() {
    // 无 Side chrome 时为 None，不静默回退 sidec01。
    assert!(UiFactionChrome::resolve(None).is_none());

    let sidec01 = UiFactionChrome::from_mix_index(1, false);
    assert_eq!(sidec01.sidebar_mix(), "sidec01.mix");
    assert_eq!(sidec01.radar_shp(), "radar.shp");
    assert_eq!(sidec01.radar_pal(), "sidebar.pal");

    let yuri_pack = UiFactionChrome::from_mix_index(2, true);
    assert_eq!(yuri_pack.sidebar_mix(), "sidec02.mix");
    assert_eq!(yuri_pack.radar_shp(), "radary.shp");
    assert_eq!(yuri_pack.radar_pal(), "radaryuri.pal");

    // MO ThirdSide：`MixFileIndex=3` 且未开 YuriFileNames，侧栏包却只有 `radary*`。
    // 候选表必须仍能落到第二套文件名，否则雷达槽留黑。
    let epsilon = UiFactionChrome::from_mix_index(3, false);
    assert_eq!(epsilon.sidebar_mix(), "sidec03.mix");
    assert_eq!(epsilon.radar_shp(), "radar.shp");
    assert_eq!(
        epsilon.radar_shp_pal_candidates()[1],
        ("radary.shp", "radaryuri.pal")
    );

    // 任意多阵营：index 5 / 6 直接生成 sidecNN。
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
        Some("Foehn".into()),
        Some(false),
    )
    .expect("index 6");
    assert_eq!(sixth.sidebar_mix(), "sidec06.mix");
    assert_eq!(sixth.radar_shp(), "radary.shp");
    assert_eq!(sixth.score_background_candidates()[0], "mpxscrnl.shp");
    assert_eq!(sixth.score_palette_candidates()[0], "mpxscrn.pal");
    assert_eq!(sixth.eva_sample_keys()[0], "Foehn");
    // 无显式结算键时：候选为空（不猜名字）。
    assert!(score_screen_background_shp(&fifth).is_empty());
    assert!(score_screen_palette(&fifth).is_empty());

    let from_def = UiFactionChrome::from_side_chrome(&ra_assets::SideChromeDef {
        id: "FifthSide".into(),
        mix_file_index: Some(5),
        yuri_file_names: false,
        score_background: Some("mpxscrnl.shp".into()),
        score_palette: Some("mpxscrn.pal".into()),
        eva_tag: Some("CustomEva".into()),
        score_stats_shade: Some(false),
    })
    .expect("side chrome");
    assert_eq!(from_def.mix_file_index, 5);
    assert_eq!(from_def.score_background_shp(), "mpxscrnl.shp");
    assert_eq!(from_def.eva_tag.as_deref(), Some("CustomEva"));
    assert!(!from_def.score_stats_shade);
}

#[test]
fn eva_fallback_stems_follow_tag_not_allied_first() {
    assert_eq!(eva_voice_stem_prefix("Allied"), Some("ceva"));
    assert_eq!(eva_voice_stem_prefix("Russian"), Some("csof"));
    assert_eq!(eva_voice_stem_prefix("Yuri"), Some("cyur"));
    assert_eq!(eva_voice_stem_prefix("Foehn"), None);

    assert_eq!(
        eva_fallback_sample_names("EVA_BattleControlTerminated", Some("Yuri")),
        vec!["cyur015".to_string(), "CYUR015".to_string()]
    );
    assert_eq!(
        eva_fallback_sample_names("EVA_MissionAccomplished", Some("Russian")),
        vec!["csof013".to_string(), "CSOF013".to_string()]
    );
    assert!(eva_fallback_sample_names("EVA_BattleControlTerminated", Some("Foehn")).is_empty());
    assert!(eva_fallback_sample_names("EVA_UnitLost", Some("Allied")).is_empty());
}
