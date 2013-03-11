//! 按版本对应的 INI 名加载规则。

mod overlay_types;

use ra_adaptor::ResourceChain;
use ra_assets::IniDocument;
use ra_types::{AssetSource, GameEdition, RaResult};

pub use overlay_types::OverlayTypeRegistry;

#[derive(Debug, Clone)]
pub struct RulesDb {
    pub edition: GameEdition,
    pub rules: IniDocument,
    pub art: IniDocument,
    pub overlay_types: OverlayTypeRegistry,
}

pub fn load_rules(source: &dyn AssetSource, edition: GameEdition) -> RaResult<RulesDb> {
    let chain = ResourceChain::for_edition(edition);
    let rules = IniDocument::parse(&source.read(chain.rules_ini)?)?;
    let art = IniDocument::parse(&source.read(chain.art_ini)?)?;
    let overlay_types = OverlayTypeRegistry::from_rules(&rules);
    Ok(RulesDb {
        edition,
        rules,
        art,
        overlay_types,
    })
}
