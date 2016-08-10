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
fn yuri_stock_disables_score_shade_allied_keeps_it() {
    let yuri = ra_adaptor_yuri::stock_ui::stock_side_chromes().iter().find(|s| s.id == "ThirdSide").unwrap();
    assert!(!yuri.score_stats_shade);
    assert_eq!(yuri.score_background, Some("mpyscrnl.shp"));
    let gdi = ra_adaptor_yuri::stock_ui::stock_side_chromes().iter().find(|s| s.id == "GDI").unwrap();
    assert!(gdi.score_stats_shade);
}

#[test]
fn load_screen_stock_pal_pairs_base_shp_with_shared_mpls() {
    let americans = ra_adaptor_ra2::stock_ui::stock_country_ui().iter().find(|c| c.id == "Americans").unwrap();
    assert_eq!(americans.load_screen_pal, "mpls.pal");
    assert_ne!(americans.load_screen_pal, "mplsu.pal");

    let yuri = ra_adaptor_yuri::stock_ui::stock_country_ui_all().find(|c| c.id == "YuriCountry").unwrap();
    assert_eq!(yuri.load_screen_pal, "mpyls.pal");
    assert_eq!(yuri.load_screen, "ls800yuri.shp");
}
