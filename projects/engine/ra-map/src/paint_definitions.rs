//! 绘制装载结果：已固化的叠画 / 图标提示与受损规则。
//!
//! 装载请走 [`PaintDefinitionsLoader`]（持有开放 art/rules）；经
//! [`PaintDefinitionsLoader::seal_with_runtime`] / [`PaintDefinitionsLoader::drop_documents`]
//! 产出宿主侧 [`PaintDefinitions`]（**不**保存 `IniDocument`）。产品 / 预览路径只消费后者。

use std::collections::HashMap;

use ra_assets::{IniDocument, IniMergePolicy, materialize_ini_layers};
use ra_types::{AssetSource, RuntimeDefinitions, TechnoClass};

use crate::{
    MapEntityKind, MapInfo,
    mobile_paint::MobilePaintHintTable,
    overlay_paint::{OverlayPaintHintTable, flat_tiberium_display_names, flat_tiberium_display_type_name},
    structure_damage::StructureDamageRules,
    structure_paint::{StructureAnimHintTable, StructurePaintHintTable},
    terrain_paint::TerrainPaintHintTable,
};

/// 某类型建造栏图标候选资源名（PCX 优先，再 SHP）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CameoAssetNames {
    /// `CameoPCX=` 及回退候选（含扩展名）。
    pub pcx: Vec<String>,
    /// `Cameo=` / `AltCameo=` 及 `{type}icon.shp` 等回退。
    pub shp: Vec<String>,
}

/// 建造栏图标候选名提示表（按类型 id 复用）。
#[derive(Debug, Clone, Default)]
struct CameoPaintHintTable {
    by_type: HashMap<String, CameoAssetNames>,
}

impl CameoPaintHintTable {
    fn get(&self, type_id: &str) -> Option<&CameoAssetNames> {
        self.by_type.get(type_id)
    }

    fn contains(&self, type_id: &str) -> bool {
        self.by_type.contains_key(type_id)
    }

    fn insert(&mut self, type_id: String, names: CameoAssetNames) {
        self.by_type.insert(type_id, names);
    }
}

/// 宿主侧绘制结果：仅强类型 hint / 受损规则，不含 art/rules `IniDocument`。
#[derive(Debug, Clone, Default)]
pub struct PaintDefinitions {
    /// 从 rules 一次解出的建筑受损阈值 / 火焰类型（无 rules 时为缺省）。
    pub damage: StructureDamageRules,
    /// 建筑类型叠画提示表（跨 paint / anim-bank / buildup 复用）。
    pub(crate) structure_hints: StructurePaintHintTable,
    /// 建筑活动层 art 节提示表（按 anim 节名复用）。
    pub(crate) structure_anim_hints: StructureAnimHintTable,
    /// 移动单位类型叠画提示表（跨 paint 调用复用）。
    pub(crate) mobile_hints: MobilePaintHintTable,
    /// 地形物件类型叠画提示表（跨 paint / anim-bank 复用）。
    pub(crate) terrain_hints: TerrainPaintHintTable,
    /// Overlay 类型叠画提示表（按类型名 + 显示名，跨 paint 调用复用）。
    pub(crate) overlay_hints: OverlayPaintHintTable,
    /// 建造栏图标候选名表（跨侧栏刷新复用）。
    cameo_hints: CameoPaintHintTable,
    /// 绘制时主体 SHP 加载失败的建筑类型键（去重）。
    missing_structure_shp: Vec<String>,
}

/// 装载期 staging：持有开放 art/rules，seal / drop 后交出无文档的 [`PaintDefinitions`]。
#[derive(Debug, Clone)]
pub struct PaintDefinitionsLoader {
    art: Option<IniDocument>,
    rules: Option<IniDocument>,
    paint: PaintDefinitions,
}

impl PaintDefinitionsLoader {
    /// 从资源源各读一次 art / rules（缺文件则为空），并固化受损规则。
    pub fn load(source: &dyn AssetSource, art_ini: &str, rules_ini: &str) -> Self {
        Self::load_files(source, &[art_ini], &[rules_ini])
    }

