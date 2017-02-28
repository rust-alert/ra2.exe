//! 自 `adapters/ra-adaptor/src/definitions.rs` 迁出的单元测试（集成测试 crate）。

// 自 adapters/ra-adaptor/src/definitions.rs :: tests
use ra_adaptor::{RulesSystem, definitions::*};
use ra_assets::{
    ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, RulesGlobals, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry,
};
use ra_types::{BuiltinCapability, GameEdition, TerrainSpawnerDefinitions};

fn rules_from(text: &[u8]) -> RulesSystem {
    rules_from_with_art(text, b"")
}

fn rules_from_with_art(rules_text: &[u8], art_text: &[u8]) -> RulesSystem {
    let rules = IniDocument::parse(rules_text).expect("test rules ini");
    let art = if art_text.is_empty() { IniDocument::default() } else { IniDocument::parse(art_text).expect("test art ini") };
    let mut techno_types = TechnoTypeRegistry::from_rules(&rules);
    techno_types.apply_art_geometry(&art);
    let super_weapons = SuperWeaponTypeRegistry::from_rules(&rules);
    let warheads = WarheadRegistry::from_names(
        &rules,
        techno_types
            .iter()
            .flat_map(|t| [t.warhead.as_str(), t.secondary_warhead.as_str()])
            .chain(super_weapons.iter().map(|sw| sw.weapon_warhead.as_str())),
    );
    RulesSystem {
        edition: GameEdition::Ra2,
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::from_rules(&rules),
        techno_types,
        warheads,
        super_weapons,
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
    let defs = build_runtime_definitions(&rules).expect("freeze");
    assert_eq!(defs.repair_percent, 25);
    assert_eq!(defs.repair_step, 16);
    assert_eq!(defs.repair_interval_ticks, 28);
}

#[test]
fn build_runtime_definitions_falls_back_to_stock_repair_defaults() {
    let rules = rules_from(b"[BuildingTypes]\n0=GAPOWR\n[GAPOWR]\nCost=1\nStrength=1\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
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
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let sw = defs.super_weapons.get("LightningStorm").expect("SW");
    assert_eq!(sw.ui_name, "NAME:LIGHTNINGSTORM");
    assert_eq!(sw.kind, "LIGHTNINGSTORM");
    assert_eq!(sw.action, "LIGHTNINGSTORM");
    assert_eq!(sw.recharge_time, 10);
    assert_eq!(sw.sidebar_image, "SSWLSICON");
    assert_eq!(sw.weapon, "LIGHTNINGBOLT");
    assert!(sw.weapon_id.is_some());
    let weapon = defs.weapons.get_by_id(sw.weapon_id.expect("SW weapon id")).expect("SW weapon");
    assert_eq!(weapon.type_key, "LIGHTNINGBOLT");
    assert_eq!(weapon.damage, 250);
    assert_eq!(weapon.range, 8);
    assert_eq!(weapon.rof, 1);
    assert!(weapon.warhead_id.is_some());
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
    let defs = build_runtime_definitions(&rules).expect("freeze");
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
    let defs = build_runtime_definitions(&rules).expect("freeze");
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
    let defs = build_runtime_definitions(&rules).expect("freeze");
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
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let mtnk = defs.techno.get("MTNK").expect("MTNK");
    assert_eq!(mtnk.primary, "90MM");
    assert!(mtnk.primary_id.is_some());
    assert_eq!(mtnk.warhead, "SA");
    assert!(mtnk.warhead_id.is_some());
    let weapon = defs.weapons.get_by_id(mtnk.primary_id.expect("primary id")).expect("bound weapon");
    assert_eq!(weapon.type_key, "90MM");
    assert_eq!(weapon.damage, 50);
    assert_eq!(weapon.range, 6);
    assert_eq!(weapon.rof, 8);
    assert_eq!(weapon.warhead_id, mtnk.warhead_id);
    assert!(weapon.projectile.is_empty());
    assert!(weapon.projectile_id.is_none());
    let wh = defs.warheads.get_by_id(mtnk.warhead_id.expect("warhead id")).expect("bound warhead");
    assert_eq!(wh.type_key, "SA");
    assert_eq!(*wh.verses, [100; 11]);
    assert_eq!(wh.spread, 0);
    assert_eq!(wh.prone_damage, 100);
}

#[test]
fn build_runtime_definitions_leaves_empty_weapon_refs_unbound() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nCost=800\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let mtnk = defs.techno.get("MTNK").expect("MTNK");
    assert!(mtnk.primary.is_empty());
    assert!(mtnk.primary_id.is_none());
    assert!(mtnk.secondary_id.is_none());
    assert!(mtnk.warhead_id.is_none());
}

#[test]
fn build_runtime_definitions_binds_projectile_id() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nCost=800\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\nProjectile=Invisible\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let weapon = defs.weapons.get("90MM").expect("weapon");
    assert_eq!(weapon.projectile, "INVISIBLE");
    assert!(weapon.projectile_id.is_some());
    let projectile = defs.projectiles.get_by_id(weapon.projectile_id.expect("projectile id")).expect("projectile");
    assert_eq!(projectile.type_key, "INVISIBLE");
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
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let fv = defs.techno.get("FV").expect("FV");
    assert_eq!(fv.secondary, "REPAIR");
    assert!(fv.secondary_id.is_some());
    assert_ne!(fv.secondary_id, fv.primary_id);
    let secondary = defs.weapons.get_by_id(fv.secondary_id.expect("secondary id")).expect("secondary");
    assert_eq!(secondary.type_key, "REPAIR");
    assert_eq!(secondary.range, 3);
    assert_eq!(secondary.rof, 20);
}

