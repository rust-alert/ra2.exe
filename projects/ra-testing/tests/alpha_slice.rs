//! Alpha 冻结竖切夹具。

use ra_testing::{ALPHA_SKIRMISH_SLICE_ID, SliceBuildingRole, SliceUnitRole, alpha_skirmish_v1};

#[test]
fn alpha_skirmish_v1_lists_minimum_base_chain() {
    let slice = alpha_skirmish_v1();
    assert_eq!(slice.slice_id, ALPHA_SKIRMISH_SLICE_ID);
    assert_eq!(slice.edition, "ra2");
    assert_eq!(slice.map_id, "mp03t4");
    assert_eq!(slice.starting_funds, 10_000);
    assert_eq!(slice.ore_income_per_trip, 700);
    assert_eq!(slice.allied_mcv, "AMCV");
    assert_eq!(slice.soviet_mcv, "SMCV");

    let roles: Vec<_> = slice.buildings.iter().map(|b| b.role).collect();
    assert!(roles.contains(&SliceBuildingRole::ConstructionYard));
    assert!(roles.contains(&SliceBuildingRole::Power));
    assert!(roles.contains(&SliceBuildingRole::Barracks));
    assert!(roles.contains(&SliceBuildingRole::WarFactory));
    assert!(roles.contains(&SliceBuildingRole::Refinery));

    assert!(slice.units.iter().any(|u| u.role == SliceUnitRole::Infantry && u.side == "allied"));
    assert!(slice.units.iter().any(|u| u.role == SliceUnitRole::Vehicle && u.side == "soviet"));
    assert!(slice.units.iter().any(|u| u.type_id == "AMCV"));
    assert!(slice.units.iter().any(|u| u.type_id == "SMCV"));
}
