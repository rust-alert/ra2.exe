//! 地图移动单位叠画：按 rules/art 的 `Image` 解析资源；`Voxel=yes` 优先 VXL，否则 SHP。

use std::collections::HashMap;

use ra_assets::{
    HvaFile, IniDocument, Palette, ShpFile, VplFile, VxlFile, VxlLayerPose, rasterize_vxl_layer_poses, rasterize_vxl_shadow_layer_poses,
};
use ra_types::AssetSource;

use crate::{
    MapEntity, MapEntityKind, MapInfo,
    compose::{ShadowBlit, TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
    lighting::apply_rgba_tint,
    theater::{new_theater_shp_name, theater_palette},
};

/// 移动单位叠画所需的 rules / art 提示（按类型 id 去重一次）。
#[derive(Debug, Clone)]
struct MobileTypePaintHints {
    image_key: String,
    prefer_voxel: bool,
    new_theater: bool,
}

fn mobile_type_paint_hints(rules: Option<&IniDocument>, art: Option<&IniDocument>, type_id: &str) -> MobileTypePaintHints {
    let image_key = resolve_mobile_image_key(rules, art, type_id);
    let prefer_voxel = art.and_then(|a| a.get(&image_key, "Voxel")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    let new_theater = art.and_then(|a| a.get(&image_key, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    MobileTypePaintHints { image_key, prefer_voxel, new_theater }
}

fn collect_mobile_type_paint_hints(
    rules: Option<&IniDocument>,
    art: Option<&IniDocument>,
    mobiles: &[&MapEntity],
) -> HashMap<String, MobileTypePaintHints> {
    let mut out = HashMap::new();
    for ent in mobiles {
        out.entry(ent.type_id.clone()).or_insert_with(|| mobile_type_paint_hints(rules, art, &ent.type_id));
    }
    out
}

/// 移动单位绘制姿态：行走循环帧 + 是否移动中 + 格内像素偏移。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[doc(hidden)]
pub struct MobilePaintPose {
    /// 行走 / 待机循环索引（仿真 `hva_frame`）。
    pub anim_frame: u16,
    /// `true` 时步兵取 `Walk` 序列，否则 `Ready`/`Guard`。
    pub moving: bool,
    /// 相对当前格屏幕原点的水平偏移（预览像素；由 `move_accum` 滑向下一格）。
    pub offset_x: i32,
    /// 相对当前格屏幕原点的垂直偏移（预览像素）。
    pub offset_y: i32,
}

/// 步兵朝向槽表（零售 32 项），由 [`infantry_facing_slot`] 索引。
pub const INFANTRY_FACING_SLOT_TABLE: [u8; 32] =
    [7, 7, 6, 6, 6, 6, 5, 5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0, 7, 7];

/// 叠画单位 / 步兵 / 飞行器。`remap_owner` 提供房屋色调色板。
///
/// 图像键解析顺序：`rules.ini` 的 `Image` → `art.ini` 的 `Image` → 类型 id 本身。
/// （例如 `AMCV` 的 rules `Image=MCV` → `mcv.vxl`，不可误读成不存在的 `amcv.vxl`。）
///
/// `pose_of` 提供行走帧；大厅预览可传 `|_| MobilePaintPose::default()`。
pub fn paint_map_mobiles(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    docs: &crate::PaintIniDocs,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
    pose_of: &dyn Fn(&MapEntity) -> MobilePaintPose,
) -> usize {
    let mobiles: Vec<_> =
        map.entities.iter().filter(|e| matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)).collect();
    if mobiles.is_empty() {
        return 0;
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = docs.art.as_ref();
    let rules = docs.rules.as_ref();
    let type_hints = collect_mobile_type_paint_hints(rules, art, &mobiles);
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
    let mut blit_cache: HashMap<(String, u16, String), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for ent in mobiles {
        let Some(hint) = type_hints.get(&ent.type_id)
        else {
            continue;
        };
        let image_key = hint.image_key.clone();
        let prefer_voxel = hint.prefer_voxel;
        let pose = pose_of(ent);
        let frame_index = resolve_mobile_shp_frame(art, &image_key, ent, pose);
        let cache_key = (image_key.clone(), frame_index, ent.owner.clone());
        let tint = map.tint_at(ent.x, ent.y, z_at(ent.x, ent.y));
        if let Some(blit) = blit_cache.get(&cache_key) {
            let mut painted = blit.clone();
            apply_rgba_tint(&mut painted.rgba, tint);
            painted.offset_x = painted.offset_x.saturating_add(pose.offset_x);
            painted.offset_y = painted.offset_y.saturating_add(pose.offset_y);
            items.push((ent.x, ent.y, painted));
            continue;
        }

        let pal = remap_owner(&obj_pal, &ent.owner);
        let blit = if prefer_voxel {
            load_mobile_vxl_layers(source, &image_key.to_ascii_lowercase(), &pal, vpl.as_ref(), ent.facing, ent.facing)
                .or_else(|| load_mobile_shp(source, hint.new_theater, &image_key, map, &pal, frame_index, &mut shp_cache))
        }
        else {
            load_mobile_shp(source, hint.new_theater, &image_key, map, &pal, frame_index, &mut shp_cache)
                .or_else(|| load_mobile_vxl_layers(source, &image_key.to_ascii_lowercase(), &pal, vpl.as_ref(), ent.facing, ent.facing))
        };
        if let Some(mut blit) = blit {
            blit_cache.insert(cache_key, blit.clone());
            apply_rgba_tint(&mut blit.rgba, tint);
            blit.offset_x = blit.offset_x.saturating_add(pose.offset_x);
            blit.offset_y = blit.offset_y.saturating_add(pose.offset_y);
            items.push((ent.x, ent.y, blit));
        }
    }

    paint_cell_sprites(image, &items, z_at)
}

#[doc(hidden)]
pub fn resolve_mobile_image_key(rules: Option<&IniDocument>, art: Option<&IniDocument>, type_id: &str) -> String {
    rules.and_then(|r| r.get(type_id, "Image")).or_else(|| art.and_then(|a| a.get(type_id, "Image"))).unwrap_or(type_id).to_ascii_uppercase()
}

/// 步兵朝向字节 → SHP 朝向槽（0..=7）。
pub fn infantry_facing_slot(facing: u8) -> u16 {
    let step = (((u16::from(facing) >> 2) + 1) >> 1) as usize & 0x1F;
    u16::from(INFANTRY_FACING_SLOT_TABLE[step])
}

/// 解析 art 序列值 `Start,Count,FacingsOrMultiplier`；第三字段为朝向步长。
pub fn parse_sequence_triple(raw: &str) -> Option<(u16, u16, u16)> {
    let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let start: u16 = parts[0].parse().ok()?;
    let count: u16 = parts[1].parse().ok()?;
    let multiplier: u16 = parts[2].parse().ok()?;
    Some((start, count, multiplier))
}

#[doc(hidden)]
pub fn sequence_section_name(art: &IniDocument, image_key: &str) -> Option<String> {
    art.get(image_key, "Sequence").map(|s| s.trim().to_ascii_uppercase()).filter(|s| !s.is_empty())
}

#[doc(hidden)]
pub fn sequence_value<'a>(art: &'a IniDocument, seq_section: &str, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(v) = art.get(seq_section, key) {
            return Some(v);
        }
    }
    None
}

/// 由姿态与 art 序列解析 SHP 帧；无序列时回退到朝向桶。
pub fn resolve_mobile_shp_frame(art: Option<&IniDocument>, image_key: &str, ent: &MapEntity, pose: MobilePaintPose) -> u16 {
    let Some(art) = art
    else {
        return u16::from(ent.facing / 32);
    };
    // 载具 WalkFrames 等另议；步兵靠 `Sequence=`。
    if ent.kind != MapEntityKind::Infantry {
        return u16::from(ent.facing / 32);
    }
    let Some(seq_section) = sequence_section_name(art, image_key)
    else {
        return infantry_facing_slot(ent.facing);
    };
    let seq_keys: &[&str] = if pose.moving { &["Walk", "Panic"] } else { &["Ready", "Guard"] };
    let Some(raw) = sequence_value(art, &seq_section, seq_keys)
    else {
        return infantry_facing_slot(ent.facing);
    };
    let Some((start, count, multiplier)) = parse_sequence_triple(raw)
    else {
        return infantry_facing_slot(ent.facing);
    };
    let step = if count > 0 { pose.anim_frame % count } else { 0 };
    if multiplier == 0 {
        return start.saturating_add(step);
    }
    let slot = infantry_facing_slot(ent.facing);
    start.saturating_add(slot.saturating_mul(multiplier)).saturating_add(step)
}

#[doc(hidden)]
pub fn load_mobile_vxl_layers(
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
    // 落影只用车身层（炮塔 / 炮管不参与）。
    let shadow = layers.first().and_then(|body| rasterize_vxl_shadow_layer_poses(std::slice::from_ref(body))).map(|s| {
        let mut mask = vec![0u8; (s.width as usize) * (s.height as usize)];
        for (i, px) in mask.iter_mut().enumerate() {
            if s.rgba.get(i * 4 + 3).copied().unwrap_or(0) != 0 {
                *px = 1;
            }
        }
        ShadowBlit { width: s.width, height: s.height, offset_x: s.offset_x + TILE_WIDTH / 2, offset_y: s.offset_y + TILE_HEIGHT / 2, mask }
    });
    Some(TileBlit {
        width: sprite.width,
        height: sprite.height,
        offset_x: sprite.offset_x + TILE_WIDTH / 2,
        offset_y: sprite.offset_y + TILE_HEIGHT / 2,
        rgba: sprite.rgba,
        shadow,
    })
}

#[doc(hidden)]
pub fn load_mobile_shp(
    source: &dyn AssetSource,
    new_theater: bool,
    image_key: &str,
    map: &MapInfo,
    obj_pal: &Palette,
    frame_index: u16,
    shp_cache: &mut HashMap<String, ShpFile>,
) -> Option<TileBlit> {
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
    let frame = shp
        .frames
        .get(usize::from(frame_index))
        .or_else(|| shp.frames.get(usize::from(frame_index % shp.frames.len().max(1) as u16)))
        .or_else(|| shp.frames.first())?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    Some(TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x),
        offset_y: i32::from(frame.frame_y),
        rgba: frame.to_rgba(obj_pal),
        shadow: None,
    })
}
