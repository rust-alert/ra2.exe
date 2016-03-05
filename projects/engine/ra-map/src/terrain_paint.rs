//! 地图 `[Terrain]` 物件 SHP 叠画。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile, shp_body_frame_count};
use ra_types::AssetSource;

use crate::{
    MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
    lighting::apply_rgba_tint,
    theater::{theater_palette, theater_tmp_extension},
};

/// FA2 `IsoView` 对地形物件（树/岩）的额外 Y（钻石中心叠画后再偏 −3）。
const TERRAIN_OBJECT_Y_FUDGE: i32 = -3;

/// 逻辑帧率：`rules` 的 `AnimationRate` 以该帧率为单位间隔。
const TERRAIN_LOGIC_FPS: u32 = 15;

/// 将 `rules` `AnimationRate`（逻辑帧间隔）换算为毫秒/动画帧。
pub fn terrain_animation_rate_ms(animation_rate: u32) -> u32 {
    let frames = animation_rate.max(1);
    (frames * 1000) / TERRAIN_LOGIC_FPS
}

/// 按呈现时钟与 `AnimationRate` 选取地形动画主体帧下标。
///
/// `frame_count` 应为 `shp_body_frame_count`（不含落影半幅）。
pub fn terrain_anim_frame(clock_ms: u64, animation_rate: u32, frame_count: usize) -> usize {
    if frame_count == 0 {
        return 0;
    }
    let rate = u64::from(terrain_animation_rate_ms(animation_rate).max(1));
    ((clock_ms / rate) % frame_count as u64) as usize
}

/// 将地形物件叠到合成图上。返回画上的物件数。
///
/// `rules_ini` 提供 `IsAnimated` / `AnimationRate`。`anim_clock_ms` 驱动动画选帧。
/// 静态物件始终画主体第 0 帧。动画物件按时钟在主体帧间循环。
pub fn paint_map_terrain_objects(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    rules_ini: &str,
    anim_clock_ms: u64,
) -> usize {
    if map.terrain_objects.is_empty() {
        return 0;
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let rules = source.read(rules_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    // `Theater=yes` 地形物件统一使用等距剧院调色板。`SpawnsTiberium` 等玩法字段
    // 不改变 SHP 的索引语义，矿柱也属于这一资源族。
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
    let mut blit_cache: HashMap<(String, u16), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for obj in &map.terrain_objects {
        let image_key = art.as_ref().and_then(|a| a.get(&obj.name, "Image")).unwrap_or(obj.name.as_str()).to_ascii_uppercase();
        let animated = rules.as_ref().is_some_and(|r| is_yes(r.get(&obj.name, "IsAnimated")));
        let anim_rate = rules
            .as_ref()
            .and_then(|r| r.get(&obj.name, "AnimationRate"))
            .and_then(parse_u32)
            .unwrap_or(1);
        let tint = map.tint_at(obj.x, obj.y, z_at(obj.x, obj.y));

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
        let body_n = shp_body_frame_count(&shp.frames);
        if body_n == 0 {
            continue;
        }
        let frame_idx = if animated {
            terrain_anim_frame(anim_clock_ms, anim_rate, body_n) as u16
        } else {
            0
        };
        let cache_key = (image_key.clone(), frame_idx);
        if let Some(blit) = blit_cache.get(&cache_key) {
            let mut painted = blit.clone();
            apply_rgba_tint(&mut painted.rgba, tint);
            items.push((obj.x, obj.y, painted));
            continue;
        }
        let Some(frame) = shp.frames.get(usize::from(frame_idx))
        else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        // 与 overlay / 建筑一致：子帧相对整幅画布裁切，锚在钻石中心（再加 FA2 −3 Y）。
        let mut blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x as i16) - i32::from(shp.width) / 2 + TILE_WIDTH / 2,
            offset_y: i32::from(frame.frame_y as i16) - i32::from(shp.height) / 2 + TILE_HEIGHT / 2 + TERRAIN_OBJECT_Y_FUDGE,
            rgba: frame.to_rgba(&obj_pal),
            shadow: None,
        };
        blit_cache.insert(cache_key, blit.clone());
        apply_rgba_tint(&mut blit.rgba, tint);
        items.push((obj.x, obj.y, blit));
    }

    paint_cell_sprites(image, &items, z_at)
}

fn is_yes(raw: Option<&str>) -> bool {
    raw.is_some_and(|v| v.eq_ignore_ascii_case("yes"))
}

fn parse_u32(raw: &str) -> Option<u32> {
    raw.trim().parse().ok()
}
