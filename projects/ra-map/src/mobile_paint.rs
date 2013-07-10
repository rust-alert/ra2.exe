//! 地图移动单位叠画：优先 SHP，否则 VXL（含炮塔层）正交投影。

use std::collections::HashMap;

use ra_assets::{HvaFile, IniDocument, Palette, ShpFile, VplFile, VxlFile, VxlLayerPose, rasterize_vxl_layer_poses};
use ra_types::AssetSource;

use crate::{
    MapEntityKind, MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
    theater::{new_theater_shp_name, theater_palette},
};

/// 叠画单位 / 步兵 / 飞行器。`remap_owner` 提供房屋色调色板。
pub fn paint_map_mobiles(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> usize {
    let mobiles: Vec<_> = map
        .entities
        .iter()
        .filter(|e| matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
        .collect();
    if mobiles.is_empty() {
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
    let vpl = source.read("voxels.vpl").ok().and_then(|b| VplFile::parse(&b).ok());

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, u8, String), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for ent in mobiles {
        let image_key =
            art.as_ref().and_then(|a| a.get(&ent.type_id, "Image")).unwrap_or(ent.type_id.as_str()).to_ascii_uppercase();
        let frame_hint = ent.facing / 32;
        let cache_key = (image_key.clone(), frame_hint, ent.owner.clone());
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((ent.x, ent.y, blit.clone()));
            continue;
        }

        let pal = remap_owner(&obj_pal, &ent.owner);

        if let Some(blit) = load_mobile_shp(source, &art, &image_key, map, &pal, frame_hint, &mut shp_cache) {
            blit_cache.insert(cache_key, blit.clone());
            items.push((ent.x, ent.y, blit));
            continue;
        }

        let stem = image_key.to_ascii_lowercase();
        if let Some(blit) = load_mobile_vxl_layers(source, &stem, &pal, vpl.as_ref(), ent.facing, ent.facing) {
            blit_cache.insert(cache_key, blit.clone());
            items.push((ent.x, ent.y, blit));
        }
    }

    paint_cell_sprites(image, &items, z_at)
}

fn load_mobile_vxl_layers(
    source: &dyn AssetSource,
    stem: &str,
    pal: &Palette,
    vpl: Option<&VplFile>,
    body_facing: u8,
    turret_facing: u8,
) -> Option<TileBlit> {
    let body_name = format!("{stem}.vxl");
    let body_bytes = source.read(&body_name).ok()?;
    let body = VxlFile::parse(&body_bytes).ok()?;
    let body_hva = source.read(&format!("{stem}.hva")).ok().and_then(|b| HvaFile::parse(&b).ok());

    let mut owned: Vec<(VxlFile, Option<HvaFile>, bool)> = vec![(body, body_hva, false)];
    for suffix in ["tur", "barl", "barrel"] {
        let vxl_name = format!("{stem}{suffix}.vxl");
        let Ok(bytes) = source.read(&vxl_name)
        else {
            continue;
        };
        let Ok(vxl) = VxlFile::parse(&bytes)
        else {
            continue;
        };
        let hva = source.read(&format!("{stem}{suffix}.hva")).ok().and_then(|b| HvaFile::parse(&b).ok());
        owned.push((vxl, hva, true));
        if suffix.starts_with("bar") {
            break;
        }
    }

    let layers: Vec<VxlLayerPose<'_>> = owned
        .iter()
        .map(|(v, h, is_turret)| VxlLayerPose {
            vxl: v,
            hva: h.as_ref(),
            facing: if *is_turret { turret_facing } else { body_facing },
            frame: 0,
        })
        .collect();
    let sprite = rasterize_vxl_layer_poses(&layers, pal, vpl)?;
    Some(TileBlit {
        width: sprite.width,
        height: sprite.height,
        offset_x: sprite.offset_x + TILE_WIDTH / 2,
        offset_y: sprite.offset_y + TILE_HEIGHT / 2,
        rgba: sprite.rgba,
    })
}

fn load_mobile_shp(
    source: &dyn AssetSource,
    art: &Option<IniDocument>,
    image_key: &str,
    map: &MapInfo,
    obj_pal: &Palette,
    frame_hint: u8,
    shp_cache: &mut HashMap<String, ShpFile>,
) -> Option<TileBlit> {
    let new_theater = art.as_ref().and_then(|a| a.get(image_key, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    let candidates = if new_theater {
        vec![new_theater_shp_name(image_key, map.theater), format!("{}.shp", image_key.to_ascii_lowercase())]
    }
    else {
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
    let frame = shp.frames.get(usize::from(frame_hint)).or_else(|| shp.frames.first())?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    Some(TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x),
        offset_y: i32::from(frame.frame_y),
        rgba: frame.to_rgba(obj_pal),
    })
}
