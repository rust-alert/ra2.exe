use ra_map::{MapInfo, compose_terrain_preview};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_cells_yield_none() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    assert!(compose_terrain_preview(&EmptySource, &map).is_none());
}
