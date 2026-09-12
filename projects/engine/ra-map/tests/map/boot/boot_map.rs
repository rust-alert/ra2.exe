//! 自顶层 `boot_map.rs`。

use ra_map::{
    find_boot_map, find_first_boot_map, list_parseable_boot_maps, list_parseable_maps_from_missions_pkt, list_parseable_maps_from_names,
};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};
use std::collections::HashMap;

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

struct MemSource {
    files: HashMap<String, Vec<u8>>,
}
impl AssetSource for MemSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files.get(&relative.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_source_yields_no_boot_map() {
    assert!(find_first_boot_map(GameEdition::Ra2, &EmptySource).is_none());
    assert!(list_parseable_boot_maps(GameEdition::Ra2, &EmptySource).is_empty());
}

#[test]
fn list_parseable_maps_from_names_keeps_order_and_skips_bad() {
    let good = b"[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n[Basic]\nName=t\n";
    let source = MemSource {
        files: HashMap::from([("a.map".into(), good.to_vec()), ("bad.map".into(), b"not ini".to_vec()), ("c.map".into(), good.to_vec())]),
    };
    let listed = list_parseable_maps_from_names(GameEdition::Ra2, &source, ["missing.map", "a.map", "bad.map", "c.map"]);
    assert_eq!(listed.iter().map(|m| m.file_name.as_str()).collect::<Vec<_>>(), vec!["a.map", "c.map"]);
}

#[test]
fn list_parseable_maps_from_missions_pkt_keeps_multimaps_source_order() {
    let map_body = b"[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n[Basic]\nName=t\n";
    let pkt = b"\
[MultiMaps]\n\
1=MP02T2\n\
2=MP06T2\n\
3=MP01T4\n\
[MP02T2]\n\
Description=DESC:MP02T2\n\
GameMode=standard\n\
[MP06T2]\n\
Description=DESC:MP06T2\n\
GameMode=standard, meatgrind\n\
[MP01T4]\n\
Description=DESC:MP01T4\n\
";
    let source = MemSource {
        files: HashMap::from([
            ("mp02t2.map".into(), map_body.to_vec()),
            ("mp06t2.map".into(), map_body.to_vec()),
            // mp01t4 故意缺失：应跳过且不打乱其余源序
            ("mp99t4.map".into(), map_body.to_vec()),
        ]),
    };
    let listed = list_parseable_maps_from_missions_pkt(GameEdition::Ra2, &source, pkt);
    assert_eq!(listed.iter().map(|m| m.file_name.as_str()).collect::<Vec<_>>(), vec!["mp02t2.map", "mp06t2.map"]);
    assert_eq!(listed[0].name_csf, "DESC:MP02T2");
    assert_eq!(
        listed[1].game_modes,
        vec![ra_types::GameModeName::parse("standard"), ra_types::GameModeName::parse("meatgrind")]
    );
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
fn boot_map_name_csf_from_file_stem() {
    use ra_map::{boot_map_name_csf_key, resolve_boot_map_name_csf};
    assert_eq!(boot_map_name_csf_key("mp03t4.map"), "DESC:MP03T4");
    assert_eq!(boot_map_name_csf_key("MP01T4.MAP"), "DESC:MP01T4");
    assert_eq!(resolve_boot_map_name_csf("mp03t4.map", ""), "DESC:MP03T4");
    assert_eq!(resolve_boot_map_name_csf("custom.map", "DESC:CUSTOM"), "DESC:CUSTOM");
    assert_eq!(resolve_boot_map_name_csf("custom.map", "  DESC:FOO  "), "DESC:FOO");
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
