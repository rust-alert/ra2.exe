//! 自 `engine/ra-types/src/definition/structure.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-types/src/definition/structure.rs :: tests
use ra_types::definition::structure::BuildCat;

#[test]
fn build_cat_combat_is_defense_tab() {
    assert_eq!(BuildCat::parse("Combat"), BuildCat::Combat);
    assert_eq!(BuildCat::parse("combat"), BuildCat::Combat);
    assert!(BuildCat::Combat.is_defense_tab());
}

#[test]
fn build_cat_missing_or_other_goes_to_building_tab() {
    assert_eq!(BuildCat::parse(""), BuildCat::Building);
    assert_eq!(BuildCat::parse("Tech"), BuildCat::Building);
    assert_eq!(BuildCat::parse("Power"), BuildCat::Building);
    assert_eq!(BuildCat::parse("Resource"), BuildCat::Building);
    assert!(!BuildCat::Building.is_defense_tab());
}
