//! 地图 `[Terrain]` 物件 SHP 叠画。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile};
use ra_types::AssetSource;

use crate::{
    MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    theater::{theater_palette, theater_tmp_extension},
};

/// 将地形物件叠到合成图上。返回画上的物件数。
pub fn paint_map_terrain_objects(source: &dyn AssetSource, map: &MapInfo, image: &mut TerrainImage, art_ini: &str) -> usize {
    if map.terrain_objects.is_empty() {
        return 0;
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    // `[Terrain]` 物件多为剧院扩展名 SHP（如 `.tem`），须用剧院调色板；
    // `unittem.pal` 仅作缺剧院 pal 时的回退（错用会导致树等呈噪点色）。
    let obj_pal = source
        .read(theater_palette(map.theater))
        .ok()
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok()));
    let Some(obj_pal) = obj_pal
    else {
        return 0;
    };

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<String, TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for obj in &map.terrain_objects {
        let image_key = art.as_ref().and_then(|a| a.get(&obj.name, "Image")).unwrap_or(obj.name.as_str()).to_ascii_uppercase();
        if let Some(blit) = blit_cache.get(&image_key) {
            items.push((obj.x, obj.y, blit.clone()));
            continue;
        }
        let file = format!("{}.{ext}", image_key.to_ascii_lowercase());
        if !shp_cache.contains_key(&file) {
            let Ok(bytes) = source.read(&file)
            else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&bytes)
            else {
                continue;
            };
            shp_cache.insert(file.clone(), shp);
        }
        let Some(shp) = shp_cache.get(&file)
        else {
            continue;
        };
        let Some(frame) = shp.frames.first()
        else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x),
            offset_y: i32::from(frame.frame_y),
            rgba: frame.to_rgba(&obj_pal),
        };
        blit_cache.insert(image_key, blit.clone());
        items.push((obj.x, obj.y, blit));
    }

    paint_cell_sprites(image, &items, z_at)
}
