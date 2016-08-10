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
    assert_eq!(m.damage, 0);
    assert_eq!(m.range, 0);
    assert!(m.primary.is_empty());
    assert_eq!(reg.get("htnk").unwrap().rof, 0);
}

#[test]
fn parse_category_naval_and_tech_level() {
    let doc = IniDocument::parse(
        b"[InfantryTypes]\n0=ADOG\n\
[VehicleTypes]\n0=DEST\n\
[ADOG]\nStrength=100\nSpeed=8\nSight=5\nCost=200\nTechLevel=-1\nCategory=Dog\nOwner=Americans\n\
[DEST]\nStrength=600\nSpeed=6\nSight=7\nCost=1000\nTechLevel=5\nNaval=yes\nOwner=Americans\n",
    )
    .unwrap();
    let reg = TechnoTypeRegistry::from_rules(&doc);
    let dog = reg.get("ADOG").unwrap();
    assert_eq!(dog.category, "Dog");
    assert_eq!(dog.tech_level, -1);
    assert!(!dog.naval);
    let dest = reg.get("DEST").unwrap();
    assert!(dest.naval);
    assert_eq!(dest.tech_level, 5);
}

#[test]
fn parse_primary_weapon_damage_and_range() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=90mm\n\
[90mm]\nDamage=75\nROF=20\nRange=5\nProjectile=Invisible\nWarhead=AP\n",
    )
    .unwrap();
    let reg = TechnoTypeRegistry::from_rules(&doc);
    let m = reg.get("MTNK").unwrap();
    assert_eq!(m.primary, "90MM");
    assert_eq!(m.damage, 75);
    assert_eq!(m.range, 5);
    assert_eq!(m.rof, 20);
    assert_eq!(m.warhead, "AP");
}