#[test]
fn build_runtime_definitions_projects_techno_fields_without_rescanning_section() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=FV\n\
[BuildingTypes]\n0=GAPOWR\n1=GAWEAP\n\
[FV]\nCost=600\nStrength=200\nPrerequisite=GAWEAP,POWER\nBuildLimit=2\nDeploysInto=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nPower=150\nConstructionYard=yes\nFactory=BuildingType\nCapturable=yes\n\
[GAWEAP]\nCost=2000\nStrength=1000\nPower=-30\nFactory=UnitType\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let fv = defs.techno.get("FV").expect("FV");
    let gaweap_id = defs.techno.get("GAWEAP").expect("GAWEAP").id;
    assert_eq!(
        fv.prerequisite,
        vec![ra_types::PrerequisiteToken::Type(gaweap_id), ra_types::PrerequisiteToken::Group(ra_types::PrerequisiteGroupKind::Power),]
    );
    assert_eq!(fv.build_limit, 2);
    let deploy = defs.deployables.iter().find(|d| d.source_key == "FV").expect("deploy");
    assert_eq!(deploy.target_key, "GAPOWR");
    assert_ne!(deploy.target, ra_types::TypeId(0));
    let power = defs.structures.get("GAPOWR").expect("GAPOWR");
    assert_eq!(power.power.output, 150);
    assert!(power.construction_yard);
    assert!(power.capturable);
    assert!(power.production.is_some());
}

#[test]
fn build_runtime_definitions_freezes_countries_into_house_table() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n1=Russians\n\
[Americans]\nUIName=Name:Americans\nSide=GDI\nMultiplay=yes\n\
[Russians]\nUIName=Name:Russians\nSide=Nod\nMultiplay=yes\nMultiplayObsolete=yes\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    assert_eq!(defs.houses.len(), 5, "two countries plus NEUTRAL/SPECIAL/CIVILIAN");
    let usa = defs.houses.get("Americans").expect("Americans");
    assert_ne!(usa.id, ra_types::HouseId(0));
    assert_eq!(usa.side, "GDI");
    assert_eq!(usa.stolen_tech, Some(ra_types::StolenTechKind::Allied));
    assert!(usa.multiplay);
    let rus = defs.houses.get("Russians").expect("Russians");
    assert_eq!(rus.stolen_tech, Some(ra_types::StolenTechKind::Soviet));
    assert!(!rus.multiplay);
    assert_eq!(defs.stolen_tech_by_house.get(usa.id), Some(ra_types::StolenTechKind::Allied));
    assert!(defs.houses.get("NEUTRAL").is_some());
    assert!(defs.houses.get("SPECIAL").is_some());
    assert!(defs.houses.get("CIVILIAN").is_some());
}