    /// 按自底向顶文件名列表装载 art / rules（缺文件跳过），合并为单文档后固化受损规则。
    ///
    /// 与 `ResourceChain` 的 underlay → primary 顺序对齐。
    pub fn load_files(source: &dyn AssetSource, art_files: &[&str], rules_files: &[&str]) -> Self {
        let policy = IniMergePolicy::last_wins();
        let art_layers = read_ini_layers(source, art_files);
        let rules_layers = read_ini_layers(source, rules_files);
        let art = materialize_ini_layers(&art_layers, &policy);
        let rules = materialize_ini_layers(&rules_layers, &policy);
        let damage = rules.as_ref().map(StructureDamageRules::from_rules_doc).unwrap_or_default();
        Self {
            art,
            rules,
            paint: PaintDefinitions {
                damage,
                structure_hints: StructurePaintHintTable::default(),
                structure_anim_hints: StructureAnimHintTable::default(),
                mobile_hints: MobilePaintHintTable::default(),
                terrain_hints: TerrainPaintHintTable::default(),
                overlay_hints: OverlayPaintHintTable::default(),
                cameo_hints: CameoPaintHintTable::default(),
                missing_structure_shp: Vec::new(),
            },
        }
    }

    /// [`Self::load`] 后立即按定义与地图 seal，供预览 / 测试走与 boot 相同的无文档路径。
    pub fn load_sealed(
        source: &dyn AssetSource,
        art_ini: &str,
        rules_ini: &str,
        defs: &RuntimeDefinitions,
        map: &MapInfo,
    ) -> PaintDefinitions {
        Self::load(source, art_ini, rules_ini).seal_with_runtime(defs, map)
    }

    /// 装载后按地图 overlay 与类型回调预填 hint，再丢弃文档（overlay 测试 / 无完整 `OverlayTypeRegistry` 时用）。
    pub fn load_sealed_for_overlays(
        source: &dyn AssetSource,
        art_ini: &str,
        rules_ini: &str,
        map: &MapInfo,
        overlay_type_name: &dyn Fn(u8) -> Option<String>,
        is_tiberium: &dyn Fn(u8) -> bool,
    ) -> PaintDefinitions {
        let mut loader = Self::load(source, art_ini, rules_ini);
        loader.preload_map_overlays(map, overlay_type_name, is_tiberium);
        loader.drop_documents()
    }

    /// 借用装载中已写入的 hint 表（不含开放文档）。
    pub fn paint(&self) -> &PaintDefinitions {
        &self.paint
    }

    /// 可变借用 hint 表（仅名称回退 `ensure_*`；读 INI 请用本 loader 的解析入口）。
    pub fn paint_mut(&mut self) -> &mut PaintDefinitions {
        &mut self.paint
    }

    /// staging 仍持有 art/rules（即便某侧文件缺失）。
    pub fn documents_sealed(&self) -> bool {
        false
    }

    /// 在仍持有 art/rules 时，按地图 overlay 格与类型回调写入 hint。
    pub fn preload_map_overlays(
        &mut self,
        map: &MapInfo,
        overlay_type_name: &dyn Fn(u8) -> Option<String>,
        is_tiberium: &dyn Fn(u8) -> bool,
    ) {
        let art = self.art.as_ref();
        let rules = self.rules.as_ref();
        for cell in &map.overlays {
            let Some(type_name) = overlay_type_name(cell.overlay_id)
            else {
                continue;
            };
            let display_name = if is_tiberium(cell.overlay_id) {
                flat_tiberium_display_type_name(&type_name, cell.x, cell.y)
            }
            else {
                type_name.clone()
            };
            self.paint.ensure_overlay_hint_with(art, rules, &type_name, &display_name);
        }
    }

