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

#[test]
fn score_screen_shade_differs_by_edition() {
    assert!(ra_adaptor::score_screen_style(GameEdition::Ra2).stats_shade);
    assert!(!ra_adaptor::score_screen_style(GameEdition::Yr).stats_shade);
    assert!(!ra_adaptor::score_screen_style(GameEdition::Mo3).stats_shade);
}