#[test]
fn build_runtime_definitions_freezes_structure_light_profiles() {
    let rules = rules_from(
        b"[BuildingTypes]\n0=GALITE\n1=GAPOWR\n\
[GALITE]\nCost=200\nStrength=400\nLightIntensity=0.5\nLightVisibility=2500\nLightRedTint=1\nLightGreenTint=0.5\nLightBlueTint=0.25\n\
[GAPOWR]\nCost=600\nStrength=600\nPower=150\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let lite = defs.structures.get("GALITE").expect("GALITE");
    let light = lite.light.expect("light profile");
    assert_eq!(light.intensity, 500);
    assert_eq!(light.radius_leptons, 2500);
    assert_eq!(light.tint, [1000, 500, 250]);
    assert!(defs.structures.get("GAPOWR").expect("GAPOWR").light.is_none());
}

#[test]
fn build_runtime_definitions_rejects_unknown_warhead_reference() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nCost=800\nPrimary=90mm\nWarhead=MISSINGWH\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=MISSINGWH\n",
    );
    let err = build_runtime_definitions(&rules).expect_err("missing warhead must fail freeze");
    let msg = err.to_string();
    assert!(msg.contains("warhead"), "{msg}");
    assert!(msg.contains("MISSINGWH"), "{msg}");
}

#[test]
fn build_runtime_definitions_rejects_unknown_deploys_into_target() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=MCV\n\
[MCV]\nCost=3000\nStrength=1000\nDeploysInto=MISSINGYARD\n",
    );
    let err = build_runtime_definitions(&rules).expect_err("missing DeploysInto must fail freeze");
    let msg = err.to_string();
    assert!(msg.contains("techno"), "{msg}");
    assert!(msg.contains("MISSINGYARD"), "{msg}");
}

#[test]
fn build_runtime_definitions_rejects_unknown_structure_super_weapon() {
    let rules = rules_from(
        b"[BuildingTypes]\n0=GATECH\n\
[GATECH]\nCost=1500\nStrength=600\nSuperWeapon=MissingStorm\n",
    );
    let err = build_runtime_definitions(&rules).expect_err("missing SuperWeapon must fail freeze");
    let msg = err.to_string();
    assert!(msg.contains("super_weapon"), "{msg}");
    assert!(msg.contains("MISSINGSTORM") || msg.contains("MissingStorm"), "{msg}");
}

#[test]
fn build_runtime_definitions_rejects_unknown_owner_house_when_countries_present() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nCost=800\nOwner=MissingHouse\n",
    );
    let err = build_runtime_definitions(&rules).expect_err("unknown Owner house must fail freeze");
    let msg = err.to_string();
    assert!(msg.contains("house"), "{msg}");
    assert!(msg.contains("MISSINGHOUSE") || msg.contains("MissingHouse"), "{msg}");
}

#[test]
fn build_runtime_definitions_allows_ambient_owner_house_with_countries() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Neutral\n",
    );
    let defs = build_runtime_definitions(&rules).expect("Neutral Owner should pass");
    assert!(defs.structures.get("GAPOWR").expect("GAPOWR").owner.owner_allows("Neutral"));
    let neutral = defs.houses.get("NEUTRAL").expect("ambient NEUTRAL must receive HouseId");
    assert_ne!(neutral.id, ra_types::HouseId(0));
    assert!(defs.structures.get("GAPOWR").expect("GAPOWR").owner_ids.allows(neutral.id));
}

#[test]
fn build_runtime_definitions_binds_owner_required_forbidden_house_ids() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n1=Alliance\n2=Russians\n\
[Americans]\nSide=GDI\n\
[Alliance]\nSide=GDI\n\
[Russians]\nSide=Nod\n\
[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nCost=800\nOwner=Americans,Alliance\nRequiredHouses=Americans\nForbiddenHouses=Russians\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let mtnk = defs.techno.get("MTNK").expect("MTNK");
    let americans = defs.houses.get("AMERICANS").expect("Americans").id;
    let alliance = defs.houses.get("ALLIANCE").expect("Alliance").id;
    let russians = defs.houses.get("RUSSIANS").expect("Russians").id;
    assert!(mtnk.owner_ids.allows(americans));
    assert!(mtnk.owner_ids.allows(alliance));
    assert!(!mtnk.owner_ids.allows(russians));
    assert!(mtnk.required_house_ids.allows(americans));
    assert!(!mtnk.required_house_ids.allows(alliance));
    assert!(mtnk.forbidden_house_ids.forbids(russians));
    assert!(!mtnk.forbidden_house_ids.forbids(americans));
}

