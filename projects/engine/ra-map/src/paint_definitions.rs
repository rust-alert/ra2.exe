//! 绘制装载结果：合并后的 art/rules 与已固化的叠画相关规则。
//!
//! 目标是演进为不含 `IniDocument` 的强类型绘制定义；当前仍暂存合并后的
//! art/rules 文档，供叠画 hint 路径读取，字段仅 crate 内可见。

use std::collections::HashMap;

use ra_assets::{IniDocument, IniMergePolicy, materialize_ini_layers};
use ra_types::AssetSource;

use crate::mobile_paint::MobilePaintHintTable;
use crate::overlay_paint::OverlayPaintHintTable;
use crate::structure_damage::StructureDamageRules;
use crate::structure_paint::{StructureAnimHintTable, StructurePaintHintTable};
use crate::terrain_paint::TerrainPaintHintTable;

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

/// 绘制侧装载结果（受损规则已固化；art/rules 为过渡持有）。
#[derive(Debug, Clone, Default)]
pub struct PaintDefinitions {
    /// underlay→primary 合并后的 art（crate 内过渡持有）。
    art: Option<IniDocument>,
    /// underlay→primary 合并后的 rules（crate 内过渡持有）。
    rules: Option<IniDocument>,
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
}

impl PaintDefinitions {
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
            damage,
            structure_hints: StructurePaintHintTable::default(),
            structure_anim_hints: StructureAnimHintTable::default(),
            mobile_hints: MobilePaintHintTable::default(),
            terrain_hints: TerrainPaintHintTable::default(),
            overlay_hints: OverlayPaintHintTable::default(),
            cameo_hints: CameoPaintHintTable::default(),
        }
    }

    /// 是否已装入 art 文档。
    pub(crate) fn has_art(&self) -> bool {
        self.art.is_some()
    }

    /// 是否已装入 rules 文档。
    pub(crate) fn has_rules(&self) -> bool {
        self.rules.is_some()
    }

    /// 合并后的 art 文档（无则 `None`）。
    pub(crate) fn art_doc(&self) -> Option<&IniDocument> {
        self.art.as_ref()
    }

    /// 合并后的 rules 文档（无则 `None`）。
    pub(crate) fn rules_doc(&self) -> Option<&IniDocument> {
        self.rules.as_ref()
    }

    /// 确保表中含该类型建造栏图标候选名（已有则跳过 INI 扫描）。
    pub fn ensure_cameo_hint(&mut self, type_id: &str) {
        if self.cameo_hints.contains(type_id) {
            return;
        }
        let names = resolve_cameo_asset_names(self.art_doc(), type_id);
        self.cameo_hints.insert(type_id.to_string(), names);
    }

    /// 解析建造栏图标候选名（缓存后复用；无 art 时仅类型 id 回退）。
    pub fn cameo_asset_names(&mut self, type_id: &str) -> CameoAssetNames {
        self.ensure_cameo_hint(type_id);
        self.cameo_hints
            .get(type_id)
            .cloned()
            .unwrap_or_else(|| resolve_cameo_asset_names(None, type_id))
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
    } else {
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
    } else {
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
