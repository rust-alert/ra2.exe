//! 地图 `[Terrain]` 物件 SHP 叠画。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile};
use ra_types::AssetSource;

use crate::{
    MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
    lighting::apply_rgba_tint,
    theater::{theater_palette, theater_tiberium_palette, theater_tmp_extension},
};

/// FA2 `IsoView` 对地形物件（树/岩）的额外 Y（钻石中心叠画后再偏 −3）。
const TERRAIN_OBJECT_Y_FUDGE: i32 = -3;

/// 将地形物件叠到合成图上。返回画上的物件数。
///
/// `rules_ini`：`SpawnsTiberium=yes`（如 `TIBTRE*` 矿柱）须用剧院地表 pal（`temperat.pal`），
/// 不可用 `isotem.pal`，否则在浅绿地上呈灰白幽灵剪影。
pub fn paint_map_terrain_objects(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    rules_ini: &str,
) -> usize {
    if map.terrain_objects.is_empty() {
        return 0;
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let rules = source.read(rules_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    // 普通树/岩：剧院砖盘（`isotem`）；矿柱：地表盘（`temperat`）；再回退 `unittem`。
    let theater_pal = source.read(theater_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok());
    let tib_pal = source
        .read(theater_tiberium_palette(map.theater))
        .ok()
        .and_then(|b| Palette::parse(&b).ok());
    let unit_pal = source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok());
    if theater_pal.is_none() && tib_pal.is_none() && unit_pal.is_none() {
        return 0;
    }

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    // image_key + 是否矿柱色板，避免同图键串色。
    let mut blit_cache: HashMap<(String, bool), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for obj in &map.terrain_objects {
        let image_key = art.as_ref().and_then(|a| a.get(&obj.name, "Image")).unwrap_or(obj.name.as_str()).to_ascii_uppercase();
        let use_tib_pal = terrain_uses_tiberium_palette(rules.as_ref(), &obj.name);
        let cache_key = (image_key.clone(), use_tib_pal);
        let tint = map.tint_at(obj.x, obj.y, z_at(obj.x, obj.y));
        if let Some(blit) = blit_cache.get(&cache_key) {
            let mut painted = blit.clone();
            apply_rgba_tint(&mut painted.rgba, tint);
            items.push((obj.x, obj.y, painted));
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
        let pal = if use_tib_pal {
            tib_pal.as_ref().or(theater_pal.as_ref()).or(unit_pal.as_ref())
        } else {
            theater_pal.as_ref().or(tib_pal.as_ref()).or(unit_pal.as_ref())
        };
        let Some(pal) = pal
        else {
            continue;
        };
        // 与 overlay / 建筑一致：子帧相对整幅画布裁切，锚在钻石中心（再加 FA2 −3 Y）。
        let mut blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x as i16) - i32::from(shp.width) / 2 + TILE_WIDTH / 2,
            offset_y: i32::from(frame.frame_y as i16) - i32::from(shp.height) / 2 + TILE_HEIGHT / 2 + TERRAIN_OBJECT_Y_FUDGE,
            rgba: frame.to_rgba(pal),
            shadow: None,
        };
        blit_cache.insert(cache_key, blit.clone());
        apply_rgba_tint(&mut blit.rgba, tint);
        items.push((obj.x, obj.y, blit));
    }

    paint_cell_sprites(image, &items, z_at)
}

/// `SpawnsTiberium=yes` 的地形物件（零售矿柱 `TIBTRE*`）用地表矿石调色板。
fn terrain_uses_tiberium_palette(rules: Option<&IniDocument>, name: &str) -> bool {
    rules
        .and_then(|r| r.get(name, "SpawnsTiberium"))
        .is_some_and(|v| v.eq_ignore_ascii_case("yes"))
}
