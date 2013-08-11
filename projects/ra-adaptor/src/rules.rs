//! 按资源链装载 rules/art 与派生注册表。

use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_types::{AssetSource, GameEdition, RaResult};

use crate::ResourceChain;

/// 一局装载用的规则快照。
#[derive(Debug, Clone)]
pub struct RulesDb {
    /// 规则来源对应的 `GameEdition`。
    pub edition: GameEdition,
    /// 解析后的 `rules` INI 文档。
    pub rules: IniDocument,
    /// 解析后的 `art` INI 文档。
    pub art: IniDocument,
    /// 从 rules 派生的 overlay 类型注册表。
    pub overlay_types: OverlayTypeRegistry,
    /// 从 rules 派生的配色方案表。
    pub color_schemes: ColorSchemes,
    /// 从 rules 派生的 techno 类型注册表。
    pub techno_types: TechnoTypeRegistry,
    /// 从 techno 主武器引用的弹头表。
    pub warheads: WarheadRegistry,
}

/// 用显式 `ResourceChain` 加载（适配组合装配后的入口）。
pub fn load_rules_chain(source: &dyn AssetSource, chain: &ResourceChain) -> RaResult<RulesDb> {
    let rules = IniDocument::parse(&source.read(chain.rules_ini)?)?;
    let art = IniDocument::parse(&source.read(chain.art_ini)?)?;
    let overlay_types = OverlayTypeRegistry::from_rules(&rules);
    let color_schemes = ColorSchemes::from_rules(&rules);
    let techno_types = TechnoTypeRegistry::from_rules(&rules);
    let warheads = WarheadRegistry::from_names(&rules, techno_types.iter().map(|t| t.warhead.as_str()));
    Ok(RulesDb { edition: chain.edition, rules, art, overlay_types, color_schemes, techno_types, warheads })
}
/// 按互斥 `GameEdition` 取默认资源表再加载（兼容旧调用）。
pub fn load_rules(source: &dyn AssetSource, edition: GameEdition) -> RaResult<RulesDb> {
    load_rules_chain(source, &ResourceChain::for_edition(edition))
}
