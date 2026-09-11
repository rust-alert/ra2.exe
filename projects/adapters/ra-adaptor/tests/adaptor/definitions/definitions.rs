//! 自 `adapters/ra-adaptor/src/definitions.rs` 迁出的单元测试（集成测试 crate）。

// 自 adapters/ra-adaptor/src/definitions.rs :: tests
use ra_adaptor::{RulesSystem, definitions::*};
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_types::{BuiltinCapability, GameEdition, TerrainSpawnerDefinitions};

fn rules_from(text: &[u8]) -> RulesSystem {
    rules_from_with_art(text, b"")
}

fn rules_from_with_art(rules_text: &[u8], art_text: &[u8]) -> RulesSystem {
    let rules = IniDocument::parse(rules_text).expect("test rules ini");
    let art = if art_text.is_empty() { IniDocument::default() } else { IniDocument::parse(art_text).expect("test art ini") };
    let mut techno_types = TechnoTypeRegistry::from_rules(&rules);
    techno_types.apply_art_geometry(&art);
    let warheads = WarheadRegistry::from_names(
        &rules,
        techno_types
            .iter()
            .flat_map(|t| [t.warhead.as_str(), t.secondary_warhead.as_str()]),
    );
    RulesSystem {
        edition: GameEdition::Ra2,
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types,
        warheads,
        super_weapons: SuperWeaponTypeRegistry::from_rules(&rules),
    }
}

#[test]
fn repair_rate_minutes_to_ticks_matches_stock_general() {
    assert_eq!(repair_rate_minutes_to_ticks(0.016), 14);
    assert_eq!(repair_rate_minutes_to_ticks(0.0), 14);
    assert_eq!(repair_rate_minutes_to_ticks(-1.0), 14);
}

#[test]
fn build_runtime_definitions_reads_general_repair_keys() {
    let rules = rules_from(
        b"[General]\nRepairPercent=25\nRepairStep=16\nRepairRate=.032\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\n",
    );
    let defs = build_runtime_definitions(&rules);
    assert_eq!(defs.repair_percent, 25);
    assert_eq!(defs.repair_step, 16);
    assert_eq!(defs.repair_interval_ticks, 28);
}

#[test]
fn build_runtime_definitions_falls_back_to_stock_repair_defaults() {
    let rules = rules_from(b"[BuildingTypes]\n0=GAPOWR\n[GAPOWR]\nCost=1\nStrength=1\n");
    let defs = build_runtime_definitions(&rules);
    assert_eq!(defs.repair_percent, 15);
    assert_eq!(defs.repair_step, 8);
    assert_eq!(defs.repair_interval_ticks, 14);
}

#[test]
fn build_runtime_definitions_parses_super_weapon_types_and_building_link() {
    let rules = rules_from(
        b"[SuperWeaponTypes]\n0=LightningStorm\n\
[LightningStorm]\nUIName=Name:LightningStorm\nType=LightningStorm\nAction=LightningStorm\nRechargeTime=10\nSidebarImage=SSWLSICON\nWeapon=LightningBolt\n\
[LightningBolt]\nDamage=250\nROF=1\nRange=8\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
[BuildingTypes]\n0=GACNST\n1=GATECH\n\
[GACNST]\nConstructionYard=yes\nCost=2500\nStrength=1000\n\
[GATECH]\nCost=1500\nStrength=600\nSuperWeapon=LightningStorm\n",
    );
    let defs = build_runtime_definitions(&rules);
    let sw = defs.super_weapons.get("LightningStorm").expect("SW");
    assert_eq!(sw.ui_name, "NAME:LIGHTNINGSTORM");
    assert_eq!(sw.kind, "LIGHTNINGSTORM");
    assert_eq!(sw.action, "LIGHTNINGSTORM");
    assert_eq!(sw.recharge_time, 10);
    assert_eq!(sw.sidebar_image, "SSWLSICON");
    assert_eq!(sw.weapon, "LIGHTNINGBOLT");
    assert_ne!(sw.weapon_id, ra_types::WeaponId(0));
    let weapon = defs.weapons.get_by_id(sw.weapon_id).expect("SW weapon");
    assert_eq!(weapon.type_key, "LIGHTNINGBOLT");
    assert_eq!(weapon.damage, 250);
    assert_eq!(weapon.range, 8);
    assert_eq!(weapon.rof, 1);
    assert_ne!(weapon.warhead_id, ra_types::WarheadId(0));
    assert_eq!(defs.structures.get("GATECH").and_then(|s| s.super_weapon.as_deref()), Some("LIGHTNINGSTORM"));
    let gatech = defs.structures.get("GATECH").expect("tech");
    let sw_id = gatech.super_weapon_id.expect("bound SW id");
    assert_eq!(defs.super_weapons.get_by_id(sw_id).map(|d| d.type_key.as_str()), Some("LIGHTNINGSTORM"));
    assert!(defs.capabilities.builtins.contains(&BuiltinCapability::SuperWeapon));
    assert!(gatech.capabilities.contains(&BuiltinCapability::SuperWeapon));
}

