//! 遭遇战启动预览：地形 + overlay + 物件 + 建筑；会话播种后可再叠移动单位 SHP。

use image::RgbaImage;
use ra_assets::{Hsv, IniDocument, Palette};
use ra_types::AssetSource;

use crate::{
    MapInfo, MobilePaintPose, OverlayLayerFilter, StructureAnimBank, StructureAnimMode, TerrainAnimBank, TerrainPaintMode,
    compose::TerrainImage, fallback_preview::RawRgbaImage, mobile_paint::paint_map_mobiles,
    overlay_paint::paint_map_overlays, structure_paint::collect_structure_anim_bank, structure_paint::paint_map_structures,
    structure_paint::paint_structure_anim_bank, terrain_paint::collect_terrain_anim_bank, terrain_paint::paint_map_terrain_objects,
    terrain_paint::paint_terrain_anim_bank, terrain_preview::compose_terrain_preview,
};

/// 各叠画层统计（供 boot 注记）。
#[derive(Debug, Clone, Default)]
pub struct SkirmishPreviewStats {
    /// Overlay SHP 画上的格数。
    pub overlay_shp: usize,
    /// Overlay 色块回退格数。
    pub overlay_mark: usize,
    /// 静态地形物件叠画数。
    pub terrain_objects: usize,
    /// 动画地形物件层数。
    pub terrain_anims: usize,
    /// 建筑叠画数（含 bib / 炮塔等）。
    pub structures: usize,
    /// 建筑主体缺失占位色块数。
    pub structure_mark: usize,
    /// 移动单位叠画数。
    pub mobiles: usize,
}

/// 启动预览合成结果（含注记与原点）。
#[derive(Debug, Clone)]
pub struct BootPreviewResult {
    /// 预览图像（已叠当前时钟活动层）。
    pub image: RawRgbaImage,
    /// 不含建筑/地形活动层的预览底图（对局时钟刷新用）。
    pub base_without_anims: RgbaImage,
    /// 建筑活动层银行。
    pub anim_bank: StructureAnimBank,
    /// 动画地形物件银行（矿柱等）。
    pub terrain_anim_bank: TerrainAnimBank,
    /// 画布原点世界 X。
    pub origin_x: i32,
    /// 画布原点世界 Y。
    pub origin_y: i32,
    /// 人类可读注记。
    pub note: String,
    /// 各层统计。
    pub stats: SkirmishPreviewStats,
}

/// 合成启动预览图（地形 / overlay / 物件 / 建筑；地图放置段里的移动单位一并叠画）。
///
/// 顺序：地面 overlay → 静态地形物件 → 建筑主体 → 桥 overlay → 移动单位 →
/// 动画地形 → 建筑活动层。底图不含后两层，供对局按时钟刷新。
///
/// 返回 `(合成图, 无活动层底图, 统计, 建筑活动层, 地形活动层)`。
pub fn compose_skirmish_preview(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    rules_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    is_tiberium: &dyn Fn(u8) -> bool,
    tiberium_hsv: &dyn Fn(u8) -> Option<Hsv>,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
    anim_clock_ms: u64,
) -> Option<(TerrainImage, RgbaImage, SkirmishPreviewStats, StructureAnimBank, TerrainAnimBank)> {
    // 预览叠画需要点光源；从 rules 收集后挂到地图副本上（不改调用方 MapInfo）。
    let mut lit_map = map.clone();
    if let Ok(bytes) = source.read(rules_ini) {
        if let Ok(doc) = IniDocument::parse(&bytes) {
            lit_map.refresh_point_lights(&doc);
        }
    }
    let map = &lit_map;

    let mut image = compose_terrain_preview(source, map)?;
    let (ground_shp, ground_mark) = paint_map_overlays(
        source,
        map,
        &mut image,
        art_ini,
        rules_ini,
        overlay_type_name,
        is_tiberium,
        tiberium_hsv,
        OverlayLayerFilter::Ground,
    );
    let terrain_objects =
        paint_map_terrain_objects(source, map, &mut image, art_ini, rules_ini, TerrainPaintMode::StaticOnly);
    let terrain_anim_bank = collect_terrain_anim_bank(source, map, art_ini, rules_ini);
    let (structures, structure_mark) =
        paint_map_structures(source, map, &mut image, art_ini, rules_ini, remap_owner, StructureAnimMode::BodyOnly);
    let (bridge_shp, bridge_mark) = paint_map_overlays(
        source,
        map,
        &mut image,
        art_ini,
        rules_ini,
        overlay_type_name,
        is_tiberium,
        tiberium_hsv,
        OverlayLayerFilter::Bridge,
    );
    let anim_bank = collect_structure_anim_bank(source, map, art_ini, rules_ini, remap_owner);
    let mobiles = paint_map_mobiles(source, map, &mut image, art_ini, rules_ini, remap_owner, &|_| MobilePaintPose::default());
    let base_without_anims = image.image.clone();
    // 矿柱等动画地形在刷新时叠在建筑主体之上；矿柱极少与建筑同格，可接受。
    let terrain_anim_n = paint_terrain_anim_bank(&mut image, &terrain_anim_bank, anim_clock_ms);
    let anim_n = paint_structure_anim_bank(&mut image, &anim_bank, anim_clock_ms);
    Some((
        image,
        base_without_anims,
        SkirmishPreviewStats {
            overlay_shp: ground_shp + bridge_shp,
            overlay_mark: ground_mark + bridge_mark,
            terrain_objects: terrain_objects + terrain_anim_n,
            terrain_anims: terrain_anim_bank.layers.len(),
            structures: structures + anim_n,
            structure_mark,
            mobiles,
        },
        anim_bank,
        terrain_anim_bank,
    ))
}

