//! 自 `adapters/ra-adaptor/src/definitions.rs` 迁出的单元测试（集成测试 crate）。

// 自 adapters/ra-adaptor/src/definitions.rs :: tests
use ra_adaptor::{RulesSystem, definitions::*};
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_types::{BuiltinCapability, GameEdition};

fn rules_from(text: &[u8]) -> RulesSystem {
    rules_from_with_art(text, b"")
}

fn rules_from_with_art(rules_text: &[u8], art_text: &[u8]) -> RulesSystem {
    let rules = IniDocument::parse(rules_text).expect("test rules ini");
    let art = if art_text.is_empty() { IniDocument::default() } else { IniDocument::parse(art_text).expect("test art ini") };
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art,
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
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
[LightningStorm]\nUIName=Name:LightningStorm\nType=LightningStorm\nAction=LightningStorm\nRechargeTime=10\nSidebarImage=SSWLSICON\n\
[BuildingTypes]\n0=GACNST\n1=GATECH\n\
[GACNST]\nConstructionYard=yes\nCost=2500\nStrength=1000\n\
[GATECH]\nCost=1500\nStrength=600\nSuperWeapon=LightningStorm\n",
    );
    let defs = build_runtime_definitions(&rules);
    let sw = defs.super_weapons.get("LightningStorm").expect("SW");
    assert_eq!(sw.ui_name, "Name:LightningStorm");
    assert_eq!(sw.kind, "LIGHTNINGSTORM");
    assert_eq!(sw.action, "LIGHTNINGSTORM");
    assert_eq!(sw.recharge_time, 10);
    assert_eq!(sw.sidebar_image, "SSWLSICON");
    assert_eq!(defs.structures.get("GATECH").and_then(|s| s.super_weapon.as_deref()), Some("LIGHTNINGSTORM"));
    assert!(defs.capabilities.builtins.contains(&BuiltinCapability::SuperWeapon));
    assert!(defs.structures.get("GATECH").expect("tech").capabilities.contains(&BuiltinCapability::SuperWeapon));
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
