//! 按资源链装载 rules/art 与派生注册表。

use ra_assets::{
    ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry,
    overlay_types_from_rules,
};
use ra_types::{AssetSource, GameEdition, OverlayTypeRegistry, RaResult};

use crate::ResourceChain;

/// 一局装载用的规则快照。
#[derive(Debug, Clone)]
pub struct RulesSystem {
    /// 规则来源对应的 `GameEdition`。
    pub edition: GameEdition,
    /// 解析后的 `rules` INI 文档。
    pub rules: IniDocument,
    /// 解析后的 `art` INI 文档。
    pub art: IniDocument,
    /// `[General]` / 对话 / 语音等装载期全局字段。
    pub globals: RulesGlobals,
    /// 从 rules 派生的 overlay 类型注册表。
    pub overlay_types: OverlayTypeRegistry,
    /// 从 rules 派生的配色方案表。
    pub color_schemes: ColorSchemes,
    /// 从 rules 派生的国家 / 势力表。
    pub countries: CountryRegistry,
    /// 从 rules 派生的 techno 类型注册表。
    pub techno_types: TechnoTypeRegistry,
    /// 从 techno 主武器引用的弹头表。
    pub warheads: WarheadRegistry,
    /// 从 `[SuperWeaponTypes]` 派生的超武类型表。
    pub super_weapons: SuperWeaponTypeRegistry,
}

/// 用显式 `ResourceChain` 加载（适配组合装配后的入口）。
pub fn load_rules_chain(source: &dyn AssetSource, chain: &ResourceChain) -> RaResult<RulesSystem> {
    let rules_bytes = source.read(chain.rules_ini)?;
    let rules = IniDocument::parse(&rules_bytes)
        .map_err(|e| ra_types::RaError::Parse(format!("{} ({} bytes): {e}", chain.rules_ini, rules_bytes.len())))?;
    let art_bytes = source.read(chain.art_ini)?;
    let art =
        IniDocument::parse(&art_bytes).map_err(|e| ra_types::RaError::Parse(format!("{} ({} bytes): {e}", chain.art_ini, art_bytes.len())))?;
    let globals = RulesGlobals::from_rules(&rules);
    let overlay_types = overlay_types_from_rules(&rules);
    let color_schemes = ColorSchemes::from_rules(&rules);
    let countries = CountryRegistry::from_rules(&rules);
    let techno_types = TechnoTypeRegistry::from_rules(&rules);
    let warheads = WarheadRegistry::from_names(&rules, techno_types.iter().map(|t| t.warhead.as_str()));
    let super_weapons = SuperWeaponTypeRegistry::from_rules(&rules);
    Ok(RulesSystem {
        edition: chain.edition,
        rules,
        art,
        globals,
        overlay_types,
        color_schemes,
        countries,
        techno_types,
        warheads,
        super_weapons,
    })
}

/// 按互斥 `GameEdition` 取默认资源表再加载（兼容旧调用）。
pub fn load_rules(source: &dyn AssetSource, edition: GameEdition) -> RaResult<RulesSystem> {
    load_rules_chain(source, &ResourceChain::for_edition(edition))
}
