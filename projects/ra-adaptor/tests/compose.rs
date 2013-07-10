//! `compose` 模块集成测试（仅通过 crate 公开 API）。

use ra_adaptor::{AdaptorStack, BaseGame, ExtensionId};
use ra_types::GameEdition;

#[test]
fn mo3_is_yr_plus_phobos_layout() {
    let s = AdaptorStack::from_edition(GameEdition::Mo3);
    assert_eq!(s.base, BaseGame::Yr);
    assert!(s.mo_layout);
    assert_eq!(s.extensions, vec![ExtensionId::Phobos]);
    assert_eq!(s.to_edition(), GameEdition::Mo3);
}

#[test]
fn retail_roundtrip() {
    assert_eq!(AdaptorStack::from_edition(GameEdition::Ra2).to_edition(), GameEdition::Ra2);
    assert_eq!(AdaptorStack::from_edition(GameEdition::Yr).to_edition(), GameEdition::Yr);
}
