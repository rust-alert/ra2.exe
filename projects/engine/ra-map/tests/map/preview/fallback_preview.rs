//! 自顶层 `fallback_preview.rs`。

use ra_map::{Theater, load_fallback_theater_tile, load_fallback_unit_sprite};
use ra_types::{AssetSource, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn empty_source_yields_none() {
    assert!(load_fallback_theater_tile(&EmptySource, Theater::Temperate).is_none());
    assert!(load_fallback_unit_sprite(&EmptySource).is_none());
}