#[test]
fn build_runtime_definitions_rejects_unknown_prerequisite_techno() {
    let rules = rules_from(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nCost=800\nPrerequisite=MISSINGYARD\n",
    );
    let err = build_runtime_definitions(&rules).expect_err("unknown Prerequisite must fail freeze");
    let msg = err.to_string();
    assert!(msg.contains("techno"), "{msg}");
    assert!(msg.contains("MISSINGYARD"), "{msg}");
    assert!(msg.contains("Prerequisite"), "{msg}");
}

#[test]
fn build_runtime_definitions_rejects_unknown_prerequisite_group_member() {
    let rules = rules_from(
        b"[General]\nPrerequisitePower=MISSINGPWR\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\n",
    );
    let err = build_runtime_definitions(&rules).expect_err("unknown group member must fail freeze");
    let msg = err.to_string();
    assert!(msg.contains("techno"), "{msg}");
    assert!(msg.contains("MISSINGPWR"), "{msg}");
    assert!(msg.contains("PrerequisitePower") || msg.contains("PrerequisiteGroups"), "{msg}");
}

#[test]
fn build_runtime_definitions_rejects_unknown_base_unit() {
    let rules = rules_from(
        b"[General]\nBaseUnit=MISSINGMCV\n\
[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\n",
    );
    let err = build_runtime_definitions(&rules).expect_err("unknown BaseUnit must fail freeze");
    let msg = err.to_string();
    assert!(msg.contains("techno"), "{msg}");
    assert!(msg.contains("MISSINGMCV"), "{msg}");
    assert!(msg.contains("BaseUnit"), "{msg}");
}

#[test]
fn bind_map_placements_resolves_techno_and_house_ids() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 3,
        y: 4,
        facing: 32,
        sub_cell: 0,
        mission: ra_types::MissionName::default(),
        tag: ra_types::TagName::default(),
    }];
    let placements = ra_adaptor::bind_map_placements(&entities, &defs, &[]).expect("bind");
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].definition_id, defs.techno.get("GAPOWR").expect("GAPOWR").id);
    assert_eq!(placements[0].owner, defs.houses.get("Americans").expect("Americans").id);
    assert_eq!(placements[0].x, 3);
    assert_eq!(placements[0].y, 4);
}

#[test]
fn bind_map_placements_rejects_unknown_techno_type() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "MISSINGBLDG".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: ra_types::MissionName::default(),
        tag: ra_types::TagName::default(),
    }];
    let err = ra_adaptor::bind_map_placements(&entities, &defs, &[]).expect_err("unknown techno");
    let msg = err.to_string();
    assert!(msg.contains("techno"), "{msg}");
    assert!(msg.contains("MISSINGBLDG"), "{msg}");
}

#[test]
fn bind_map_placements_accepts_ambient_owner_house() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Structure,
        owner: "NEUTRAL".into(),
        type_id: "GAPOWR".into(),
        health: 128,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
        mission: ra_types::MissionName::default(),
        tag: ra_types::TagName::default(),
    }];
    let placements = ra_adaptor::bind_map_placements(&entities, &defs, &[]).expect("ambient owner");
    assert_eq!(placements[0].owner, defs.houses.get("NEUTRAL").expect("NEUTRAL").id);
}

#[test]
fn bind_map_placements_rejects_unknown_tag() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: ra_types::MissionName::default(),
        tag: "MISSINGTAG".into(),
    }];
    let err = ra_adaptor::bind_map_placements(&entities, &defs, &[]).expect_err("unknown tag");
    let msg = err.to_string();
    assert!(msg.contains("tag"), "{msg}");
    assert!(msg.contains("MISSINGTAG"), "{msg}");
}

#[test]
fn bind_map_placements_accepts_none_tag_sentinel() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: ra_types::MissionName::default(),
        tag: "None".into(),
    }];
    let placements = ra_adaptor::bind_map_placements(&entities, &defs, &[]).expect("NONE tag is absent");
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].tag, None);
}

