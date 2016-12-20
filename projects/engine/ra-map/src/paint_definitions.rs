//! 绘制装载结果：art/rules 文档与已固化的叠画相关规则。
//!
//! 目标是演进为不含 `IniDocument` 的强类型绘制定义；当前仍暂存 art/rules
//! 供尚未迁完的叠画路径使用，但字段仅 crate 内可见。

use ra_assets::IniDocument;
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
    /// `art*.ini` 解析结果（crate 内过渡持有）。
    pub(crate) art: Option<IniDocument>,
    /// `rules*.ini` 解析结果（crate 内过渡持有）。
    pub(crate) rules: Option<IniDocument>,
    /// 从 rules 一次解出的建筑受损阈值 / 火焰类型（无 rules 时为缺省）。
    pub damage: StructureDamageRules,
    /// 建筑类型叠画提示表（跨 paint / anim-bank / buildup 复用）。
    pub(crate) structure_hints: StructurePaintHintTable,
}

impl PaintDefinitions {
    /// 从资源源各读一次 art / rules（缺文件则为 `None`），并固化受损规则。
    pub fn load(source: &dyn AssetSource, art_ini: &str, rules_ini: &str) -> Self {
        let rules = read_optional_ini(source, rules_ini);
        let damage = rules.as_ref().map(StructureDamageRules::from_rules_doc).unwrap_or_default();
        Self {
            art: read_optional_ini(source, art_ini),
            rules,
            damage,
            structure_hints: StructurePaintHintTable::default(),
        }
    }

    /// 是否已装入 rules 文档。
    pub(crate) fn has_rules(&self) -> bool {
        self.rules.is_some()
    }

    /// 从暂存 art 解析建造栏图标候选名（无 art 时仅类型 id 回退）。
    pub fn cameo_asset_names(&self, type_id: &str) -> CameoAssetNames {
        let mut pcx = Vec::new();
        let mut shp = Vec::new();
        if let Some(art) = self.art.as_ref() {
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
