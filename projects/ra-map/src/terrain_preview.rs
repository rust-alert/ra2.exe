//! 从 `AssetSource` 合成地图地形预览（不含 overlay / 实体叠画）。

use std::collections::HashMap;

use ra_assets::{Palette, TmpFile};
use ra_types::AssetSource;

use crate::compose::{compose_terrain_rgba, TerrainImage, TileBlit};
use crate::theater::{theater_ini_name, theater_palette, theater_tmp_extension};
use crate::tileset::parse_tileset_ini;
use crate::MapInfo;

/// 用剧院调色板与 TMP 合成等距地形图。
pub fn compose_terrain_preview(
    source: &dyn AssetSource,
    map: &MapInfo,
) -> Option<TerrainImage> {
    if map.cells.is_empty() {
        return None;
    }
    let pal_bytes = source.read(theater_palette(map.theater)).ok()?;
    let pal = Palette::parse(&pal_bytes).ok()?;
    let ini_bytes = source.read(theater_ini_name(map.theater)).ok()?;
    let lookup = parse_tileset_ini(&ini_bytes, theater_tmp_extension(map.theater)).ok()?;

    let mut file_cache: HashMap<String, TmpFile> = HashMap::new();
    let mut blit_cache: HashMap<(i32, u8), TileBlit> = HashMap::new();

    let mut resolve = |tile_num: i32, sub_tile: u8| -> Option<TileBlit> {
        if let Some(blit) = blit_cache.get(&(tile_num, sub_tile)) {
            return Some(blit.clone());
        }
        let name = lookup.filename(tile_num)?.to_string();
        if !file_cache.contains_key(&name) {
            let data = source.read(&name).ok()?;
            let tmp = TmpFile::parse(&data).ok()?;
            file_cache.insert(name.clone(), tmp);
        }
        let tmp = file_cache.get(&name)?;
        let index = usize::from(sub_tile);
        let tile = tmp.tiles.get(index)?.as_ref()?;
        let rgba = tmp.tile_to_rgba(index, &pal).ok()?;
        let blit = TileBlit {
            width: tile.pixel_width,
            height: tile.pixel_height,
            offset_x: tile.offset_x,
            offset_y: tile.offset_y,
            rgba,
        };
        blit_cache.insert((tile_num, sub_tile), blit.clone());
        Some(blit)
    };

    compose_terrain_rgba(&map.cells, &mut resolve)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_types::{GameEdition, RaError, RaResult};

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
}
