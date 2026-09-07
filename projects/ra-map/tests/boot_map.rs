use ra_map::{find_boot_map, find_first_boot_map, list_parseable_boot_maps};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_source_yields_placeholder_map() {
    let loaded = find_first_boot_map(GameEdition::Ra2, &EmptySource);
    assert_eq!(loaded.map.name, "boot");
    assert!(loaded.note.contains("map:无"));
}

#[test]
fn list_parseable_on_empty_is_empty() {
    assert!(list_parseable_boot_maps(GameEdition::Ra2, &EmptySource).is_empty());
}

#[test]
fn find_boot_map_preferred_falls_back_when_missing() {
    let loaded = find_boot_map(GameEdition::Ra2, &EmptySource, Some("nope.map"));
    assert_eq!(loaded.map.name, "boot");
}