#[test]
fn bind_map_placements_resolves_tag_id() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let triggers = ra_types::bind_map_triggers(
        &[ra_types::MapTrigger {
            id: "TR1".into(),
            house: "Americans".into(),
            linked: ra_types::TriggerName::default(),
            name: "Trig".into(),
            disabled: false,
            easy: true,
            normal: true,
            hard: true,
        }],
        &defs,
    )
    .expect("bind triggers");
    let tags = ra_types::bind_map_tags(
        &[ra_types::MapTag { id: "T1".into(), persistence: 0, name: "Start".into(), trigger_id: "TR1".into() }],
        &triggers,
    )
    .expect("bind tags");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
        mission: ra_types::MissionName::default(),
        tag: "T1".into(),
    }];
    let placements = ra_adaptor::bind_map_placements(&entities, &defs, &tags).expect("bind tag");
    assert_eq!(placements[0].tag, Some(tags[0].id));
    assert_eq!(tags[0].trigger_id, triggers[0].id);
}

#[test]
fn bind_map_cell_tags_resolves_tag_id() {
    let rules = rules_from(b"[Countries]\n0=Americans\n[Americans]\nSide=GDI\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let triggers = ra_types::bind_map_triggers(
        &[ra_types::MapTrigger {
            id: "TRZ".into(),
            house: "Neutral".into(),
            linked: ra_types::TriggerName::default(),
            name: "ZoneTrig".into(),
            disabled: false,
            easy: true,
            normal: true,
            hard: true,
        }],
        &defs,
    )
    .expect("bind triggers");
    let tags =
        ra_types::bind_map_tags(&[ra_types::MapTag { id: "ZONE".into(), persistence: 0, name: "Zone".into(), trigger_id: "TRZ".into() }], &triggers)
            .expect("bind tags");
    let cells = [ra_types::MapCellTag { x: 3, y: 4, tag_id: "ZONE".into() }];
    let bound = ra_types::bind_map_cell_tags(&cells, &tags).expect("bind cell tags");
    assert_eq!(bound.len(), 1);
    assert_eq!(bound[0].x, 3);
    assert_eq!(bound[0].y, 4);
    assert_eq!(bound[0].tag, tags[0].id);
}

#[test]
fn bind_map_cell_tags_rejects_unknown_tag() {
    let err =
        ra_types::bind_map_cell_tags(&[ra_types::MapCellTag { x: 1, y: 2, tag_id: "MISSING".into() }], &[]).expect_err("unknown cell tag");
    let msg = err.to_string();
    assert!(msg.contains("tag"), "{msg}");
    assert!(msg.contains("MISSING"), "{msg}");
}

#[test]
fn bind_map_placements_rejects_unknown_mission() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: "NotAMission".into(),
        tag: ra_types::TagName::default(),
    }];
    let err = ra_adaptor::bind_map_placements(&entities, &defs, &[]).expect_err("unknown mission");
    let msg = err.to_string();
    assert!(msg.contains("mission"), "{msg}");
    assert!(msg.contains("NOTAMISSION"), "{msg}");
}

#[test]
fn bind_map_placements_resolves_mission_kind() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[InfantryTypes]\n0=E1\n\
[E1]\nCost=100\nStrength=125\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let entities = vec![ra_types::MapPlacedEntity {
        kind: ra_types::MapPlacedEntityKind::Infantry,
        owner: "Americans".into(),
        type_id: "E1".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
        mission: "Guard".into(),
        tag: ra_types::TagName::default(),
    }];
    let placements = ra_adaptor::bind_map_placements(&entities, &defs, &[]).expect("bind mission");
    assert_eq!(placements[0].mission, Some(ra_types::MissionKind::Guard));
}

#[test]
fn bind_map_tags_resolves_trigger_id() {
    let rules = rules_from(b"[Countries]\n0=Americans\n[Americans]\nSide=GDI\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let triggers = ra_types::bind_map_triggers(
        &[
            ra_types::MapTrigger {
                id: "TR1".into(),
                house: "Americans".into(),
                linked: "<none>".into(),
                name: "First".into(),
                disabled: false,
                easy: true,
                normal: true,
                hard: true,
            },
            ra_types::MapTrigger {
                id: "TR2".into(),
                house: "Americans".into(),
                linked: "TR1".into(),
                name: "Second".into(),
                disabled: false,
                easy: true,
                normal: true,
                hard: true,
            },
        ],
        &defs,
    )
    .expect("bind triggers");
    assert_eq!(triggers.len(), 2);
    assert_eq!(triggers[0].linked, None);
    assert_eq!(triggers[1].linked, Some(triggers[0].id));
    assert_eq!(triggers[0].house, defs.houses.get("Americans").expect("Americans").id);
    let tags = ra_types::bind_map_tags(
        &[ra_types::MapTag { id: "T1".into(), persistence: 2, name: "Win".into(), trigger_id: "TR2".into() }],
        &triggers,
    )
    .expect("bind tags");
    assert_eq!(tags[0].trigger_id, triggers[1].id);
}

