//! 按版本对应的 INI 名加载规则。

use ra_assets::IniDocument;
use ra_types::{AssetSource, GameEdition, RaResult, ResourceChain};

#[derive(Debug, Clone)]
pub struct RulesDb {
    pub edition: GameEdition,
    pub rules: IniDocument,
    pub art: IniDocument,
}

pub fn load_rules(source: &dyn AssetSource, edition: GameEdition) -> RaResult<RulesDb> {
    let chain = ResourceChain::for_edition(edition);
    let rules = IniDocument::parse(&source.read(chain.rules_ini)?)?;
    let art = IniDocument::parse(&source.read(chain.art_ini)?)?;
    Ok(RulesDb {
        edition,
        rules,
        art,
    })
}
