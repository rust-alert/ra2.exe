//! `compose_boot_preview` 失败封闭：不得用单砖/单位 SHP 冒充成功预览。

use ra_assets::Palette;
use ra_map::{MapInfo, Theater, compose_boot_preview};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn compose_boot_preview_returns_none_without_silent_fallback() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "no-preview");
    map.width = 8;
    map.height = 8;
    map.theater = Theater::Temperate;
    let identity = |pal: &Palette, _owner: &str| pal.clone();
    let out = compose_boot_preview(&EmptySource, &map, "art.ini", "rules.ini", &|_| None, &|_| false, &|_| None, &identity);
    assert!(out.is_none(), "不得在地形合成失败后仍返回 Some");
}