    /// 解析建造栏图标候选名（写入 hint 后可在 seal / drop 后复用）。
    pub fn cameo_asset_names(&mut self, type_id: &str) -> CameoAssetNames {
        let art = self.art.as_ref();
        self.paint.ensure_cameo_hint_with(art, type_id);
        self.paint.cameo_asset_names(type_id)
    }

    /// 按冻结定义与地图填满叠画 / 图标 hint，然后丢弃 art/rules `IniDocument`。
    pub fn seal_with_runtime(mut self, defs: &RuntimeDefinitions, map: &MapInfo) -> PaintDefinitions {
        {
            let art = self.art.as_ref();
            let rules = self.rules.as_ref();
            let paint = &mut self.paint;
            for techno in defs.techno.iter() {
                match techno.class {
                    TechnoClass::Building => paint.ensure_structure_hint_with(art, rules, &techno.type_key),
                    TechnoClass::Infantry | TechnoClass::Vehicle | TechnoClass::Aircraft => {
                        paint.ensure_mobile_hint_with(art, rules, &techno.type_key)
                    }
                }
                paint.ensure_cameo_hint_with(art, techno.type_key.as_str());
            }
            for structure in defs.structures.iter() {
                paint.ensure_structure_hint_with(art, rules, &structure.type_key);
                paint.ensure_cameo_hint_with(art, structure.type_key.as_str());
            }
            for spawner in defs.terrain_spawners.iter() {
                paint.ensure_terrain_hint_with(art, rules, spawner.type_key.as_str());
            }
            for id in 0u8..=255 {
                let Some(type_name) = defs.overlays.name(id)
                else {
                    continue;
                };
                paint.ensure_overlay_hint_with(art, rules, type_name, type_name);
                if defs.overlays.is_harvestable(id) {
                    for display in flat_tiberium_display_names(type_name) {
                        paint.ensure_overlay_hint_with(art, rules, type_name, &display);
                    }
                }
            }
            for ent in &map.entities {
                match ent.kind {
                    MapEntityKind::Structure => paint.ensure_structure_hint_with(art, rules, &ent.type_id),
                    MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft => {
                        paint.ensure_mobile_hint_with(art, rules, &ent.type_id)
                    }
                }
                paint.ensure_cameo_hint_with(art, ent.type_id.as_str());
            }
            for obj in &map.terrain_objects {
                paint.ensure_terrain_hint_with(art, rules, obj.name.as_str());
            }
            for cell in &map.overlays {
                let Some(type_name) = defs.overlays.name(cell.overlay_id)
                else {
                    continue;
                };
                let display_name = if defs.overlays.is_harvestable(cell.overlay_id) {
                    flat_tiberium_display_type_name(type_name, cell.x, cell.y)
                }
                else {
                    type_name.to_string()
                };
                paint.ensure_overlay_hint_with(art, rules, type_name, &display_name);
            }
            paint.preload_structure_anim_hints_with(art);
        }
        self.drop_documents()
    }

    /// 丢弃 art/rules 文档（不扫描 hint），交出宿主侧 paint。
    pub fn drop_documents(self) -> PaintDefinitions {
        self.paint
    }
}

impl PaintDefinitions {
    /// 宿主侧 paint 永不持有 art/rules 文档。
    pub fn documents_sealed(&self) -> bool {
        true
    }

    /// 确保表中含该类型建造栏图标候选名（已有则跳过；无文档时仅类型名回退）。
    pub fn ensure_cameo_hint(&mut self, type_id: &str) {
        self.ensure_cameo_hint_with(None, type_id);
    }

    pub(crate) fn ensure_cameo_hint_with(&mut self, art: Option<&IniDocument>, type_id: &str) {
        if self.cameo_hints.contains(type_id) {
            return;
        }
        let names = resolve_cameo_asset_names(art, type_id);
        self.cameo_hints.insert(type_id.to_string(), names);
    }

