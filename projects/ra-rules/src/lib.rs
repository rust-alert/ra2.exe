//! 按资源链对应的 INI 名加载规则。

mod color_schemes;
mod overlay_types;
mod techno_types;

use ra_adaptor::ResourceChain;
use ra_assets::IniDocument;
use ra_types::{AssetSource, GameEdition, RaResult};

pub use color_schemes::ColorSchemes;
pub use overlay_types::OverlayTypeRegistry;
pub use techno_types::{TechnoKind, TechnoType, TechnoTypeRegistry};

#[derive(Debug, Clone)]
pub struct RulesDb {
    pub edition: GameEdition,
    pub rules: IniDocument,
    pub art: IniDocument,
    pub overlay_types: OverlayTypeRegistry,
    pub color_schemes: ColorSchemes,
    pub techno_types: TechnoTypeRegistry,
}

/// 用显式 `ResourceChain` 加载（适配组合装配后的入口）。
pub fn load_rules_chain(source: &dyn AssetSource, chain: &ResourceChain) -> RaResult<RulesDb> {
    let rules = IniDocument::parse(&source.read(chain.rules_ini)?)?;
    let art = IniDocument::parse(&source.read(chain.art_ini)?)?;
    let overlay_types = OverlayTypeRegistry::from_rules(&rules);
    let color_schemes = ColorSchemes::from_rules(&rules);
    let techno_types = TechnoTypeRegistry::from_rules(&rules);
    Ok(RulesDb {
        edition: chain.edition,
        rules,
        art,
        overlay_types,
        color_schemes,
        techno_types,
    })
}

/// 按互斥 `GameEdition` 取默认资源表再加载（兼容旧调用）。
pub fn load_rules(source: &dyn AssetSource, edition: GameEdition) -> RaResult<RulesDb> {
    load_rules_chain(source, &ResourceChain::for_edition(edition))
}
