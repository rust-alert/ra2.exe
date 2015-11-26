//! 地图 Overlay SHP 叠画（类型名由调用方解析，避免依赖规则 crate）。

use std::collections::{HashMap, HashSet};

use ra_assets::{Hsv, IniDocument, Palette, ShpFile};
use ra_types::AssetSource;

use crate::{
    MapInfo, OverlayCell,
    compose::{TerrainImage, TileBlit, paint_cell_sprites, paint_overlay_markers},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
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
/// `tiberium_hsv`：矿/宝石 `[Tiberiums] Color=` 对应的 HSV（索引 16..=31 remap）；
/// 原版 `NeonGreen=0,0,0` 为矿石哨兵，调用方应换成可用金色方案。
/// `art_ini` / `rules_ini`：art 与 rules 文件名（rules 提供 `Image=`，如 `BRIDGE1`→`BRIDGE`）。
///
/// 返回 `(shp 画上的格子数, 色块标记数)`。
pub fn paint_map_overlays(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    rules_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    is_tiberium: &dyn Fn(u8) -> bool,
    tiberium_hsv: &dyn Fn(u8) -> Option<Hsv>,
) -> (usize, usize) {
    if map.overlays.is_empty() {
        return (0, 0);
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let rules = source.read(rules_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let unit_pal = source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok());
    let theater_pal = source.read(theater_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok());
    let tib_pal = source.read(theater_tiberium_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok());
    if unit_pal.is_none() && theater_pal.is_none() && tib_pal.is_none() {
        let mark = paint_overlay_markers(image, &map.overlays, z_at);
        return (0, mark);
    }

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();

    // 第一遍：解析 image_key / 加载 SHP，记录「首选帧可画」的锚点格。
    // 低桥 LOBRDB 等同图三连格里只有 data=1 有像素；data=0/2 空帧是 footprint，不能回退。
    // 断桥端头 LOBRDG 等只有空帧、又无同图锚点邻格时，回退到首个可画帧。
    let mut resolved: Vec<ResolvedOverlay> = Vec::new();
    let mut unresolved: Vec<OverlayCell> = Vec::new();
    let mut anchors: HashSet<(String, u16, u16)> = HashSet::new();

    for cell in &map.overlays {
        let Some(type_name) = overlay_type_name(cell.overlay_id)
        else {
            unresolved.push(*cell);
            continue;
        };
        let tib = is_tiberium(cell.overlay_id);
        let tib_hsv = if tib { tiberium_hsv(cell.overlay_id) } else { None };
        let display_name = if tib {
            flat_tiberium_display_type_name(&type_name, cell.x, cell.y)
        } else {
            type_name.clone()
        };
        let (image_key, new_theater, theater_yes) =
            resolve_overlay_art_keys(art.as_ref(), rules.as_ref(), &type_name, &display_name);
        let pal_kind: u8 = if tib {
            2
        } else if theater_yes && !new_theater {
            1
        } else {
            0
        };

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
        let preferred_drawable = shp_cache.get(&file).is_some_and(|shp| frame_drawable(shp, cell.data));
        if preferred_drawable {
            anchors.insert((image_key.clone(), cell.x, cell.y));
        }
        resolved.push(ResolvedOverlay {
            x: cell.x,
            y: cell.y,
            data: cell.data,
            type_name,
            image_key,
            file,
            pal_kind,
            tib_hsv,
        });
    }

    let mut blit_cache: HashMap<(String, u8, u8, u32, i32), TileBlit> = HashMap::new();
    let mut tib_pal_cache: HashMap<u32, Palette> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for item in &resolved {
        let Some(shp) = shp_cache.get(&item.file)
        else {
            continue;
        };
        let allow_fallback = !has_same_image_anchor_neighbor(&anchors, &item.image_key, item.x, item.y);
        let Some(frame_idx) = select_overlay_frame_index(shp, item.data, allow_fallback)
        else {
            continue;
        };
        let y_adjust = overlay_draw_y_adjust(&item.type_name, item.data);
        let hsv_key = item
            .tib_hsv
            .map(|h| u32::from(h.h) << 16 | u32::from(h.s) << 8 | u32::from(h.v))
            .unwrap_or(0);
        let cache_key = (item.image_key.clone(), frame_idx, item.pal_kind, hsv_key, y_adjust);
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((item.x, item.y, blit.clone()));
            continue;
        }
        if item.pal_kind == 2 {
            if let Some(hsv) = item.tib_hsv {
                if let Some(base) = tib_pal.as_ref().or(theater_pal.as_ref()).or(unit_pal.as_ref()) {
                    tib_pal_cache.entry(hsv_key).or_insert_with(|| base.with_hsv_remap(hsv));
                }
            }
        }
        let pal: Option<&Palette> = match item.pal_kind {
            2 => tib_pal_cache
                .get(&hsv_key)
                .or(tib_pal.as_ref())
                .or(theater_pal.as_ref())
                .or(unit_pal.as_ref()),
            1 => theater_pal.as_ref().or(tib_pal.as_ref()).or(unit_pal.as_ref()),
            _ => unit_pal.as_ref().or(theater_pal.as_ref()).or(tib_pal.as_ref()),
        };
        let Some(pal) = pal
        else {
            continue;
        };
        let Some(frame) = shp.frames.get(usize::from(frame_idx))
        else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        // TS/RA2 overlay：子帧相对整幅画布裁切；叠画锚在钻石中心，再加高桥等 Y 修正。
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x as i16) - i32::from(shp.width) / 2 + TILE_WIDTH / 2,
            offset_y: i32::from(frame.frame_y as i16) - i32::from(shp.height) / 2 + TILE_HEIGHT / 2 + y_adjust,
            rgba: frame.to_rgba(pal),
        };
        blit_cache.insert(cache_key, blit.clone());
        items.push((item.x, item.y, blit));
    }

    let shp_n = paint_cell_sprites(image, &items, z_at);
    let mark_n = if unresolved.is_empty() { 0 } else { paint_overlay_markers(image, &unresolved, z_at) };
    (shp_n, mark_n)
}

