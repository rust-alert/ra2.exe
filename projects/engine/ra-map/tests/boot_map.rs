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