#[test]
fn bind_map_tags_rejects_unknown_trigger() {
    let err = ra_types::bind_map_tags(
        &[ra_types::MapTag { id: "T1".into(), persistence: 0, name: "Bad".into(), trigger_id: "MISSING".into() }],
        &[],
    )
    .expect_err("unknown trigger");
    let msg = err.to_string();
    assert!(msg.contains("trigger"), "{msg}");
    assert!(msg.contains("MISSING"), "{msg}");
}

#[test]
fn bind_map_events_and_actions_resolve_trigger_id() {
    let rules = rules_from(b"[Countries]\n0=Americans\n[Americans]\nSide=GDI\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let triggers = ra_types::bind_map_triggers(
        &[ra_types::MapTrigger {
            id: "TR1".into(),
            house: "Americans".into(),
            linked: "<none>".into(),
            name: "Timer".into(),
            disabled: false,
            easy: true,
            normal: true,
            hard: true,
        }],
        &defs,
    )
    .expect("bind triggers");
    let events = ra_types::bind_map_events(
        &[ra_types::MapEvent {
            id: "TR1".into(),
            conditions: vec![ra_types::MapEventCondition { kind_code: 13, params: vec!["10".into(), "0".into()] }],
        }],
        &triggers,
    )
    .expect("bind events");
    let actions = ra_types::bind_map_actions(
        &[ra_types::MapAction {
            id: "TR1".into(),
            commands: vec![ra_types::MapActionCommand {
                kind_code: 1,
                params: ["Americans".into(), String::new(), String::new(), String::new(), String::new(), String::new(), String::new()],
            }],
        }],
        &triggers,
        &[],
        &[],
    )
    .expect("bind actions");
    assert_eq!(events[0].trigger_id, triggers[0].id);
    assert_eq!(actions[0].trigger_id, triggers[0].id);
    assert!(actions[0].commands[0].team_id.is_none());
    assert!(actions[0].commands[0].target_trigger_id.is_none());
    assert!(actions[0].commands[0].tag_id.is_none());
}