struct ResolvedOverlay {
    x: u16,
    y: u16,
    data: u8,
    type_name: String,
    image_key: String,
    file: String,
    pal_kind: u8,
    tib_hsv: Option<Hsv>,
}

/// 解析 overlay 的 SHP 键与剧院标志：rules `Image=`（如 `BRIDGE1`→`BRIDGE`）再落到 art 节。
///
/// 画图键优先级：art `Image=` → rules `Image=` → `display_name`（矿石坐标变体等）。
fn resolve_overlay_art_keys(
    art: Option<&IniDocument>,
    rules: Option<&IniDocument>,
    type_name: &str,
    display_name: &str,
) -> (String, bool, bool) {
    let rules_image = rules.and_then(|r| r.get(type_name, "Image")).map(str::to_ascii_uppercase);
    let rules_image_or_type = rules_image.clone().unwrap_or_else(|| type_name.to_ascii_uppercase());
    let art_section = art
        .and_then(|a| {
            for candidate in [type_name, rules_image_or_type.as_str(), display_name] {
                if a.get(candidate, "Theater").is_some()
                    || a.get(candidate, "NewTheater").is_some()
                    || a.get(candidate, "Image").is_some()
                {
                    return Some(candidate.to_ascii_uppercase());
                }
            }
            None
        })
        .unwrap_or_else(|| type_name.to_ascii_uppercase());
    let image_key = art
        .and_then(|a| a.get(&art_section, "Image"))
        .map(str::to_ascii_uppercase)
        .or(rules_image)
        .unwrap_or_else(|| display_name.to_ascii_uppercase());
    let new_theater = art
        .and_then(|a| a.get(&art_section, "NewTheater"))
        .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    let theater_yes = art
        .and_then(|a| a.get(&art_section, "Theater"))
        .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    (image_key, new_theater, theater_yes)
}

/// 高桥主体相对格子中心的额外 Y（零售 `Get_Draw_Offset`：NS −16，EW −31）。
fn overlay_draw_y_adjust(type_name: &str, data: u8) -> i32 {
    if is_high_bridge_body_name(type_name) {
        if (9..=17).contains(&data) {
            -31
        } else {
            -16
        }
    } else {
        0
    }
}

fn is_high_bridge_body_name(name: &str) -> bool {
    matches!(
        name.to_ascii_uppercase().as_str(),
        "BRIDGE1" | "BRIDGE2" | "BRIDGEB1" | "BRIDGEB2"
    )
}

fn frame_drawable(shp: &ShpFile, idx: u8) -> bool {
    shp.frames
        .get(usize::from(idx))
        .is_some_and(|f| f.frame_width > 0 && f.frame_height > 0)
}

fn has_same_image_anchor_neighbor(anchors: &HashSet<(String, u16, u16)>, image_key: &str, x: u16, y: u16) -> bool {
    let key = image_key.to_string();
    for (nx, ny) in [
        (x.wrapping_sub(1), y),
        (x.wrapping_add(1), y),
        (x, y.wrapping_sub(1)),
        (x, y.wrapping_add(1)),
    ] {
        if anchors.contains(&(key.clone(), nx, ny)) {
            return true;
        }
    }
    false
}

/// 首选帧可画则用之；否则在无同图锚点邻格时回退到首个可画帧。
fn select_overlay_frame_index(shp: &ShpFile, preferred: u8, allow_fallback: bool) -> Option<u8> {
    if frame_drawable(shp, preferred) {
        return Some(preferred);
    }
    if !allow_fallback {
        return None;
    }
    shp.frames
        .iter()
        .enumerate()
        .find(|(_, f)| f.frame_width > 0 && f.frame_height > 0)
        .and_then(|(i, _)| u8::try_from(i).ok())
}
