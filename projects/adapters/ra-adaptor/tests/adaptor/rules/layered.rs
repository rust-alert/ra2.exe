//! 层叠 rules 装载入口。

use ra_adaptor::{build_runtime_definitions, rules_system_from_layered_ini_bytes};
use ra_types::GameEdition;

#[test]
fn layered_rules_merge_techno_cost_and_append_owner() {
    let base = b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nOwner=Americans\nCost=700\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let top = b"[MTNK]\nOwner=Alliance\nCost=800\n";
    let rules = rules_system_from_layered_ini_bytes(GameEdition::Ra2, &[base.as_slice(), top.as_slice()], &[])
        .expect("layered rules");
    let m = rules.techno_types.get("MTNK").expect("MTNK");
    assert_eq!(m.cost, 800);
    assert!(m.owner.owner_allows("Americans"));
    assert!(m.owner.owner_allows("Alliance"));

    let defs = build_runtime_definitions(&rules);
    let techno = defs.techno.get("MTNK").expect("def");
    assert_eq!(techno.cost, 800);
    assert!(techno.owner.owner_allows("Americans"));
    assert!(techno.owner.owner_allows("Alliance"));
    assert_ne!(techno.primary_id, ra_types::WeaponId(0));
}
