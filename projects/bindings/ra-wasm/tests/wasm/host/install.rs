//! 自 `bindings/ra-wasm/src/host/install.rs` 迁出的单元测试（集成测试 crate）。

// 自 bindings/ra-wasm/src/host/install.rs :: tests
use ra_types::{GameEdition, RaError};
use ra_wasm::host::install::*;

fn bag_named(names: &[&str]) -> InstallBag {
    let mut bag = InstallBag::new();
    for name in names {
        bag.ingest(name, vec![0u8; 64]);
    }
    bag
}

#[test]
fn detect_ra2_from_classic_markers() {
    let bag = bag_named(&["RA2.MIX", "language.mix", "game.exe"]);
    assert_eq!(detect_edition_from_names(&bag).unwrap(), GameEdition::Ra2);
}

#[test]
fn detect_yr_from_md_markers() {
    let bag = bag_named(&["ra2md.mix", "langmd.mix", "gamemd.exe"]);
    assert_eq!(detect_edition_from_names(&bag).unwrap(), GameEdition::Yr);
}

#[test]
fn ambiguous_when_both_ra2_and_yr_markers() {
    let bag = bag_named(&["ra2.mix", "ra2md.mix"]);
    assert!(matches!(detect_edition_from_names(&bag), Err(RaError::AmbiguousEdition(_))));
}

#[test]
fn prepare_explicit_edition_reports_missing_base() {
    let mut bag = bag_named(&["expand01.mix"]);
    let report = bag.prepare(Some(GameEdition::Ra2)).unwrap();
    assert_eq!(report.edition(), "ra2");
    assert!(report.missing_base().iter().any(|n| n.eq_ignore_ascii_case("ra2.mix")));
    assert!(report.missing_base().iter().any(|n| n.eq_ignore_ascii_case("language.mix")));
    assert!(report.summary().contains("edition=ra2"));
    assert!(!ra_wasm::host::boot::can_boot(report.mounted_root(), report.missing_base().len()));
}

#[test]
fn boot_helpers_match_counts() {
    assert!(ra_wasm::host::boot::has_install_input(1));
    assert!(!ra_wasm::host::boot::has_install_input(0));
    assert!(ra_wasm::host::boot::can_boot(3, 0));
    assert!(!ra_wasm::host::boot::can_boot(0, 0));
    assert!(!ra_wasm::host::boot::can_boot(2, 1));
}
