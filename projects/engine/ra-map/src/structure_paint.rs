//! 地图建筑放置段 SHP 叠画。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile};
use ra_types::AssetSource;

use crate::{
    MapEntityKind, MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    theater::{new_theater_shp_name, theater_palette},
};

/// 叠画 `[Structures]`。`remap_owner(base, owner)` 返回房屋色调色板。
pub fn paint_map_structures(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> usize {
    let structures: Vec<_> = map.entities.iter().filter(|e| e.kind == MapEntityKind::Structure).collect();
    if structures.is_empty() {
        return 0;
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let obj_pal = source
        .read("unittem.pal")
        .ok()
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| source.read(theater_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok()));
    let Some(obj_pal) = obj_pal
    else {
        return 0;
    };

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, String), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for ent in structures {
        let image_key = art.as_ref().and_then(|a| a.get(&ent.type_id, "Image")).unwrap_or(ent.type_id.as_str()).to_ascii_uppercase();
        let cache_key = (image_key.clone(), ent.owner.clone());
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((ent.x, ent.y, blit.clone()));
            continue;
        }
        let new_theater = art.as_ref().and_then(|a| a.get(&ent.type_id, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let candidates = if new_theater {
            vec![new_theater_shp_name(&image_key, map.theater), format!("{}.shp", image_key.to_ascii_lowercase())]
        }
        else {
            vec![format!("{}.shp", image_key.to_ascii_lowercase()), new_theater_shp_name(&image_key, map.theater)]
        };

        let mut loaded: Option<String> = None;
        for file in &candidates {
            if shp_cache.contains_key(file) {
                loaded = Some(file.clone());
                break;
            }
            let Ok(bytes) = source.read(file)
            else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&bytes)
            else {
                continue;
            };
            shp_cache.insert(file.clone(), shp);
            loaded = Some(file.clone());
            break;
        }
        let Some(file) = loaded
        else {
            continue;
        };
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
        let pal = remap_owner(&obj_pal, &ent.owner);
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x),
            offset_y: i32::from(frame.frame_y),
            rgba: frame.to_rgba(&pal),
        };
        blit_cache.insert(cache_key, blit.clone());
        items.push((ent.x, ent.y, blit));
    }

    paint_cell_sprites(image, &items, z_at)
}