#[test]
fn build_runtime_definitions_reads_foundation_from_art() {
    let rules = rules_from_with_art(
        b"[BuildingTypes]\n0=NAWEAP\n\
[NAWEAP]\nCost=2000\nStrength=1000\nOwner=Russians\n",
        b"[NAWEAP]\nFoundation=5x3\nHeight=6\n",
    );
    let defs = build_runtime_definitions(&rules);
    let s = defs.structures.get("NAWEAP").expect("NAWEAP");
    assert_eq!((s.foundation.width, s.foundation.height), (5, 3));
    assert_eq!(s.height, 6);
}

#[test]
fn build_runtime_definitions_foundation_follows_art_image() {
    let rules = rules_from_with_art(
        b"[BuildingTypes]\n0=NAWEAP2\n\
[NAWEAP2]\nCost=2000\nStrength=1000\nOwner=Russians\n",
        b"[NAWEAP2]\nImage=NAWEAP\n\
[NAWEAP]\nFoundation=5x3\nHeight=6\n",
    );
    let defs = build_runtime_definitions(&rules);
    let s = defs.structures.get("NAWEAP2").expect("NAWEAP2");
    assert_eq!((s.foundation.width, s.foundation.height), (5, 3));
    assert_eq!(s.height, 6);
}

#[test]
fn build_runtime_definitions_rules_foundation_fallback_without_art() {
    let rules = rules_from(
        b"[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nFoundation=2x2\nHeight=4\n",
    );
    let defs = build_runtime_definitions(&rules);
    let s = defs.structures.get("GAPOWR").expect("GAPOWR");
    assert_eq!((s.foundation.width, s.foundation.height), (2, 2));
    assert_eq!(s.height, 4);
}

#[test]
fn build_runtime_definitions_binds_primary_weapon_and_warhead_ids() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let defs = build_runtime_definitions(&rules);
    let mtnk = defs.techno.get("MTNK").expect("MTNK");
    assert_eq!(mtnk.primary, "90MM");
    assert_ne!(mtnk.primary_id, ra_types::WeaponId(0));
    assert_eq!(mtnk.warhead, "SA");
    assert_ne!(mtnk.warhead_id, ra_types::WarheadId(0));
    let weapon = defs.weapons.get_by_id(mtnk.primary_id).expect("bound weapon");
    assert_eq!(weapon.type_key, "90MM");
    assert_eq!(weapon.damage, 50);
    assert_eq!(weapon.range, 6);
    assert_eq!(weapon.rof, 8);
    assert_eq!(weapon.warhead_id, mtnk.warhead_id);
    assert!(weapon.projectile.is_empty());
    let wh = defs.warheads.get_by_id(mtnk.warhead_id).expect("bound warhead");
    assert_eq!(wh.type_key, "SA");
    assert_eq!(*wh.verses, [100; 11]);
    assert_eq!(wh.spread, 0);
    assert_eq!(wh.prone_damage, 100);
}

#[test]
fn build_runtime_definitions_binds_secondary_weapon_id() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=FV\n\
[FV]\nPrimary=HoverMissile\nSecondary=Repair\n\
[HoverMissile]\nDamage=50\nROF=40\nRange=6\nWarhead=SA\n\
[Repair]\nDamage=0\nROF=20\nRange=3\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let defs = build_runtime_definitions(&rules);
    let fv = defs.techno.get("FV").expect("FV");
    assert_eq!(fv.secondary, "REPAIR");
    assert_ne!(fv.secondary_id, ra_types::WeaponId(0));
    assert_ne!(fv.secondary_id, fv.primary_id);
    let secondary = defs.weapons.get_by_id(fv.secondary_id).expect("secondary");
    assert_eq!(secondary.type_key, "REPAIR");
    assert_eq!(secondary.range, 3);
    assert_eq!(secondary.rof, 20);
}

#[test]
fn build_runtime_definitions_projects_techno_fields_without_rescanning_section() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=FV\n\
[BuildingTypes]\n0=GAPOWR\n\
[FV]\nCost=600\nStrength=200\nPrerequisite=GAWEAP,POWER\nBuildLimit=2\nDeploysInto=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nPower=150\nConstructionYard=yes\nFactory=BuildingType\nCapturable=yes\n",
    );
    let defs = build_runtime_definitions(&rules);
    let fv = defs.techno.get("FV").expect("FV");
    assert_eq!(
        fv.prerequisite,
        vec![
            ra_types::PrerequisiteToken::UnboundType("GAWEAP".into()),
            ra_types::PrerequisiteToken::Group(ra_types::PrerequisiteGroupKind::Power),
        ]
    );
    assert_eq!(fv.build_limit, 2);
    let deploy = defs.deployables.iter().find(|d| d.source_key == "FV").expect("deploy");
    assert_eq!(deploy.target_key, "GAPOWR");
    let power = defs.structures.get("GAPOWR").expect("GAPOWR");
    assert_eq!(power.power.output, 150);
    assert!(power.construction_yard);
    assert!(power.capturable);
    assert!(power.production.is_some());
}