/// 把额外实体（如航点播种的 MCV）叠画到已有预览 RGBA 上，保留原点。
///
/// 用于会话打开后按航点播种的初始单位叠画，不改 `MapInfo` 放置段。
pub fn paint_mobiles_onto_preview_rgba(
    source: &dyn AssetSource,
    entities_map: &MapInfo,
    image: &mut RgbaImage,
    origin_x: i32,
    origin_y: i32,
    art_ini: &str,
    rules_ini: &str,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
    pose_of: &dyn Fn(&crate::MapEntity) -> MobilePaintPose,
) -> usize {
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let n = paint_map_mobiles(source, entities_map, &mut terrain, art_ini, rules_ini, remap_owner, pose_of);
    *image = terrain.image;
    n
}

/// 合成启动预览。
///
/// 仅在真实地形合成成功时返回 `Some`。**禁止**在失败后用剧院单砖或单位 SHP 冒充地图预览；
/// 显式探测请直接调用 `load_fallback_theater_tile` / `load_fallback_unit_sprite`。
pub fn compose_boot_preview(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    rules_ini: &str,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    is_tiberium: &dyn Fn(u8) -> bool,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> Option<BootPreviewResult> {
    let (image, base_without_anims, stats, anim_bank, terrain_anim_bank) =
        compose_skirmish_preview(source, map, art_ini, rules_ini, overlay_type_name, is_tiberium, &|_| None, remap_owner, 0)?;
    let terrain_hit = terrain_anim_bank
        .layers
        .first()
        .map(|l| {
            format!(
                "{} {}x{} body#{}/{} pal={}",
                l.file, l.canvas_width, l.canvas_height, l.frames.len(), l.shp_frames, l.palette
            )
        })
        .unwrap_or_else(|| "-".into());
    let note = format!(
        "map:{} cells={} drawn={} overlay#{} shp#{} mark#{} terrain_shp#{} terrain_anim#{} ({}) struct_shp#{} struct_miss#{} mobile_shp#{} anim#{} {}x{}",
        map.name,
        map.cells.len(),
        image.drawn,
        map.overlays.len(),
        stats.overlay_shp,
        stats.overlay_mark,
        stats.terrain_objects,
        stats.terrain_anims,
        terrain_hit,
        stats.structures,
        stats.structure_mark,
        stats.mobiles,
        anim_bank.layers.len(),
        image.image.width(),
        image.image.height()
    );
    Some(BootPreviewResult {
        origin_x: image.origin_x,
        origin_y: image.origin_y,
        image: RawRgbaImage { label: note.clone(), image: image.image },
        base_without_anims,
        anim_bank,
        terrain_anim_bank,
        note,
        stats,
    })
}
