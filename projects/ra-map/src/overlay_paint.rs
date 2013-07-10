//! 地图 Overlay SHP 叠画（类型名由调用方解析，避免依赖规则 crate）。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile};
use ra_types::AssetSource;

use crate::{
    MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites, paint_overlay_markers},
    theater::{new_theater_shp_name, theater_palette, theater_tmp_extension},
};

/// 将 overlay 叠到地形图上：优先 SHP，失败格回退色块。
///
/// `overlay_type_name`：由 rules `[OverlayTypes]` 解析得到的 id→名。
/// `art_ini`：art 文件名（如 `art.ini` / `artmd.ini`）。
///
/// 返回 `(shp 画上的格子数, 色块标记数)`。
pub fn paint_map_overlays(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
) -> (usize, usize) {
    if map.overlays.is_empty() {
        return (0, 0);
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
        let mark = paint_overlay_markers(image, &map.overlays, z_at);
        return (0, mark);
    };

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, u8), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();
    let mut unresolved = Vec::new();

    for cell in &map.overlays {
        let Some(type_name) = overlay_type_name(cell.overlay_id)
        else {
            unresolved.push(*cell);
            continue;
        };
        let image_key =
            art.as_ref().and_then(|a| a.get(&type_name, "Image")).unwrap_or(type_name.as_str()).to_ascii_uppercase();
        let frame_idx = cell.data;
        let cache_key = (image_key.clone(), frame_idx);
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((cell.x, cell.y, blit.clone()));
            continue;
        }

        let new_theater =
            art.as_ref().and_then(|a| a.get(&type_name, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let theater_yes =
            art.as_ref().and_then(|a| a.get(&type_name, "Theater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let mut candidates = Vec::new();
        if theater_yes {
            candidates.push(format!("{}.{ext}", image_key.to_ascii_lowercase()));
        }
        if new_theater {
            candidates.push(new_theater_shp_name(&image_key, map.theater));
        }
        candidates.push(format!("{}.shp", image_key.to_ascii_lowercase()));
        candidates.push(new_theater_shp_name(&image_key, map.theater));
        candidates.push(format!("{}.{ext}", image_key.to_ascii_lowercase()));

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
            unresolved.push(*cell);
            continue;
        };
        let Some(shp) = shp_cache.get(&file)
        else {
            unresolved.push(*cell);
            continue;
        };
        let frame = shp.frames.get(usize::from(frame_idx)).or_else(|| shp.frames.first());
        let Some(frame) = frame
        else {
            unresolved.push(*cell);
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            unresolved.push(*cell);
            continue;
        }
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x),
            offset_y: i32::from(frame.frame_y),
            rgba: frame.to_rgba(&obj_pal),
        };
        blit_cache.insert(cache_key, blit.clone());
        items.push((cell.x, cell.y, blit));
    }

    let shp_n = paint_cell_sprites(image, &items, z_at);
    let mark_n = if unresolved.is_empty() { 0 } else { paint_overlay_markers(image, &unresolved, z_at) };
    (shp_n, mark_n)
}
