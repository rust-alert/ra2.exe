//! 集成测试：原 `src/techno_types.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn parse_vehicle_list() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n1=HTNK\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nSight=6\nCost=800\nTechLevel=2\nOwner=Americans\nImage=MTNK\nROF=12\n\
[HTNK]\nStrength=600\nArmor=heavy\nSpeed=4\nSight=6\nCost=1400\nTechLevel=6\nOwner=Americans\n",
    )
    .unwrap();
    let reg = TechnoTypeRegistry::from_rules(&doc);
    assert_eq!(reg.len(), 2);
    assert_eq!(reg.count_kind(TechnoKind::Vehicle), 2);
    let m = reg.get("mtnk").unwrap();
    assert_eq!(m.strength, 300);
    assert_eq!(m.speed, 6);
    assert_eq!(m.cost, 800);
    assert_eq!(m.image, "MTNK");
    assert_eq!(m.rof, 12);
    assert_eq!(reg.get("htnk").unwrap().rof, 0);
}