    /// 解析建造栏图标候选名（缓存后复用；无 art 时仅类型 id 回退）。
    pub fn cameo_asset_names(&mut self, type_id: &str) -> CameoAssetNames {
        self.ensure_cameo_hint(type_id);
        self.cameo_hints.get(type_id).cloned().unwrap_or_else(|| resolve_cameo_asset_names(None, type_id))
    }

    /// seal 后仍无 art 节的建筑类型键（将回退占位叠画）。
    pub fn structure_types_missing_art(&self) -> Vec<&str> {
        self.structure_hints.types_missing_art()
    }

    /// seal 后仍无 art 节的移动单位类型键。
    pub fn mobile_types_missing_art(&self) -> Vec<&str> {
        self.mobile_hints.types_missing_art()
    }

    /// seal 后仍无 art 节的地形物件类型名。
    pub fn terrain_types_missing_art(&self) -> Vec<&str> {
        self.terrain_hints.types_missing_art()
    }

    /// seal 后仍无 art 节的 overlay 类型名。
    pub fn overlay_types_missing_art(&self) -> Vec<&str> {
        self.overlay_hints.types_missing_art()
    }

    /// 绘制期主体 SHP 加载失败的建筑类型键（已排序去重）。
    pub fn structure_types_missing_shp(&self) -> &[String] {
        &self.missing_structure_shp
    }

    /// 记录一次建筑主体 SHP 缺失（同类型只记一次）。
    pub(crate) fn note_missing_structure_shp(&mut self, type_id: &str) {
        let key = type_id.trim().to_ascii_uppercase();
        if key.is_empty() || self.missing_structure_shp.iter().any(|k| k == &key) {
            return;
        }
        self.missing_structure_shp.push(key);
        self.missing_structure_shp.sort_unstable();
    }
}

fn resolve_cameo_asset_names(art: Option<&IniDocument>, type_id: &str) -> CameoAssetNames {
    let mut pcx = Vec::new();
    let mut shp = Vec::new();
    if let Some(art) = art {
        push_cameo_pcx_name(&mut pcx, art.get(type_id, "CameoPCX"));
        push_cameo_shp_name(&mut shp, art.get(type_id, "Cameo"));
        let image_key = art.get(type_id, "Image").unwrap_or(type_id);
        if !image_key.eq_ignore_ascii_case(type_id) {
            push_cameo_pcx_name(&mut pcx, art.get(image_key, "CameoPCX"));
            push_cameo_shp_name(&mut shp, art.get(image_key, "Cameo"));
        }
        push_cameo_shp_name(&mut shp, art.get(type_id, "AltCameo"));
    }
    shp.push(format!("{type_id}icon.shp"));
    shp.push(format!("{type_id}.shp"));
    CameoAssetNames { pcx, shp }
}

fn push_cameo_shp_name(out: &mut Vec<String>, raw: Option<&str>) {
    let Some(c) = raw.map(str::trim).filter(|s| !s.is_empty())
    else {
        return;
    };
    if c.to_ascii_lowercase().ends_with(".shp") {
        out.push(c.to_string());
    }
    else {
        out.push(format!("{c}.shp"));
    }
}

fn push_cameo_pcx_name(out: &mut Vec<String>, raw: Option<&str>) {
    let Some(c) = raw.map(str::trim).filter(|s| !s.is_empty())
    else {
        return;
    };
    if c.to_ascii_lowercase().ends_with(".pcx") {
        out.push(c.to_string());
    }
    else {
        out.push(format!("{c}.pcx"));
    }
}

/// 读取并解析单个 INI；失败返回 `None`。
pub fn read_optional_ini(source: &dyn AssetSource, name: &str) -> Option<IniDocument> {
    source.read(name).ok().and_then(|b| IniDocument::parse(&b).ok())
}

/// 按自底向顶顺序读取 INI；缺文件或解析失败则跳过该层。
fn read_ini_layers(source: &dyn AssetSource, names: &[&str]) -> Vec<IniDocument> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(doc) = read_optional_ini(source, name) {
            out.push(doc);
        }
    }
    out
}
