//! 地图 Overlay SHP 叠画（类型名由调用方解析，避免依赖规则 crate）。

use std::collections::HashMap;

use ra_assets::{Hsv, IniDocument, Palette, ShpFile, shp_body_frame_count};
use ra_types::AssetSource;

use crate::{
    MapInfo, OverlayCell,
    compose::{TerrainImage, TileBlit, paint_cell_sprites, paint_overlay_markers},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
    lighting::apply_rgba_tint,
    theater::{new_theater_shp_name, theater_palette, theater_tiberium_palette, theater_tmp_extension},
};

/// 叠画层过滤：桥应压在谷底建筑之上，需分两遍画。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayLayerFilter {
    /// 全部 overlay。
    All,
    /// 非桥（矿石、围墙、岩石等）。
    Ground,
    /// 高/低桥（`BRIDGE*` / `LOBRD*`）。
    Bridge,
}

/// 是否为桥类 overlay 类型名（含高桥与低桥家族）。
pub fn is_bridge_overlay_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.starts_with("BRIDGE") || upper.starts_with("LOBRD")
}

fn layer_allows(filter: OverlayLayerFilter, is_bridge: bool) -> bool {
    match filter {
        OverlayLayerFilter::All => true,
        OverlayLayerFilter::Ground => !is_bridge,
        OverlayLayerFilter::Bridge => is_bridge,
    }
}

