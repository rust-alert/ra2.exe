//! 遭遇战启动预览：地形 + overlay + 物件 + 建筑；会话播种后可再叠移动单位 SHP。

use image::RgbaImage;
use ra_assets::{Hsv, IniDocument, Palette};
use ra_types::AssetSource;

use crate::{
    MapInfo, MobilePaintPose, OverlayLayerFilter, StructureAnimBank, StructureAnimMode, StructureLightTable, TerrainAnimBank, TerrainPaintMode,
    compose::TerrainImage,
    fallback_preview::RawRgbaImage,
    mobile_paint::paint_map_mobiles,
    overlay_paint::paint_map_overlays,
    structure_paint::{collect_structure_anim_bank, paint_map_structures, paint_structure_anim_bank},
    terrain_paint::{
        collect_ore_tree_anim_bank, collect_terrain_anim_bank, format_terrain_anim_layer_diag, paint_map_terrain_objects,
        paint_ore_tree_frames, paint_terrain_anim_bank,
    },
    terrain_preview::compose_terrain_preview,
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
    /// 不含建筑/地形活动层与矿柱的预览底图（对局时钟刷新用）。
    pub base_without_anims: RgbaImage,
    /// 不含可采 overlay 的定格底图（已含物件/建筑/桥；产矿与采集脏刷新 underlay）。
    pub ore_underlay: RgbaImage,
    /// 建筑活动层银行。
    pub anim_bank: StructureAnimBank,
    /// 动画地形物件银行（旗帜等常循环）。
    pub terrain_anim_bank: TerrainAnimBank,
    /// 矿柱帧银行（由产矿状态机选帧）。
    pub ore_tree_anim_bank: TerrainAnimBank,
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
/// 顺序：非可采地面 overlay →（分叉 underlay）→ 可采 overlay（仅主图）→
/// 静态地形物件 / 建筑 / 桥（主图与 underlay 同步）→ 移动单位（仅主图）→
/// 动画地形 → 矿柱 Idle → 建筑活动层。
/// `ore_underlay` = 无可采矿的定格层，脏刷新时 `clone` 后再叠当前可采矿即可。
///
/// 返回 `(合成图, 无活动层底图, 矿 underlay, 统计, 建筑活动层, 地形活动层, 矿柱银行)`。
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
) -> Option<(TerrainImage, RgbaImage, RgbaImage, SkirmishPreviewStats, StructureAnimBank, TerrainAnimBank, TerrainAnimBank)> {
    // 预览叠画需要点光源；从 rules 收集后挂到地图副本上（不改调用方 MapInfo）。
    let mut lit_map = map.clone();
    if let Ok(bytes) = source.read(rules_ini) {
        if let Ok(doc) = IniDocument::parse(&bytes) {
            lit_map.refresh_point_lights(&StructureLightTable::from_rules_ini(&doc));
        }
    }
    let map = &lit_map;

    let mut map_non_ore = map.clone();
    map_non_ore.overlays.retain(|c| !is_tiberium(c.overlay_id));
    let mut map_ore = map.clone();
    map_ore.overlays.retain(|c| is_tiberium(c.overlay_id));

    let mut image = compose_terrain_preview(source, map)?;
    let (ground_non_ore_shp, ground_non_ore_mark) = paint_map_overlays(
        source,
        &map_non_ore,
        &mut image,
        art_ini,
        rules_ini,
        overlay_type_name,
        is_tiberium,
        tiberium_hsv,
        OverlayLayerFilter::Ground,
    );
    let mut underlay = TerrainImage { image: image.image.clone(), drawn: 0, origin_x: image.origin_x, origin_y: image.origin_y };
    let (ground_ore_shp, ground_ore_mark) = paint_map_overlays(
        source,
        &map_ore,
        &mut image,
        art_ini,
        rules_ini,
        overlay_type_name,
        is_tiberium,
        tiberium_hsv,
        OverlayLayerFilter::Ground,
    );
    let terrain_objects = paint_map_terrain_objects(source, map, &mut image, art_ini, rules_ini, TerrainPaintMode::StaticOnly);
    let _ = paint_map_terrain_objects(source, map, &mut underlay, art_ini, rules_ini, TerrainPaintMode::StaticOnly);
    let terrain_anim_bank = collect_terrain_anim_bank(source, map, art_ini, rules_ini);
    let ore_tree_anim_bank = collect_ore_tree_anim_bank(source, map, art_ini, rules_ini);
    let (structures, structure_mark) =
        paint_map_structures(source, map, &mut image, art_ini, rules_ini, remap_owner, StructureAnimMode::BodyOnly);
    let _ = paint_map_structures(source, map, &mut underlay, art_ini, rules_ini, remap_owner, StructureAnimMode::BodyOnly);
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
    let _ = paint_map_overlays(
        source,
        map,
        &mut underlay,
        art_ini,
        rules_ini,
        overlay_type_name,
        is_tiberium,
        tiberium_hsv,
        OverlayLayerFilter::Bridge,
    );
    let ore_underlay = underlay.image;
    let anim_bank = collect_structure_anim_bank(source, map, art_ini, rules_ini, remap_owner);
    let mobiles = paint_map_mobiles(source, map, &mut image, art_ini, rules_ini, remap_owner, &|_| MobilePaintPose::default());
    let base_without_anims = image.image.clone();
    // 旗帜等常循环地形在刷新时叠在建筑主体之上。
    let terrain_anim_n = paint_terrain_anim_bank(&mut image, &terrain_anim_bank, anim_clock_ms);
    let ore_idle: Vec<(u16, u16, u16)> = ore_tree_anim_bank.layers.iter().map(|l| (l.x, l.y, 0)).collect();
    let ore_n = paint_ore_tree_frames(&mut image, &ore_tree_anim_bank, &ore_idle);
    let anim_n = paint_structure_anim_bank(&mut image, &anim_bank, anim_clock_ms);
    Some((
        image,
        base_without_anims,
        ore_underlay,
        SkirmishPreviewStats {
            overlay_shp: ground_non_ore_shp + ground_ore_shp + bridge_shp,
            overlay_mark: ground_non_ore_mark + ground_ore_mark + bridge_mark,
            terrain_objects: terrain_objects + terrain_anim_n + ore_n,
            terrain_anims: terrain_anim_bank.layers.len() + ore_tree_anim_bank.layers.len(),
            structures: structures + anim_n,
            structure_mark,
            mobiles,
        },
        anim_bank,
        terrain_anim_bank,
        ore_tree_anim_bank,
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
    tiberium_hsv: &dyn Fn(u8) -> Option<Hsv>,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> Option<BootPreviewResult> {
    let (image, base_without_anims, ore_underlay, stats, anim_bank, terrain_anim_bank, ore_tree_anim_bank) =
        compose_skirmish_preview(source, map, art_ini, rules_ini, overlay_type_name, is_tiberium, tiberium_hsv, remap_owner, 0)?;
    let terrain_hit = terrain_anim_bank
        .layers
        .first()
        .map(|l| format!("{} {}x{} body#{}/{} pal={}", l.file, l.canvas_width, l.canvas_height, l.frames.len(), l.shp_frames, l.palette))
        .unwrap_or_else(|| "-".into());
    let ore_hit = ore_tree_anim_bank.layers.first().map(|l| format_terrain_anim_layer_diag(l, 0, false, true)).unwrap_or_else(|| "-".into());
    let note = format!(
        "map:{} cells={} drawn={} overlay#{} shp#{} mark#{} terrain_shp#{} terrain_anim#{} ({}) ore_tree#{} ({}) struct_shp#{} struct_miss#{} mobile_shp#{} anim#{} {}x{}",
        map.name,
        map.cells.len(),
        image.drawn,
        map.overlays.len(),
        stats.overlay_shp,
        stats.overlay_mark,
        stats.terrain_objects,
        stats.terrain_anims,
        terrain_hit,
        ore_tree_anim_bank.layers.len(),
        ore_hit,
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
        ore_underlay,
        anim_bank,
        terrain_anim_bank,
        ore_tree_anim_bank,
        note,
        stats,
    })
}
