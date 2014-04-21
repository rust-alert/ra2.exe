//! 遭遇战启动预览：地形 + overlay + 物件 + 建筑（移动单位走动态标记）。

use ra_assets::Palette;
use ra_types::AssetSource;

use crate::{
    MapInfo, compose::TerrainImage, fallback_preview::RawRgbaImage, overlay_paint::paint_map_overlays,
    structure_paint::paint_map_structures, terrain_paint::paint_map_terrain_objects, terrain_preview::compose_terrain_preview,
};

/// 各叠画层统计（供 boot 注记）。
#[derive(Debug, Clone, Default)]
pub struct SkirmishPreviewStats {
    /// Overlay SHP 画上的格数。
    pub overlay_shp: usize,
    /// Overlay 色块回退格数。
    pub overlay_mark: usize,
    /// 地形物件叠画数。
    pub terrain_objects: usize,
    /// 建筑叠画数。
    pub structures: usize,
    /// 移动单位叠画数（启动预览常为 0）。
    pub mobiles: usize,
}

/// 启动预览合成结果（含注记与原点）。
#[derive(Debug, Clone)]
pub struct BootPreviewResult {
    /// 预览图像。
    pub image: RawRgbaImage,
    /// 画布原点世界 X。
    pub origin_x: i32,
    /// 画布原点世界 Y。
    pub origin_y: i32,
    /// 人类可读注记。
    pub note: String,
    /// 各层统计。
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
    let (overlay_shp, overlay_mark) = paint_map_overlays(source, map, &mut image, art_ini, overlay_type_name);
    let terrain_objects = paint_map_terrain_objects(source, map, &mut image, art_ini);
    let structures = paint_map_structures(source, map, &mut image, art_ini, remap_owner);
    Some((image, SkirmishPreviewStats { overlay_shp, overlay_mark, terrain_objects, structures, mobiles: 0 }))
}

/// 合成启动预览。
///
/// 仅在真实地形合成成功时返回 `Some`。**禁止**在失败后用剧院单砖或单位 SHP 冒充地图预览；
/// 显式探测请直接调用 `load_fallback_theater_tile` / `load_fallback_unit_sprite`。
pub fn compose_boot_preview(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> Option<BootPreviewResult> {
    let (image, stats) = compose_skirmish_preview(source, map, art_ini, overlay_type_name, remap_owner)?;
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
        image.image.width(),
        image.image.height()
    );
    Some(BootPreviewResult {
        origin_x: image.origin_x,
        origin_y: image.origin_y,
        image: RawRgbaImage { label: note.clone(), image: image.image },
        note,
        stats,
    })
}