#[test]
fn bind_map_actions_resolves_create_team_id() {
    let rules = rules_from(
        b"[Countries]\n0=Russians\n\
[Russians]\nSide=Nod\n\
[InfantryTypes]\n0=E1\n\
[E1]\nStrength=100\nOwner=Russians\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let triggers = ra_types::bind_map_triggers(
        &[ra_types::MapTrigger {
            id: "TR1".into(),
            house: "Russians".into(),
            linked: "<none>".into(),
            name: "Spawn".into(),
            disabled: false,
            easy: true,
            normal: true,
            hard: true,
        }],
        &defs,
    )
    .expect("bind triggers");
    let forces = ra_types::bind_map_task_forces(
        &[ra_types::MapTaskForce {
            id: "TF1".into(),
            name: "Squad".into(),
            entries: vec![ra_types::MapTaskForceEntry { count: 1, type_id: "E1".into() }],
            group: -1,
        }],
        &defs,
    )
    .expect("task forces");
    let teams = ra_types::bind_map_team_types(
        &[ra_types::MapTeamType {
            id: "TM1".into(),
            name: "Team".into(),
            house: "Russians".into(),
            script: ra_types::ScriptTypeName::default(),
            task_force: "TF1".into(),
            tag: ra_types::TagName::default(),
            waypoint: -1,
            max: 1,
            priority: 0,
            veteran_level: 0,
        }],
        &defs,
        &[],
        &forces,
        &[],
    )
    .expect("team types");
    let actions = ra_types::bind_map_actions(
        &[ra_types::MapAction {
            id: "TR1".into(),
            commands: vec![ra_types::MapActionCommand {
                kind_code: 4,
                params: [
                    "0".into(),
                    "TM1".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
            }],
        }],
        &triggers,
        &teams,
        &[],
    )
    .expect("bind actions");
    assert_eq!(actions[0].commands[0].team_id, Some(teams[0].id));
    assert!(actions[0].commands[0].target_trigger_id.is_none());
    assert!(actions[0].commands[0].tag_id.is_none());
}

#[test]
fn bind_map_actions_rejects_unknown_team_type() {
    let rules = rules_from(b"[Countries]\n0=Russians\n[Russians]\nSide=Nod\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let triggers = ra_types::bind_map_triggers(
        &[ra_types::MapTrigger {
            id: "TR1".into(),
            house: "Russians".into(),
            linked: "<none>".into(),
            name: "Spawn".into(),
            disabled: false,
            easy: true,
            normal: true,
            hard: true,
        }],
        &defs,
    )
    .expect("bind triggers");
    let err = ra_types::bind_map_actions(
        &[ra_types::MapAction {
            id: "TR1".into(),
            commands: vec![ra_types::MapActionCommand {
                kind_code: 4,
                params: [
                    "0".into(),
                    "NOSUCH".into(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
            }],
        }],
        &triggers,
        &[],
        &[],
    )
    .expect_err("unknown team");
    let msg = err.to_string();
    assert!(msg.contains("team_type"), "{msg}");
    assert!(msg.contains("NOSUCH"), "{msg}");
}

#[test]
fn bind_map_events_rejects_unknown_trigger() {
    let err = ra_types::bind_map_events(
        &[ra_types::MapEvent { id: "MISSING".into(), conditions: Vec::new() }],
        &[],
    )
    .expect_err("unknown event trigger");
    let msg = err.to_string();
    assert!(msg.contains("trigger"), "{msg}");
    assert!(msg.contains("MISSING"), "{msg}");
}

#[test]
fn bind_map_task_forces_resolves_techno_ids() {
    let rules = rules_from(b"[InfantryTypes]\n0=E1\n[E1]\nStrength=100\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let forces = ra_types::bind_map_task_forces(
        &[ra_types::MapTaskForce {
            id: "TF1".into(),
            name: "Squad".into(),
            entries: vec![ra_types::MapTaskForceEntry { count: 2, type_id: "E1".into() }],
            group: -1,
        }],
        &defs,
    )
    .expect("bind task forces");
    assert_eq!(forces[0].entries[0].definition_id, defs.techno.get("E1").expect("E1").id);
}

#[test]
fn bind_map_task_forces_rejects_unknown_techno() {
    let rules = rules_from(b"[InfantryTypes]\n0=E1\n[E1]\nStrength=100\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let err = ra_types::bind_map_task_forces(
        &[ra_types::MapTaskForce {
            id: "TF1".into(),
            name: "Squad".into(),
            entries: vec![ra_types::MapTaskForceEntry { count: 1, type_id: "NOSUCH".into() }],
            group: -1,
        }],
        &defs,
    )
    .expect_err("unknown techno");
    let msg = err.to_string();
    assert!(msg.contains("techno"), "{msg}");
    assert!(msg.contains("NOSUCH"), "{msg}");
}

#[test]
fn bind_map_ai_triggers_assigns_stable_ids() {
    let rules = rules_from(
        b"[Countries]\n0=Russians\n\
[Russians]\nSide=Nod\n\
[InfantryTypes]\n0=E1\n\
[E1]\nStrength=100\nOwner=Russians\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let forces = ra_types::bind_map_task_forces(
        &[ra_types::MapTaskForce {
            id: "TF1".into(),
            name: "Squad".into(),
            entries: vec![ra_types::MapTaskForceEntry { count: 1, type_id: "E1".into() }],
            group: -1,
        }],
        &defs,
    )
    .expect("task forces");
    let teams = ra_types::bind_map_team_types(
        &[ra_types::MapTeamType {
            id: "TM1".into(),
            name: "Team".into(),
            house: "Russians".into(),
            script: ra_types::ScriptTypeName::default(),
            task_force: "TF1".into(),
            tag: ra_types::TagName::default(),
            waypoint: -1,
            max: 1,
            priority: 0,
            veteran_level: 0,
        }],
        &defs,
        &[],
        &forces,
        &[],
    )
    .expect("team types");
    let triggers = ra_types::bind_map_ai_triggers(
        &[
            ra_types::MapAiTrigger {
                id: "AI1".into(),
                name: "First".into(),
                team: "TM1".into(),
                owner_house: "Russians".into(),
                tech_level: 1,
            },
            ra_types::MapAiTrigger {
                id: "AI2".into(),
                name: "Second".into(),
                team: "TM1".into(),
                owner_house: ra_types::HouseName::default(),
                tech_level: 0,
            },
        ],
        &defs,
        &teams,
    )
    .expect("ai triggers");
    assert_eq!(triggers.len(), 2);
    assert_eq!(triggers[0].id, ra_types::AiTriggerId(1));
    assert_eq!(triggers[1].id, ra_types::AiTriggerId(2));
    assert_ne!(triggers[0].id, triggers[1].id);
    assert_eq!(triggers[0].team, teams[0].id);
    assert_eq!(triggers[0].owner_house, Some(defs.houses.get("Russians").expect("Russians").id));
    assert_eq!(triggers[1].owner_house, None);
}

#[test]
fn bind_map_ai_triggers_rejects_unknown_team() {
    let rules = rules_from(b"[Countries]\n0=Russians\n[Russians]\nSide=Nod\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let err = ra_types::bind_map_ai_triggers(
        &[ra_types::MapAiTrigger {
            id: "AI1".into(),
            name: "Broken".into(),
            team: "MISSING".into(),
            owner_house: "Russians".into(),
            tech_level: 0,
        }],
        &defs,
        &[],
    )
    .expect_err("unknown team");
    let msg = err.to_string();
    assert!(msg.contains("team_type"), "{msg}");
    assert!(msg.contains("MISSING"), "{msg}");
}

#[test]
fn bind_map_triggers_rejects_unknown_linked() {
    let rules = rules_from(b"[Countries]\n0=Americans\n[Americans]\nSide=GDI\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let err = ra_types::bind_map_triggers(
        &[ra_types::MapTrigger {
            id: "TR1".into(),
            house: "Americans".into(),
            linked: "MISSING".into(),
            name: "Broken".into(),
            disabled: false,
            easy: true,
            normal: true,
            hard: true,
        }],
        &defs,
    )
    .expect_err("unknown linked");
    let msg = err.to_string();
    assert!(msg.contains("trigger"), "{msg}");
    assert!(msg.contains("MISSING"), "{msg}");
}

#[test]
fn bind_map_triggers_rejects_unknown_house() {
    let rules = rules_from(b"[Countries]\n0=Americans\n[Americans]\nSide=GDI\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let err = ra_types::bind_map_triggers(
        &[ra_types::MapTrigger {
            id: "TR1".into(),
            house: "NoSuchHouse".into(),
            linked: ra_types::TriggerName::default(),
            name: "Broken".into(),
            disabled: false,
            easy: true,
            normal: true,
            hard: true,
        }],
        &defs,
    )
    .expect_err("unknown house");
    let msg = err.to_string();
    assert!(msg.contains("house"), "{msg}");
    assert!(msg.contains("NOSUCHHOUSE"), "{msg}");
}

#[test]
fn bind_map_houses_resolves_country_and_allies() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n1=Alliance\n2=Russians\n\
[Americans]\nSide=GDI\n\
[Alliance]\nSide=GDI\n\
[Russians]\nSide=Nod\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let houses = ra_types::bind_map_houses(
        &[ra_types::MapHouse {
            name: "Player House".into(),
            country: "Americans".into(),
            tech_level: 10,
            credits: 100,
            iq: 0,
            edge: ra_types::MapEdge::North,
            player_control: true,
            color: "Gold".into(),
            allies: vec!["Alliance".into(), "None".into()],
        }],
        &defs,
    )
    .expect("bind houses");
    assert_eq!(houses.len(), 1);
    assert_eq!(houses[0].country, defs.houses.get("AMERICANS").expect("Americans").id);
    assert_eq!(houses[0].allies, vec![defs.houses.get("ALLIANCE").expect("Alliance").id]);
}

#[test]
fn bind_map_houses_rejects_unknown_country() {
    let rules = rules_from(b"[Countries]\n0=Americans\n[Americans]\nSide=GDI\n");
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let err = ra_types::bind_map_houses(
        &[ra_types::MapHouse {
            name: "Bad".into(),
            country: "Missing".into(),
            tech_level: 1,
            credits: 0,
            iq: 0,
            edge: ra_types::MapEdge::West,
            player_control: false,
            color: ra_types::ColorName::default(),
            allies: Vec::new(),
        }],
        &defs,
    )
    .expect_err("unknown country");
    let msg = err.to_string();
    assert!(msg.contains("house"), "{msg}");
    assert!(msg.contains("MISSING"), "{msg}");
}
