use ra_map::{MapInfo, TerrainImage, paint_map_mobiles};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_mobiles_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage { width: 1, height: 1, pixels: vec![0; 4], drawn: 0, origin_x: 0, origin_y: 0 };
    assert_eq!(paint_map_mobiles(&EmptySource, &map, &mut image, "art.ini", &|p, _| p.clone()), 0);
}
