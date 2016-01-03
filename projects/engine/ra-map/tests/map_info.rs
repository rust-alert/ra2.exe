use ra_map::{MapInfo, Theater, game_cell_grid_side, map_matches_game_mode_filter};
use ra_types::GameEdition;

#[test]
fn parse_local_size_from_map_ini() {
    let text = b"[Map]\nSize=0,0,105,95\nLocalSize=5,4,95,85\nTheater=TEMPERATE\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
    assert_eq!(info.size_width, 105);
    assert_eq!(info.size_height, 95);
    assert_eq!(info.local_size.left, 5);
    assert_eq!(info.local_size.top, 4);
    assert_eq!(info.local_size.width, 95);
    assert_eq!(info.local_size.height, 85);
}

#[test]
fn missing_local_size_defaults_to_full_size() {
    let text = b"[Map]\nSize=0,0,50,40\nTheater=TEMPERATE\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
    assert_eq!(info.local_size.left, 0);
    assert_eq!(info.local_size.top, 0);
    assert_eq!(info.local_size.width, 50);
    assert_eq!(info.local_size.height, 40);
}

#[test]
fn parse_basic_description_csf() {
    let text = b"[Map]\nSize=0,0,50,40\nTheater=TEMPERATE\n[Basic]\nDescription=DESC:MP03T4\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
    assert_eq!(info.description_csf, "DESC:MP03T4");
}

#[test]
fn parse_basic_next_mission() {
    let text = b"[Map]\nSize=0,0,50,40\nTheater=TEMPERATE\n[Basic]\nNextMission=all02t.map\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
    assert_eq!(info.next_mission, "all02t.map");
}

#[test]
fn parse_basic_map_ini() {
    let text = b"[Map]\nSize=0,0,50,40\nTheater=SNOW\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
    assert_eq!(info.size_width, 50);
    assert_eq!(info.size_height, 40);
    // 游戏格网边长 = height + max(width, height) = 40 + 50 = 90
    assert_eq!(info.width, 90);
    assert_eq!(info.height, 90);
    assert_eq!(info.theater, Theater::Snow);
    assert!(info.cells.is_empty());
    assert!(info.game_modes.is_empty());
}

#[test]
fn parse_basic_game_modes_list() {
    let text = b"[Map]\nSize=0,0,50,40\nTheater=TEMPERATE\n[Basic]\nGameModes=standard, MeatGrind\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
    assert_eq!(info.game_modes, vec!["standard".to_string(), "MeatGrind".to_string()]);
}

#[test]
fn empty_game_modes_match_standard_only() {
    assert!(map_matches_game_mode_filter(&[], "standard"));
    assert!(map_matches_game_mode_filter(&[], "STANDARD"));
    assert!(!map_matches_game_mode_filter(&[], "meatgrind"));
    assert!(!map_matches_game_mode_filter(&[], ""));
}

#[test]
fn listed_game_modes_match_filter_case_insensitively() {
    let modes = vec!["standard".into(), "MeatGrind".into()];
    assert!(map_matches_game_mode_filter(&modes, "meatgrind"));
    assert!(map_matches_game_mode_filter(&modes, "standard"));
    assert!(!map_matches_game_mode_filter(&modes, "duel"));
}

#[test]
fn game_cell_grid_side_covers_diamond_and_legacy_pad() {
    // width<=height：边长 = 2*height
    assert_eq!(game_cell_grid_side(80, 85), 170);
    // width>height：边长 = width+height
    assert_eq!(game_cell_grid_side(100, 60), 160);
}

#[test]
fn mp03t4_style_start_waypoint_fits_game_grid() {
    let text = b"[Map]\nSize=0,0,80,85\nTheater=TEMPERATE\n[Waypoints]\n0=88028\n1=31088\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "mp03t4", text).unwrap();
    assert_eq!(info.width, 170);
    assert_eq!(info.height, 170);
    let wp0 = info.waypoints.iter().find(|w| w.index == 0).unwrap();
    assert_eq!((wp0.x, wp0.y), (28, 88));
    assert!(u32::from(wp0.x) < info.width && u32::from(wp0.y) < info.height);
}
