//! 叠画路径共用的 art / rules INI 装载。

use ra_assets::IniDocument;
use ra_types::AssetSource;

use crate::StructureDamageRules;

/// 可选的 art + rules 文档（叠画侧解析 `Image` / `Voxel` 等键）。
#[derive(Debug, Clone, Default)]
pub struct PaintIniDocs {
    /// `art*.ini` 解析结果。
    pub art: Option<IniDocument>,
    /// `rules*.ini` 解析结果。
    pub rules: Option<IniDocument>,
    /// 从 rules 一次解出的建筑受损阈值 / 火焰类型（无 rules 时为缺省）。
    pub damage: StructureDamageRules,
}

impl PaintIniDocs {
    /// 从资源源各读一次 art / rules（缺文件则为 `None`），并固化受损规则。
    pub fn load(source: &dyn AssetSource, art_ini: &str, rules_ini: &str) -> Self {
        let rules = read_optional_ini(source, rules_ini);
        let damage = rules.as_ref().map(StructureDamageRules::from_rules_doc).unwrap_or_default();
        Self {
            art: read_optional_ini(source, art_ini),
            rules,
            damage,
        }
    }
}

/// 读取并解析单个 INI；失败返回 `None`。
pub fn read_optional_ini(source: &dyn AssetSource, name: &str) -> Option<IniDocument> {
    source.read(name).ok().and_then(|b| IniDocument::parse(&b).ok())
}
