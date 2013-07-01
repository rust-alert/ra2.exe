//! 遭遇战启动预览：地形 + overlay + 物件 + 建筑（移动单位走动态标记）。

use ra_assets::Palette;
use ra_types::AssetSource;

use crate::compose::TerrainImage;
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
