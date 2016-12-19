//! 绘制装载结果：art/rules 文档与已固化的叠画相关规则。
//!
//! 目标是演进为不含 `IniDocument` 的强类型绘制定义；当前仍暂存 art/rules
//! 供尚未迁完的叠画路径使用，但字段仅 crate 内可见。

use ra_assets::IniDocument;
use ra_types::AssetSource;

use crate::structure_damage::StructureDamageRules;
use crate::structure_paint::StructurePaintHintTable;

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

    /// 过渡：尚未迁出的 cameo 等路径只读 art 文档；新代码勿再扩散。
    #[doc(hidden)]
    pub fn art_document(&self) -> Option<&IniDocument> {
        self.art.as_ref()
    }

    /// 过渡：尚未迁出的路径只读 rules 文档；新代码勿再扩散。
    #[doc(hidden)]
    pub fn rules_document(&self) -> Option<&IniDocument> {
        self.rules.as_ref()
    }

    /// 是否已装入 rules 文档。
    pub(crate) fn has_rules(&self) -> bool {
        self.rules.is_some()
    }
}

/// 读取并解析单个 INI；失败返回 `None`。
pub fn read_optional_ini(source: &dyn AssetSource, name: &str) -> Option<IniDocument> {
    source.read(name).ok().and_then(|b| IniDocument::parse(&b).ok())
}
