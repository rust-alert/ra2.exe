use ra_map::{find_boot_map, find_first_boot_map, list_parseable_boot_maps};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_source_yields_no_boot_map() {
    assert!(find_first_boot_map(GameEdition::Ra2, &EmptySource).is_none());
    assert!(list_parseable_boot_maps(GameEdition::Ra2, &EmptySource).is_empty());
}

#[test]
fn find_boot_map_preferred_errors_when_missing() {
    let err = find_boot_map(GameEdition::Ra2, &EmptySource, Some("nope.map")).unwrap_err();
    assert!(err.contains("nope.map"), "{err}");
    assert!(err.contains("不换图"), "{err}");
}

#[test]
fn find_boot_map_auto_errors_when_empty_source() {
    let err = find_boot_map(GameEdition::Ra2, &EmptySource, None).unwrap_err();
    assert!(err.contains("无可用启动地图"), "{err}");
}

#[test]
fn start_slots_from_filename_and_ai_rows() {
    use ra_map::{Waypoint, count_skirmish_start_slots, skirmish_ai_row_count};
    assert_eq!(count_skirmish_start_slots(&[], "mp03t4.map"), 4);
    assert_eq!(count_skirmish_start_slots(&[], "mp01t2.map"), 2);
    assert_eq!(skirmish_ai_row_count(4), 3);
    assert_eq!(skirmish_ai_row_count(2), 1);
    assert_eq!(skirmish_ai_row_count(8), 7);
    let wps = vec![
        Waypoint { index: 0, x: 1, y: 1 },
        Waypoint { index: 1, x: 2, y: 2 },
        Waypoint { index: 2, x: 3, y: 3 },
        Waypoint { index: 98, x: 9, y: 9 },
    ];
    assert_eq!(count_skirmish_start_slots(&wps, "whatever.map"), 3);
}
