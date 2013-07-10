use ra_map::find_first_boot_map;
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
