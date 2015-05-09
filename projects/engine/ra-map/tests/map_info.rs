use ra_map::{MapInfo, Theater, game_cell_grid_side};
use ra_types::GameEdition;

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
