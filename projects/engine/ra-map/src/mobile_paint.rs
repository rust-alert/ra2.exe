//! 地图移动单位叠画：按 rules/art 的 `Image` 解析资源；`Voxel=yes` 优先 VXL，否则 SHP。

use std::collections::HashMap;

use ra_assets::{
    HvaFile, IniDocument, Palette, ShpFile, VplFile, VxlFile, VxlLayerPose, from_row, rasterize_vxl_layer_poses,
    rasterize_vxl_shadow_layer_poses,
};
use ra_types::AssetSource;
use serde::Deserialize;

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
    /// 移动序列 `Start,Count,Multiplier`（`Walk` / `Panic`）。
    walk_triple: Option<(u16, u16, u16)>,
    /// 待机序列 `Start,Count,Multiplier`（`Ready` / `Guard`）。
    ready_triple: Option<(u16, u16, u16)>,
}

fn mobile_type_paint_hints(rules: Option<&IniDocument>, art: Option<&IniDocument>, type_id: &str) -> MobileTypePaintHints {
    let image_key = resolve_mobile_image_key(rules, art, type_id);
    let art_fields = art
        .and_then(|a| a.section(&image_key))
        .and_then(|s| s.deserialize::<MobileArtImageFields>().ok())
        .unwrap_or_default();
    let prefer_voxel = art_fields.voxel.unwrap_or(false);
    let new_theater = art_fields.new_theater.unwrap_or(false);
    let sequence_section = art_fields
        .sequence
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_uppercase());
    let (walk_triple, ready_triple) = match (art, sequence_section.as_deref()) {
        (Some(art), Some(seq)) => sequence_triples_from_section(art, seq),
        _ => (None, None),
    };
    MobileTypePaintHints {
        image_key,
        prefer_voxel,
        new_theater,
        walk_triple,
        ready_triple,
    }
}

#[derive(Debug, Default, Deserialize)]
struct MobileArtImageFields {
    #[serde(rename = "Voxel")]
    voxel: Option<bool>,
    #[serde(rename = "NewTheater")]
    new_theater: Option<bool>,
    #[serde(rename = "Sequence")]
    sequence: Option<String>,
    #[serde(rename = "Image")]
    image: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct MobileRulesImageFields {
    #[serde(rename = "Image")]
    image: Option<String>,
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
        let frame_index = resolve_mobile_shp_frame_from_hints(hint, ent, pose);
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
    let from_rules = rules
        .and_then(|r| r.section(type_id))
        .and_then(|s| s.deserialize::<MobileRulesImageFields>().ok())
        .and_then(|f| f.image);
    let from_art = art
        .and_then(|a| a.section(type_id))
        .and_then(|s| s.deserialize::<MobileArtImageFields>().ok())
        .and_then(|f| f.image);
    from_rules
        .or(from_art)
        .as_deref()
        .unwrap_or(type_id)
        .trim()
        .to_ascii_uppercase()
}

/// 步兵朝向字节 → SHP 朝向槽（0..=7）。
pub fn infantry_facing_slot(facing: u8) -> u16 {
    let step = (((u16::from(facing) >> 2) + 1) >> 1) as usize & 0x1F;
    u16::from(INFANTRY_FACING_SLOT_TABLE[step])
}

/// 解析 art 序列值 `Start,Count,FacingsOrMultiplier`；第三字段为朝向步长。
pub fn parse_sequence_triple(raw: &str) -> Option<(u16, u16, u16)> {
    let row: SequenceTripleRow = from_row(raw).ok()?;
    Some((row.start, row.count, row.multiplier))
}

#[derive(Debug, Deserialize)]
struct SequenceTripleRow {
    start: u16,
    count: u16,
    multiplier: u16,
}

#[doc(hidden)]
pub fn sequence_section_name(art: &IniDocument, image_key: &str) -> Option<String> {
    art.section(image_key)
        .and_then(|s| s.deserialize::<MobileArtImageFields>().ok())
        .and_then(|f| f.sequence)
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
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

fn sequence_triples_from_section(art: &IniDocument, seq_section: &str) -> (Option<(u16, u16, u16)>, Option<(u16, u16, u16)>) {
    let fields = art
        .section(seq_section)
        .and_then(|s| s.deserialize::<MobileSequenceSectionFields>().ok())
        .unwrap_or_default();
    let walk_triple = fields.walk.or(fields.panic).as_deref().and_then(parse_sequence_triple);
    let ready_triple = fields.ready.or(fields.guard).as_deref().and_then(parse_sequence_triple);
    (walk_triple, ready_triple)
}

#[derive(Debug, Default, Deserialize)]
struct MobileSequenceSectionFields {
    #[serde(rename = "Walk")]
    walk: Option<String>,
    #[serde(rename = "Panic")]
    panic: Option<String>,
    #[serde(rename = "Ready")]
    ready: Option<String>,
    #[serde(rename = "Guard")]
    guard: Option<String>,
}

/// 由姿态与 art 序列解析 SHP 帧；无序列时回退到朝向桶。
pub fn resolve_mobile_shp_frame(art: Option<&IniDocument>, image_key: &str, ent: &MapEntity, pose: MobilePaintPose) -> u16 {
    let hints = MobileTypePaintHints {
        image_key: image_key.to_string(),
        prefer_voxel: false,
        new_theater: false,
        walk_triple: art.and_then(|a| {
            let seq = sequence_section_name(a, image_key)?;
            sequence_triples_from_section(a, &seq).0
        }),
        ready_triple: art.and_then(|a| {
            let seq = sequence_section_name(a, image_key)?;
            sequence_triples_from_section(a, &seq).1
        }),
    };
    resolve_mobile_shp_frame_from_hints(&hints, ent, pose)
}

fn resolve_mobile_shp_frame_from_hints(hint: &MobileTypePaintHints, ent: &MapEntity, pose: MobilePaintPose) -> u16 {
    // 载具 WalkFrames 等另议；步兵靠 `Sequence=`。
    if ent.kind != MapEntityKind::Infantry {
        return u16::from(ent.facing / 32);
    }
    let Some((start, count, multiplier)) = (if pose.moving { hint.walk_triple } else { hint.ready_triple })
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
