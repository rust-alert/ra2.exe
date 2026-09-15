//! `compose_boot_preview` 失败封闭：不得用单砖/单位 SHP 冒充成功预览。
//! 遭遇战底图不得烤入地图预放机动单位（否则剥实体后仍有鬼影）。

use ra_assets::Palette;
use ra_map::{MapInfo, PaintDefinitionsLoader, StructureLightTable, Theater, compose_boot_preview};
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
    let lights = StructureLightTable::default();
    let source = EmptySource;
    let mut paint = PaintDefinitionsLoader::load_sealed(&source, "art.ini", "rules.ini", &Default::default(), &map);
    assert!(paint.documents_sealed());
    let out = compose_boot_preview(&source, &map, &mut paint, &lights, &|_| None, &|_| false, &|_| None, &identity);
    assert!(out.is_none(), "不得在地形合成失败后仍返回 Some");
}

#[test]
fn skirmish_preview_snapshots_base_before_map_mobiles() {
    // 回归：曾把 `base_without_anims` 放在 `paint_map_mobiles` 之后，遭遇战剥世界实体后
    // 预览底图仍残留动员兵 / 黑鹰等地图预放 SHP 鬼影。
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/skirmish_preview.rs"));
    let base = src.find("let base_without_anims = image.image.clone();").expect("base snapshot");
    let mobiles = src.find("let mobiles = paint_map_mobiles").expect("mobile paint");
    assert!(base < mobiles, "base_without_anims must be snapped before paint_map_mobiles");
}
