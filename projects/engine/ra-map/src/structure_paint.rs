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
///
/// 除主体外还会叠 `ActiveAnim` / `ActiveAnimTwo`（如油田旗帜 `CAOILD_F`）。
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
    // (layer_stem, owner, frame, z_adjust) → blit
    let mut blit_cache: HashMap<(String, String, u16, i32), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for ent in structures {
        let art_section = art
            .as_ref()
            .and_then(|a| {
                let image_key = a.get(&ent.type_id, "Image").unwrap_or(ent.type_id.as_str());
                if a.section(image_key).is_some() {
                    Some(image_key.to_ascii_uppercase())
                } else if a.section(&ent.type_id).is_some() {
                    Some(ent.type_id.to_ascii_uppercase())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| ent.type_id.to_ascii_uppercase());

        let remapable = art
            .as_ref()
            .and_then(|a| a.get(&art_section, "Remapable"))
            .is_none_or(|v| !v.eq_ignore_ascii_case("no"));
        let pal = if remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };

        let body_key = art.as_ref().and_then(|a| a.get(&art_section, "Image")).unwrap_or(art_section.as_str()).to_ascii_uppercase();
        let body_new_theater =
            art.as_ref().and_then(|a| a.get(&art_section, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        if let Some(blit) = load_structure_blit(
            source,
            map,
            &body_key,
            body_new_theater,
            0,
            0,
            &pal,
            &mut shp_cache,
            &mut blit_cache,
            &ent.owner,
        ) {
            items.push((ent.x, ent.y, blit));
        }

        // 活动层：泵机 / 旗帜等。预览取 Start 帧（未接逐帧驱动前为静止姿态）。
        for (anim_key, z_key) in [("ActiveAnim", "ActiveAnimZAdjust"), ("ActiveAnimTwo", "ActiveAnimTwoZAdjust")] {
            let Some(anim_name) = art.as_ref().and_then(|a| a.get(&art_section, anim_key)).map(str::to_ascii_uppercase)
            else {
                continue;
            };
            let z_adjust = art.as_ref().and_then(|a| a.get(&art_section, z_key)).and_then(parse_i32).unwrap_or(0);
            let anim_image = art.as_ref().and_then(|a| a.get(&anim_name, "Image")).unwrap_or(anim_name.as_str()).to_ascii_uppercase();
            let anim_new_theater =
                art.as_ref().and_then(|a| a.get(&anim_name, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
            let start = art.as_ref().and_then(|a| a.get(&anim_name, "Start")).and_then(parse_u16).unwrap_or(0);
            let anim_remapable = art
                .as_ref()
                .and_then(|a| a.get(&anim_name, "Remapable"))
                .map(|v| !v.eq_ignore_ascii_case("no"))
                .unwrap_or(remapable);
            let anim_pal = if anim_remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };
            if let Some(blit) = load_structure_blit(
                source,
                map,
                &anim_image,
                anim_new_theater,
                start,
                z_adjust,
                &anim_pal,
                &mut shp_cache,
                &mut blit_cache,
                &ent.owner,
            ) {
                items.push((ent.x, ent.y, blit));
            }
        }
    }

    paint_cell_sprites(image, &items, z_at)
}

fn load_structure_blit(
    source: &dyn AssetSource,
    map: &MapInfo,
    image_key: &str,
    new_theater: bool,
    frame_idx: u16,
    z_adjust: i32,
    pal: &Palette,
    shp_cache: &mut HashMap<String, ShpFile>,
    blit_cache: &mut HashMap<(String, String, u16, i32), TileBlit>,
    owner: &str,
) -> Option<TileBlit> {
    let cache_key = (image_key.to_owned(), owner.to_owned(), frame_idx, z_adjust);
    if let Some(blit) = blit_cache.get(&cache_key) {
        return Some(blit.clone());
    }

    let candidates = if new_theater {
        vec![new_theater_shp_name(image_key, map.theater), format!("{}.shp", image_key.to_ascii_lowercase())]
    } else {
        vec![format!("{}.shp", image_key.to_ascii_lowercase()), new_theater_shp_name(image_key, map.theater)]
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
    let file = loaded?;
    let shp = shp_cache.get(&file)?;
    let frame = shp.frames.get(usize::from(frame_idx)).or_else(|| shp.frames.first())?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    let blit = TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x as i16),
        // ZAdjust 按像素偏置叠画 Y，使旗帜等活动层相对主体前后正确。
        offset_y: i32::from(frame.frame_y as i16) + z_adjust,
        rgba: frame.to_rgba(pal),
    };
    blit_cache.insert(cache_key, blit.clone());
    Some(blit)
}

fn parse_i32(raw: &str) -> Option<i32> {
    raw.trim().parse().ok()
}

fn parse_u16(raw: &str) -> Option<u16> {
    raw.trim().parse().ok()
}
