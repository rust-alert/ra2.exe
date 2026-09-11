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
    assert_eq!(m.armor, ra_types::ArmorKind::Heavy);
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
fn missing_armor_defaults_to_none() {
    let doc = IniDocument::parse(b"[VehicleTypes]\n0=MTNK\n[MTNK]\nStrength=100\n").unwrap();
    let reg = TechnoTypeRegistry::from_rules(&doc);
    assert_eq!(reg.get("MTNK").unwrap().armor, ra_types::ArmorKind::None);
}

#[test]
fn unknown_armor_falls_back_to_none() {
    let doc = IniDocument::parse(b"[VehicleTypes]\n0=MTNK\n[MTNK]\nArmor=not-a-kind\n").unwrap();
    let reg = TechnoTypeRegistry::from_rules(&doc);
    assert_eq!(reg.get("MTNK").unwrap().armor, ra_types::ArmorKind::None);
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

#[test]
fn parse_build_gates_deploy_and_structure_flags() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=FV\n\
[BuildingTypes]\n0=GAPOWR\n1=GACNST\n\
[FV]\nStrength=200\nCost=600\nPrerequisite=GAWEAP,POWER\nPrerequisiteOverride=GACNST\n\
RequiredHouses=Americans,Alliance\nForbiddenHouses=Russians\nBuildLimit=1\nBuildTime=50\n\
RequiresStolenAlliedTech=yes\nDeploysInto=gapowr\nPixelSelectionBracketDelta=-5\n\
[GAPOWR]\nStrength=600\nCost=600\nPower=200\nPowered=no\nBuildCat=Combat\nCapturable=yes\n\
[GACNST]\nStrength=1000\nCost=2500\nConstructionYard=yes\nFactory=BuildingType\n\
Radar=yes\nRefinery=no\nSuperWeapon=Nuke\nPower=-50\n",
    )
    .unwrap();
    let reg = TechnoTypeRegistry::from_rules(&doc);
    let fv = reg.get("FV").unwrap();
    assert_eq!(
        fv.prerequisite.iter().cloned().collect::<Vec<_>>(),
        vec![
            ra_types::PrerequisiteToken::UnboundType("GAWEAP".into()),
            ra_types::PrerequisiteToken::Group(ra_types::PrerequisiteGroupKind::Power),
        ]
    );
    assert_eq!(
        fv.prerequisite_override.iter().cloned().collect::<Vec<_>>(),
        vec![ra_types::PrerequisiteToken::UnboundType("GACNST".into())]
    );
    assert_eq!(
        fv.required_houses.iter().map(|h| h.as_str()).collect::<Vec<_>>(),
        vec!["ALLIANCE", "AMERICANS"]
    );
    assert_eq!(
        fv.forbidden_houses.iter().map(|h| h.as_str()).collect::<Vec<_>>(),
        vec!["RUSSIANS"]
    );
    assert_eq!(fv.build_limit, 1);
    assert_eq!(fv.build_time, 50);
    assert!(fv.requires_stolen_allied_tech);
    assert!(!fv.requires_stolen_soviet_tech);
    assert_eq!(fv.pixel_selection_bracket_delta, -5);
    assert_eq!(fv.deploys_into, "GAPOWR");

    let power = reg.get("GAPOWR").unwrap();
    assert_eq!(power.power, 200);
    assert_eq!(power.powered, Some(false));
    assert_eq!(power.build_cat, ra_types::BuildCat::Combat);
    assert!(power.capturable);

    let yard = reg.get("GACNST").unwrap();
    assert!(yard.construction_yard);
    assert!(!yard.refinery);
    assert!(yard.radar);
    assert_eq!(yard.factory, "BuildingType");
    assert_eq!(yard.super_weapon, "NUKE");
    assert_eq!(yard.power, -50);
}

#[test]
fn apply_art_geometry_overrides_rules_and_follows_image() {
    let rules = IniDocument::parse(
        b"[BuildingTypes]\n0=NAWEAP\n1=NAWEAP2\n\
[NAWEAP]\nStrength=1000\nFoundation=2x2\nHeight=3\n\
[NAWEAP2]\nStrength=1000\n",
    )
    .unwrap();
    let art = IniDocument::parse(
        b"[NAWEAP]\nFoundation=5x3\nHeight=6\n\
[NAWEAP2]\nImage=NAWEAP\n",
    )
    .unwrap();
    let mut reg = TechnoTypeRegistry::from_rules(&rules);
    assert_eq!(reg.get("NAWEAP").unwrap().foundation, "2x2");
    assert_eq!(reg.get("NAWEAP").unwrap().height, Some(3));
    reg.apply_art_geometry(&art);
    let a = reg.get("NAWEAP").unwrap();
    assert_eq!(a.foundation, "5x3");
    assert_eq!(a.height, Some(6));
    let b = reg.get("NAWEAP2").unwrap();
    assert_eq!(b.foundation, "5x3");
    assert_eq!(b.height, Some(6));
}

#[test]
fn from_layered_merges_techno_fields_and_list() {
    let base = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=300\nCost=700\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=10\nRange=5\nWarhead=SA\n",
    )
    .unwrap();
    let top = IniDocument::parse(
        b"[VehicleTypes]\n1=HTNK\n\
[MTNK]\nStrength=400\n\
[HTNK]\nStrength=600\nCost=1400\n",
    )
    .unwrap();
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let docs = [base, top];
    let reg = TechnoTypeRegistry::from_layered(LayeredIniView::new(&docs, &policy));
    assert_eq!(reg.len(), 2);
    let m = reg.get("MTNK").unwrap();
    assert_eq!(m.strength, 400);
    assert_eq!(m.cost, 700);
    assert_eq!(m.damage, 50);
    assert_eq!(reg.get("HTNK").unwrap().strength, 600);
}

#[test]
fn owner_list_decodes_once_and_allows() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n[MTNK]\nOwner=Americans,Alliance\nRequiredHouses=\nForbiddenHouses=Russians\n",
    )
    .unwrap();
    let reg = TechnoTypeRegistry::from_rules(&doc);
    let m = reg.get("MTNK").unwrap();
    assert!(m.owner.owner_allows("americans"));
    assert!(m.owner.owner_allows("Alliance"));
    assert!(!m.owner.owner_allows("Russians"));
    assert!(m.required_houses.is_empty());
    assert!(m.forbidden_houses.forbids("russians"));
}
