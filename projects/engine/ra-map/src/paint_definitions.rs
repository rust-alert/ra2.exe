//! 绘制装载结果：art/rules 文档与已固化的叠画相关规则。
//!
//! 目标是演进为不含 `IniDocument` 的强类型绘制定义；当前仍暂存 art/rules
//! 供尚未迁完的叠画路径使用，但字段仅 crate 内可见。

use ra_assets::{IniDocument, IniMergePolicy, LayeredIniView};
use ra_types::AssetSource;

use crate::structure_damage::StructureDamageRules;
use crate::structure_paint::StructurePaintHintTable;

/// 某类型建造栏图标候选资源名（PCX 优先，再 SHP）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CameoAssetNames {
    /// `CameoPCX=` 及回退候选（含扩展名）。
    pub pcx: Vec<String>,
    /// `Cameo=` / `AltCameo=` 及 `{type}icon.shp` 等回退。
    pub shp: Vec<String>,
}

/// 绘制侧装载结果（受损规则已固化；art/rules 文档为过渡持有）。
#[derive(Debug, Clone, Default)]
pub struct PaintDefinitions {
    /// 自底向顶的 art 层（crate 内过渡持有）。
    art_layers: Vec<IniDocument>,
    /// 自底向顶的 rules 层（crate 内过渡持有）。
    rules_layers: Vec<IniDocument>,
    /// 顶层 art（叠画 hint 路径过渡使用；cameo / 装载已走层叠）。
    pub(crate) art: Option<IniDocument>,
    /// 顶层 rules（叠画 hint 路径过渡使用）。
    pub(crate) rules: Option<IniDocument>,
    /// 从 rules 层叠一次解出的建筑受损阈值 / 火焰类型（无 rules 时为缺省）。
    pub damage: StructureDamageRules,
    /// 建筑类型叠画提示表（跨 paint / anim-bank / buildup 复用）。
    pub(crate) structure_hints: StructurePaintHintTable,
}

impl PaintDefinitions {
    /// 从资源源各读一次 art / rules（缺文件则为空层），并固化受损规则。
    pub fn load(source: &dyn AssetSource, art_ini: &str, rules_ini: &str) -> Self {
        Self::load_files(source, &[art_ini], &[rules_ini])
    }

    /// 按自底向顶文件名列表装载 art / rules（缺文件跳过），并固化受损规则。
    ///
    /// 与 `ResourceChain` 的 underlay → primary 顺序对齐；叠画 hint 暂仍看顶层文档。
    pub fn load_files(source: &dyn AssetSource, art_files: &[&str], rules_files: &[&str]) -> Self {
        let art_layers = read_ini_layers(source, art_files);
        let rules_layers = read_ini_layers(source, rules_files);
        let art = art_layers.last().cloned();
        let rules = rules_layers.last().cloned();
        let damage = StructureDamageRules::from_rules_layers(&rules_layers);
        Self {
            art_layers,
            rules_layers,
            art,
            rules,
            damage,
            structure_hints: StructurePaintHintTable::default(),
        }
    }

    /// 是否已装入任一层 rules 文档。
    pub(crate) fn has_rules(&self) -> bool {
        !self.rules_layers.is_empty()
    }

    /// 从层叠 art 解析建造栏图标候选名（无 art 时仅类型 id 回退）。
    pub fn cameo_asset_names(&self, type_id: &str) -> CameoAssetNames {
        let mut pcx = Vec::new();
        let mut shp = Vec::new();
        if !self.art_layers.is_empty() {
            let policy = IniMergePolicy::last_wins();
            let art = LayeredIniView::new(&self.art_layers, &policy);
            push_cameo_pcx_name(&mut pcx, art.get(type_id, "CameoPCX").map(|v| v.raw));
            push_cameo_shp_name(&mut shp, art.get(type_id, "Cameo").map(|v| v.raw));
            let image_key = art.get(type_id, "Image").map(|v| v.raw).unwrap_or(type_id);
            if !image_key.eq_ignore_ascii_case(type_id) {
                push_cameo_pcx_name(&mut pcx, art.get(image_key, "CameoPCX").map(|v| v.raw));
                push_cameo_shp_name(&mut shp, art.get(image_key, "Cameo").map(|v| v.raw));
            }
            push_cameo_shp_name(&mut shp, art.get(type_id, "AltCameo").map(|v| v.raw));
        }
        shp.push(format!("{type_id}icon.shp"));
        shp.push(format!("{type_id}.shp"));
        CameoAssetNames { pcx, shp }
    }
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
