//! 自顶层 `theater.rs`。

use ra_map::{Theater, new_theater_shp_name};

#[test]
fn temperate_new_theater_name() {
    assert_eq!(new_theater_shp_name("CAMSC01", Theater::Temperate), "ctmsc01.shp");
    assert_eq!(new_theater_shp_name("CAAIRP", Theater::Temperate), "ctairp.shp");
}
