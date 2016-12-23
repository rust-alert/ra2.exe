//! 层叠 rules 装载入口。

use std::collections::HashMap;

use ra_adaptor::{
    ResourceChain, build_runtime_definitions, load_rules_chain, load_rules_chain_with_overlays,
    rules_system_from_layered_ini_bytes,
};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

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

    let defs = build_runtime_definitions(&rules).expect("freeze");
    let techno = defs.techno.get("MTNK").expect("def");
    assert_eq!(techno.cost, 800);
    assert!(techno.owner.owner_allows("Americans"));
    assert!(techno.owner.owner_allows("Alliance"));
    assert_ne!(techno.primary_id, ra_types::WeaponId(0));
}

struct MapAssetSource {
    files: HashMap<&'static str, &'static [u8]>,
}

impl AssetSource for MapAssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files
            .get(relative)
            .map(|b| b.to_vec())
            .ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn load_rules_chain_with_overlays_applies_mp_cost() {
    let base = b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nOwner=Americans\nCost=700\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let overlay = b"[MTNK]\nCost=900\n";
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let source = MapAssetSource {
        files: HashMap::from([
            (chain.rules_ini, base.as_slice()),
            (chain.art_ini, b"".as_slice()),
            ("MPBattle.ini", overlay.as_slice()),
        ]),
    };
    let rules = load_rules_chain_with_overlays(&source, &chain, &["MPBattle.ini"], &[]).expect("overlay load");
    assert_eq!(rules.techno_types.get("MTNK").expect("MTNK").cost, 900);
}

#[test]
fn load_rules_chain_with_overlays_missing_file_errors() {
    let base = b"[VehicleTypes]\n0=MTNK\n[MTNK]\nCost=700\n";
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let source = MapAssetSource {
        files: HashMap::from([(chain.rules_ini, base.as_slice()), (chain.art_ini, b"".as_slice())]),
    };
    let err = load_rules_chain_with_overlays(&source, &chain, &["MPMissing.ini"], &[]).expect_err("missing overlay");
    assert!(matches!(err, RaError::MissingFile(_)));
}

#[test]
fn load_rules_chain_with_art_overlays_overrides_foundation() {
    let rules = b"[BuildingTypes]\n0=GACNST\n[GACNST]\nStrength=1000\nFoundation=4x4\nHeight=4\n";
    let art_base = b"[GACNST]\nFoundation=3x3\nHeight=3\n";
    let art_top = b"[GACNST]\nFoundation=5x5\nHeight=7\n";
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let source = MapAssetSource {
        files: HashMap::from([
            (chain.rules_ini, rules.as_slice()),
            (chain.art_ini, art_base.as_slice()),
            ("artmod.ini", art_top.as_slice()),
        ]),
    };
    let loaded = load_rules_chain_with_overlays(&source, &chain, &[], &["artmod.ini"]).expect("art overlay");
    let building = loaded.techno_types.get("GACNST").expect("GACNST");
    assert_eq!((building.foundation.width, building.foundation.height), (5, 5));
    assert_eq!(building.height, Some(7));
}

#[test]
fn load_rules_chain_applies_edition_underlay_then_primary() {
    // 模拟 YR：`rules.ini` underlay + `rulesmd.ini` primary。
    let underlay = b"[VehicleTypes]\n0=MTNK\n[MTNK]\nOwner=Americans\nCost=700\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let primary = b"[MTNK]\nCost=850\n";
    let chain = ResourceChain::for_edition(GameEdition::Yr);
    assert_eq!(chain.rules_underlay, &["rules.ini"]);
    assert_eq!(chain.rules_ini, "rulesmd.ini");
    let source = MapAssetSource {
        files: HashMap::from([
            ("rules.ini", underlay.as_slice()),
            (chain.rules_ini, primary.as_slice()),
            (chain.art_ini, b"".as_slice()),
        ]),
    };
    let rules = load_rules_chain(&source, &chain).expect("yr underlay");
    assert_eq!(rules.techno_types.get("MTNK").expect("MTNK").cost, 850);
    assert!(rules.techno_types.get("MTNK").expect("MTNK").owner.owner_allows("Americans"));

    // underlay 缺文件时不阻断，只读 primary。
    let primary_only = b"[VehicleTypes]\n0=MTNK\n[MTNK]\nOwner=Americans\nCost=860\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let source_primary_only = MapAssetSource {
        files: HashMap::from([(chain.rules_ini, primary_only.as_slice()), (chain.art_ini, b"".as_slice())]),
    };
    let rules = load_rules_chain(&source_primary_only, &chain).expect("missing underlay ok");
    assert_eq!(rules.techno_types.get("MTNK").expect("MTNK").cost, 860);
}
