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
        vec![
            ra_types::PrerequisiteToken::Type(gaweap_id),
            ra_types::PrerequisiteToken::Group(ra_types::PrerequisiteGroupKind::Power),
        ]
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
    assert_eq!(defs.stolen_tech_by_house.get("Americans"), Some(ra_types::StolenTechKind::Allied));
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
fn bind_map_placements_resolves_tag_id() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[BuildingTypes]\n0=GAPOWR\n\
[GAPOWR]\nCost=600\nStrength=600\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let tags = ra_types::bind_map_tags(&[ra_types::MapTag {
        id: "T1".into(),
        persistence: 0,
        name: "Start".into(),
        trigger_id: "TR1".into(),
    }]);
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
}

#[test]
fn bind_map_cell_tags_resolves_tag_id() {
    let tags = ra_types::bind_map_tags(&[ra_types::MapTag {
        id: "ZONE".into(),
        persistence: 0,
        name: "Zone".into(),
        trigger_id: "TRZ".into(),
    }]);
    let cells = [ra_types::MapCellTag {
        x: 3,
        y: 4,
        tag_id: "ZONE".into(),
    }];
    let bound = ra_types::bind_map_cell_tags(&cells, &tags).expect("bind cell tags");
    assert_eq!(bound.len(), 1);
    assert_eq!(bound[0].x, 3);
    assert_eq!(bound[0].y, 4);
    assert_eq!(bound[0].tag, tags[0].id);
}

#[test]
fn bind_map_cell_tags_rejects_unknown_tag() {
    let err = ra_types::bind_map_cell_tags(
        &[ra_types::MapCellTag {
            x: 1,
            y: 2,
            tag_id: "MISSING".into(),
        }],
        &[],
    )
    .expect_err("unknown cell tag");
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
fn bind_map_task_forces_resolves_techno_ids() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[InfantryTypes]\n0=E1\n\
[E1]\nCost=100\nStrength=125\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let forces = vec![ra_types::MapTaskForce {
        id: "TF1".into(),
        name: "Squad".into(),
        entries: vec![ra_types::MapTaskForceEntry { count: 2, type_id: "E1".into() }],
        group: -1,
    }];
    let bound = ra_adaptor::bind_map_task_forces(&forces, &defs).expect("bind task force");
    assert_eq!(bound[0].entries[0].definition_id, defs.techno.get("E1").expect("E1").id);
    assert_eq!(bound[0].entries[0].count, 2);
}

#[test]
fn bind_map_task_forces_rejects_unknown_techno() {
    let rules = rules_from(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\n\
[InfantryTypes]\n0=E1\n\
[E1]\nCost=100\nStrength=125\nOwner=Americans\n",
    );
    let defs = build_runtime_definitions(&rules).expect("freeze");
    let forces = vec![ra_types::MapTaskForce {
        id: "TF1".into(),
        name: "Squad".into(),
        entries: vec![ra_types::MapTaskForceEntry { count: 1, type_id: "MISSING".into() }],
        group: -1,
    }];
    let err = ra_adaptor::bind_map_task_forces(&forces, &defs).expect_err("unknown techno");
    let msg = err.to_string();
    assert!(msg.contains("techno"), "{msg}");
    assert!(msg.contains("MISSING"), "{msg}");
}
