//! 自顶层 `waypoints.rs`。

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

// 自顶层 `waypoints_unit.rs` 并入。

// 自 engine/ra-map/src/waypoints.rs :: tests
use ra_map::waypoints::*;

#[test]
fn skirmish_start_waypoint_matches_slot_index() {
    let doc = IniDocument::parse(b"[Waypoints]\n0=5002\n1=8005\n8=100\n").expect("ini");
    let wps = parse_waypoints(&doc);
    assert_eq!(skirmish_start_waypoint(&wps, 0), Some(Waypoint { index: 0, x: 2, y: 5 }));
    assert_eq!(skirmish_start_waypoint(&wps, 1), Some(Waypoint { index: 1, x: 5, y: 8 }));
    assert_eq!(skirmish_start_waypoint(&wps, 2), None);
    assert_eq!(skirmish_start_waypoint(&wps, 8), Some(Waypoint { index: 8, x: 100, y: 0 }));
}
