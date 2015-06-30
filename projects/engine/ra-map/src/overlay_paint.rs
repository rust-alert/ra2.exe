//! 地图 Overlay SHP 叠画（类型名由调用方解析，避免依赖规则 crate）。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile};
use ra_types::AssetSource;

use crate::{
    MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites, paint_overlay_markers},
    theater::{new_theater_shp_name, theater_palette, theater_tiberium_palette, theater_tmp_extension},
};

/// 平地矿/宝石的显示用类型名（不改资源态 id / 密度帧）。
///
/// 原版在平地格用 `((i16)x * (i16)y) % 12` 在 12 个扁平外形间选图
///（`TIB01`–`TIB12` / `GEM01`–`GEM12`），其中较高外形即矿柱。
/// 地图坐标一般为正；负余数按原版再落到 `0..11`。
pub fn flat_tiberium_display_type_name(type_name: &str, x: u16, y: u16) -> String {
    const VARIANT_COUNT: i32 = 12;
    let sx = i32::from(x as i16);
    let sy = i32::from(y as i16);
    let rem = sx.wrapping_mul(sy) % VARIANT_COUNT;
    let variant = u8::try_from(rem.rem_euclid(VARIANT_COUNT)).unwrap_or(0) + 1;

    let upper = type_name.to_ascii_uppercase();
    if upper.starts_with("GEM") {
        return format!("GEM{variant:02}");
    }
    if let Some(rest) = upper.strip_prefix("TIB") {
        if let Some((family, _)) = rest.split_once('_') {
            if !family.is_empty() && family.chars().all(|c| c.is_ascii_digit()) {
                return format!("TIB{family}_{variant:02}");
            }
        }
        if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            return format!("TIB{variant:02}");
        }
    }
    type_name.to_string()
}

/// 将 overlay 叠到地形图上：优先 SHP，失败格回退色块。
///
/// `overlay_type_name`：由 rules `[OverlayTypes]` 解析得到的 id→名。
/// `is_tiberium`：该 id 是否 `Tiberium=yes`（矿/宝石须用剧院地表 pal，如 `temperat.pal`，
/// 不能用 `isotem.pal`，否则呈灰黑底块）。
/// `art_ini`：art 文件名（如 `art.ini` / `artmd.ini`）。
///
/// 返回 `(shp 画上的格子数, 色块标记数)`。
pub fn paint_map_overlays(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    is_tiberium: &dyn Fn(u8) -> bool,
) -> (usize, usize) {
    if map.overlays.is_empty() {
        return (0, 0);
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let unit_pal = source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok());
    let theater_pal = source.read(theater_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok());
    let tib_pal = source.read(theater_tiberium_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok());
    if unit_pal.is_none() && theater_pal.is_none() && tib_pal.is_none() {
        let mark = paint_overlay_markers(image, &map.overlays, z_at);
        return (0, mark);
    }

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    // (image_key, frame, pal_kind): 0=unit 1=theater_iso 2=tiberium
    let mut blit_cache: HashMap<(String, u8, u8), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();
    let mut unresolved = Vec::new();

    for cell in &map.overlays {
        let Some(type_name) = overlay_type_name(cell.overlay_id)
        else {
            unresolved.push(*cell);
            continue;
        };
        let tib = is_tiberium(cell.overlay_id);
        // 平地矿：资源态仍用 pack 里的 id，画图换成坐标派生的 TIB/GEM 外形（含矿柱）。
        let display_name = if tib {
            flat_tiberium_display_type_name(&type_name, cell.x, cell.y)
        } else {
            type_name.clone()
        };
        let art_section = art
            .as_ref()
            .and_then(|a| {
                if a.get(&display_name, "Theater").is_some()
                    || a.get(&display_name, "NewTheater").is_some()
                    || a.get(&display_name, "Image").is_some()
                {
                    Some(display_name.as_str())
                } else {
                    None
                }
            })
            .unwrap_or(type_name.as_str());
        let image_key = art
            .as_ref()
            .and_then(|a| a.get(art_section, "Image"))
            .unwrap_or(display_name.as_str())
            .to_ascii_uppercase();
        let frame_idx = cell.data;
        let new_theater = art.as_ref().and_then(|a| a.get(art_section, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let theater_yes = art.as_ref().and_then(|a| a.get(art_section, "Theater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let pal_kind: u8 = if tib {
            2
        } else if theater_yes && !new_theater {
            1
        } else {
            0
        };
        let cache_key = (image_key.clone(), frame_idx, pal_kind);
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((cell.x, cell.y, blit.clone()));
            continue;
        }

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
        // 空帧必须不画：低桥侧柱等 data 指向空帧时回退会造出幽灵板。
        let Some(frame) = drawable_frame(shp, frame_idx)
        else {
            unresolved.push(*cell);
            continue;
        };
        let Some(pal) = (match pal_kind {
            2 => tib_pal.as_ref().or(theater_pal.as_ref()).or(unit_pal.as_ref()),
            1 => theater_pal.as_ref().or(tib_pal.as_ref()).or(unit_pal.as_ref()),
            _ => unit_pal.as_ref().or(theater_pal.as_ref()).or(tib_pal.as_ref()),
        })
        else {
            unresolved.push(*cell);
            continue;
        };
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x as i16),
            offset_y: i32::from(frame.frame_y as i16),
            rgba: frame.to_rgba(pal),
        };
        blit_cache.insert(cache_key, blit.clone());
        items.push((cell.x, cell.y, blit));
    }

    let shp_n = paint_cell_sprites(image, &items, z_at);
    let mark_n = if unresolved.is_empty() { 0 } else { paint_overlay_markers(image, &unresolved, z_at) };
    (shp_n, mark_n)
}

/// 选取可画帧：仅当 `preferred` 宽高非 0；空帧不回退。
fn drawable_frame(shp: &ShpFile, preferred: u8) -> Option<&ra_assets::ShpFrame> {
    let frame = shp.frames.get(usize::from(preferred))?;
    (frame.frame_width > 0 && frame.frame_height > 0).then_some(frame)
}
