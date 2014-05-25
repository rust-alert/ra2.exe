use ra_assets::IniDocument;
use ra_map::{Waypoint, parse_waypoints};

#[test]
fn parse_sorted_waypoints() {
    let doc = IniDocument::parse(b"[Waypoints]\n2=4005\n0=98057\n").unwrap();
    let wp = parse_waypoints(&doc);
    assert_eq!(wp.len(), 2);
    assert_eq!(wp[0], Waypoint { index: 0, x: 57, y: 98 });
    assert_eq!(wp[1], Waypoint { index: 2, x: 5, y: 4 });
}
