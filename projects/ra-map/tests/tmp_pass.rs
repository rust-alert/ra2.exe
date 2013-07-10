use ra_map::{MapInfo, PassGrid, seal_pass_grid_from_tmp};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_map_seals_nothing() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut grid = PassGrid::open(4, 4);
    assert_eq!(seal_pass_grid_from_tmp(&EmptySource, &map, &mut grid), 0);
}
