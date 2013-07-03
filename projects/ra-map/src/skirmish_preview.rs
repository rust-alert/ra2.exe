//! 遭遇战启动预览：地形 + overlay + 物件 + 建筑（移动单位走动态标记）。

use ra_assets::Palette;
use ra_types::AssetSource;

use crate::compose::TerrainImage;
use crate::fallback_preview::{
    load_fallback_theater_tile, load_fallback_unit_sprite, RawRgbaImage,
};
use crate::overlay_paint::paint_map_overlays;
use crate::structure_paint::paint_map_structures;
use crate::terrain_paint::paint_map_terrain_objects;
use crate::terrain_preview::compose_terrain_preview;
use crate::MapInfo;

/// 各叠画层统计（供 boot 注记）。
#[derive(Debug, Clone, Default)]
pub struct SkirmishPreviewStats {
    pub overlay_shp: usize,
    pub overlay_mark: usize,
    pub terrain_objects: usize,
    pub structures: usize,
    pub mobiles: usize,
}

/// 启动预览合成结果（含注记与原点）。
#[derive(Debug, Clone)]
pub struct BootPreviewResult {
    pub image: RawRgbaImage,
    pub origin_x: i32,
    pub origin_y: i32,
    pub note: String,
    pub stats: SkirmishPreviewStats,
}

/// 合成启动预览图（地形 / overlay / 物件 / 建筑；移动单位由渲染层动态标记）。
pub fn compose_skirmish_preview(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> Option<(TerrainImage, SkirmishPreviewStats)> {
    let mut image = compose_terrain_preview(source, map)?;
    let (overlay_shp, overlay_mark) =
        paint_map_overlays(source, map, &mut image, art_ini, overlay_type_name);
    let terrain_objects = paint_map_terrain_objects(source, map, &mut image, art_ini);
    let structures = paint_map_structures(source, map, &mut image, art_ini, remap_owner);
    Some((
        image,
        SkirmishPreviewStats {
            overlay_shp,
            overlay_mark,
            terrain_objects,
            structures,
            mobiles: 0,
        },
    ))
}

/// 合成启动预览；失败时回退剧院砖或单位精灵。
pub fn compose_boot_preview(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> Option<BootPreviewResult> {
    if let Some((image, stats)) =
        compose_skirmish_preview(source, map, art_ini, overlay_type_name, remap_owner)
    {
        let note = format!(
            "map:{} cells={} drawn={} overlay#{} shp#{} mark#{} terrain_shp#{} struct_shp#{} mobile_shp#{} {}x{}",
            map.name,
            map.cells.len(),
            image.drawn,
            map.overlays.len(),
            stats.overlay_shp,
            stats.overlay_mark,
            stats.terrain_objects,
            stats.structures,
            stats.mobiles,
            image.width,
            image.height
        );
        return Some(BootPreviewResult {
            origin_x: image.origin_x,
            origin_y: image.origin_y,
            image: RawRgbaImage {
                label: note.clone(),
                width: image.width,
                height: image.height,
                pixels: image.pixels,
            },
            note,
            stats,
        });
    }

    let fallback = load_fallback_theater_tile(source, map.theater)
        .or_else(|| load_fallback_unit_sprite(source))?;
    Some(BootPreviewResult {
        note: fallback.label.clone(),
        origin_x: 0,
        origin_y: 0,
        image: fallback,
        stats: SkirmishPreviewStats::default(),
    })
}