/// 平地矿/宝石的显示用类型名（不改资源态 id / 密度帧）。
///
/// 原版在平地格用 `((i16)x * (i16)y) % 12` 在 12 个扁平外形间选图
/// （`TIB01`–`TIB12` / `GEM01`–`GEM12`），其中较高外形即矿柱。
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
/// `tiberium_hsv`：保留参数以兼容调用方；矿石/宝石直接用 `temperat.pal` 色带，
/// **不再**对索引 16..=31 做 HSV remap（该色带在地表 pal 里已是亮金黄高光）。
/// `art_ini` / `rules_ini`：art 与 rules 文件名（rules 提供 `Image=`，如 `BRIDGE1`→`BRIDGE`）。
/// `layer`：地面 / 桥分层（先地面后建筑再桥，避免谷底楼穿桥面）。
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
    layer: OverlayLayerFilter,
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
        let filtered: Vec<OverlayCell> = map
            .overlays
            .iter()
            .copied()
            .filter(|cell| match overlay_type_name(cell.overlay_id) {
                Some(name) => layer_allows(layer, is_bridge_overlay_name(&name)),
                None => matches!(layer, OverlayLayerFilter::All | OverlayLayerFilter::Ground),
            })
            .collect();
        let mark = paint_overlay_markers(image, &filtered, z_at);
        return (0, mark);
    }
    let _ = tiberium_hsv;

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();

    // OverlayData 字节即帧号。空帧必须不画：低桥三连格侧柱（data=0/2）靠中间格宽精灵覆盖，
    // 回退到首个可画帧会把整块桥面叠到每一侧柱上。不同 `LOBRDB*` 也不共用 image_key，
    // 「同图锚点邻格」挡不住这种串画。
    let mut resolved: Vec<ResolvedOverlay> = Vec::new();
    let mut unresolved: Vec<OverlayCell> = Vec::new();

    for cell in &map.overlays {
        let Some(type_name) = overlay_type_name(cell.overlay_id)
        else {
            if matches!(layer, OverlayLayerFilter::All | OverlayLayerFilter::Ground) {
                unresolved.push(*cell);
            }
            continue;
        };
        if !layer_allows(layer, is_bridge_overlay_name(&type_name)) {
            continue;
        }
        let tib = is_tiberium(cell.overlay_id);
        let display_name = if tib { flat_tiberium_display_type_name(&type_name, cell.x, cell.y) } else { type_name.clone() };
        let (image_key, new_theater, theater_yes) = resolve_overlay_art_keys(art.as_ref(), rules.as_ref(), &type_name, &display_name);
        let pal_kind: u8 = if tib {
            2
        }
        else if theater_yes && !new_theater {
            1
        }
        else {
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
        resolved.push(ResolvedOverlay { x: cell.x, y: cell.y, data: cell.data, type_name, image_key, file, pal_kind });
    }

    let mut blit_cache: HashMap<(String, u8, u8, i32), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for item in &resolved {
        let Some(shp) = shp_cache.get(&item.file)
        else {
            continue;
        };
        let Some(frame_idx) = select_overlay_frame_index(shp, item.data)
        else {
            continue;
        };
        let y_adjust = overlay_draw_y_adjust(&item.type_name, item.data, item.pal_kind == 2);
        let cache_key = (item.image_key.clone(), frame_idx, item.pal_kind, y_adjust);
        let tint = map.tint_at(item.x, item.y, z_at(item.x, item.y));
        if let Some(blit) = blit_cache.get(&cache_key) {
            let mut painted = blit.clone();
            apply_rgba_tint(&mut painted.rgba, tint);
            items.push((item.x, item.y, painted));
            continue;
        }
        let pal: Option<&Palette> = match item.pal_kind {
            2 => tib_pal.as_ref().or(theater_pal.as_ref()).or(unit_pal.as_ref()),
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
        let mut blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x as i16) - i32::from(shp.width) / 2 + TILE_WIDTH / 2,
            offset_y: i32::from(frame.frame_y as i16) - i32::from(shp.height) / 2 + TILE_HEIGHT / 2 + y_adjust,
            rgba: frame.to_rgba(pal),
            shadow: None,
        };
        blit_cache.insert(cache_key, blit.clone());
        apply_rgba_tint(&mut blit.rgba, tint);
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
    /// 0=`unittem`，1=剧院 pal，2=矿石 `temperat`。
    pal_kind: u8,
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
                if a.get(candidate, "Theater").is_some() || a.get(candidate, "NewTheater").is_some() || a.get(candidate, "Image").is_some() {
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
    let new_theater = art.and_then(|a| a.get(&art_section, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    let theater_yes = art.and_then(|a| a.get(&art_section, "Theater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    (image_key, new_theater, theater_yes)
}

/// 矿石 / 墙 / 箱子相对格子中心的额外 Y（零售 overlay 绘制偏置 −12）。
const TIBERIUM_OVERLAY_Y_BIAS: i32 = -12;

/// 高桥主体相对格子中心的额外 Y（零售 `Get_Draw_Offset`：NS −16，EW −31）。
fn overlay_draw_y_adjust(type_name: &str, data: u8, is_tiberium: bool) -> i32 {
    if is_high_bridge_body_name(type_name) {
        if (9..=17).contains(&data) { -31 } else { -16 }
    }
    else if is_tiberium {
        TIBERIUM_OVERLAY_Y_BIAS
    }
    else {
        0
    }
}

fn is_high_bridge_body_name(name: &str) -> bool {
    matches!(name.to_ascii_uppercase().as_str(), "BRIDGE1" | "BRIDGE2" | "BRIDGEB1" | "BRIDGEB2")
}

fn frame_drawable(shp: &ShpFile, idx: u8) -> bool {
    shp.frames.get(usize::from(idx)).is_some_and(|f| f.frame_width > 0 && f.frame_height > 0)
}

/// 只用 OverlayData 指向的主体帧；空帧 / 落影半幅一律不画、不回退。
fn select_overlay_frame_index(shp: &ShpFile, preferred: u8) -> Option<u8> {
    let body = shp_body_frame_count(&shp.frames);
    if usize::from(preferred) >= body {
        return None;
    }
    frame_drawable(shp, preferred).then_some(preferred)
}

/// 将指定 overlay 格叠到预览 RGBA（运行时产矿脏刷新；不重合成整图）。
///
/// `cells` 为空时直接返回。会沿用 `map` 的剧院与高程，仅替换 `overlays` 列表。
pub fn paint_overlays_onto_preview_rgba(
    source: &dyn AssetSource,
    map: &MapInfo,
    cells: &[OverlayCell],
    image: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
    art_ini: &str,
    rules_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    is_tiberium: &dyn Fn(u8) -> bool,
    tiberium_hsv: &dyn Fn(u8) -> Option<Hsv>,
    layer: OverlayLayerFilter,
) -> (usize, usize) {
    if cells.is_empty() {
        return (0, 0);
    }
    let mut overlay_map = map.clone();
    overlay_map.overlays = cells.to_vec();
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let n = paint_map_overlays(source, &overlay_map, &mut terrain, art_ini, rules_ini, overlay_type_name, is_tiberium, tiberium_hsv, layer);
    *image = terrain.image;
    n
}
