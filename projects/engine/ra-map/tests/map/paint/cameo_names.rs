//! `PaintDefinitions::cameo_asset_names`：候选名解析，不暴露 `IniDocument`。

use ra_map::{PaintDefinitions, PaintDefinitionsLoader};
use ra_types::{AssetSource, RaError, RaResult};
use std::collections::HashMap;

struct MapSource {
    files: HashMap<String, Vec<u8>>,
}

impl AssetSource for MapSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files.get(&relative.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

#[test]
fn cameo_asset_names_prefer_pcx_and_follow_image_redirect() {
    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[GACNST]
Image=GACNSTX
Cameo=GAICON
CameoPCX=gaicon
AltCameo=GAICONA
[GACNSTX]
Cameo=GAICONX
CameoPCX=gaiconx
"#
        .to_vec(),
    );
    files.insert("rules.ini".into(), b"[General]\n".to_vec());
    let source = MapSource { files };
    let mut loader = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini");
    // 先解析进 hint，再丢文档：产品路径只读缓存，不再持有 IniDocument。
    let names = loader.cameo_asset_names("GACNST");
    let mut paint = loader.drop_documents();
    assert!(paint.documents_sealed());
    assert_eq!(names.pcx, vec!["gaicon.pcx".to_string(), "gaiconx.pcx".to_string()]);
    assert!(names.shp.iter().any(|n| n == "GAICON.shp"));
    assert!(names.shp.iter().any(|n| n == "GAICONX.shp"));
    assert!(names.shp.iter().any(|n| n == "GAICONA.shp"));
    assert!(names.shp.iter().any(|n| n == "GACNSTicon.shp"));
    assert!(names.shp.iter().any(|n| n == "GACNST.shp"));
    assert!(names.shp.iter().any(|n| n == "GACNSTXicon.shp"));
    assert!(names.shp.iter().any(|n| n == "GACNSTX.shp"));
    let again = paint.cameo_asset_names("GACNST");
    assert_eq!(again, names);
}

#[test]
fn cameo_asset_names_follow_rules_image_when_art_section_missing() {
    // 对齐美军空指 `AMRADR`：rules `Image=GAAIRC`，art 无本类节，图标在目标节。
    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[GAAIRC]
Cameo=HELIICON
CameoPCX=heliicon
AltCameo=HELIICONA
"#
        .to_vec(),
    );
    files.insert(
        "rules.ini".into(),
        br#"[AMRADR]
Image=GAAIRC
"#
        .to_vec(),
    );
    let source = MapSource { files };
    let mut loader = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini");
    let names = loader.cameo_asset_names("AMRADR");
    let mut paint = loader.drop_documents();
    assert!(paint.documents_sealed());
    assert_eq!(names.pcx, vec!["heliicon.pcx".to_string()]);
    assert!(names.shp.iter().any(|n| n == "HELIICON.shp"));
    assert!(names.shp.iter().any(|n| n == "HELIICONA.shp"));
    assert!(names.shp.iter().any(|n| n == "AMRADRicon.shp"));
    assert!(names.shp.iter().any(|n| n == "GAAIRCicon.shp"));
    assert_eq!(paint.cameo_asset_names("AMRADR"), names);
}

#[test]
fn cameo_asset_names_rules_image_overrides_art_image() {
    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[AMRADR]
Image=WRONG
Cameo=LOCALICON
[GAAIRC]
Cameo=HELIICON
[WRONG]
Cameo=WRONGICON
"#
        .to_vec(),
    );
    files.insert(
        "rules.ini".into(),
        br#"[AMRADR]
Image=GAAIRC
"#
        .to_vec(),
    );
    let source = MapSource { files };
    let mut loader = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini");
    let names = loader.cameo_asset_names("AMRADR");
    assert!(names.shp.iter().any(|n| n == "LOCALICON.shp"));
    assert!(names.shp.iter().any(|n| n == "HELIICON.shp"));
    assert!(!names.shp.iter().any(|n| n == "WRONGICON.shp"));
}

#[test]
fn cameo_asset_names_without_art_only_type_fallbacks() {
    let mut paint = PaintDefinitions::default();
    let names = paint.cameo_asset_names("E1");
    assert!(names.pcx.is_empty());
    assert_eq!(names.shp, vec!["E1icon.shp".to_string(), "E1.shp".to_string()]);
}

#[test]
fn cameo_asset_names_top_layer_overrides_underlay() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[E1]\nCameoPCX=base\nCameo=BASEICON\n".to_vec());
    files.insert("artmd.ini".into(), b"[E1]\nCameoPCX=md\n".to_vec());
    files.insert("rules.ini".into(), b"[General]\n".to_vec());
    let source = MapSource { files };
    let mut loader = PaintDefinitionsLoader::load_files(&source, &["art.ini", "artmd.ini"], &["rules.ini"]);
    let names = loader.cameo_asset_names("E1");
    let mut paint = loader.drop_documents();
    assert!(paint.documents_sealed());
    assert_eq!(names.pcx, vec!["md.pcx".to_string()]);
    assert!(names.shp.iter().any(|n| n == "BASEICON.shp"));
    assert_eq!(paint.cameo_asset_names("E1"), names);
}

#[test]
fn seal_with_runtime_drops_ini_and_keeps_preloaded_cameo() {
    use ra_map::{MapEntity, MapEntityKind, MapInfo};
    use ra_types::{ArmorKind, Foundation, GameEdition, HouseAllowList, PowerProfile, RuntimeDefinitions, StructureDefinition, TypeId};

    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[GACNST]
Cameo=GAICON
CameoPCX=gaicon
"#
        .to_vec(),
    );
    files.insert("rules.ini".into(), b"[General]\n".to_vec());
    let source = MapSource { files };
    let loader = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini");
    assert!(!loader.documents_sealed());

    let mut defs = RuntimeDefinitions::default();
    defs.structures.insert(StructureDefinition {
        id: TypeId(1),
        type_key: "GACNST".into(),
        power: PowerProfile::default(),
        cost: 0,
        strength: 1,
        armor: ArmorKind::None,
        construction_yard: false,
        refinery: false,
        free_unit: None,
        radar: false,
        build_cat: Default::default(),
        capturable: false,
        production: None,
        owner: HouseAllowList::empty(),
        owner_ids: ra_types::HouseIdAllowList::empty(),
        foundation: Foundation::default(),
        height: 2,
        super_weapon: None,
        super_weapon_id: None,
        light: None,
        capabilities: Vec::new(),
    });
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 0,
        y: 0,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });

    let mut paint = loader.seal_with_runtime(&defs, &map);
    assert!(paint.documents_sealed());
    let names = paint.cameo_asset_names("GACNST");
    assert_eq!(names.pcx, vec!["gaicon.pcx".to_string()]);
    assert!(names.shp.iter().any(|n| n == "GAICON.shp"));
}

#[test]
fn drop_documents_clears_ini_without_scanning_hints() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[GACNST]\nCameo=GAICON\n".to_vec());
    files.insert("rules.ini".into(), b"[General]\n".to_vec());
    let source = MapSource { files };
    let loader = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini");
    assert!(!loader.documents_sealed());
    let mut paint = loader.drop_documents();
    assert!(paint.documents_sealed());
    // 未 seal 扫描时 cameo 回退到类型名，不得再读已丢弃文档。
    let names = paint.cameo_asset_names("GACNST");
    assert!(names.pcx.is_empty());
    assert_eq!(names.shp, vec!["GACNSTicon.shp".to_string(), "GACNST.shp".to_string()]);
}

#[test]
fn ensure_after_seal_uses_name_fallback_without_ini() {
    use ra_types::TechnoName;

    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[GACNST]
Cameo=GAICON
Image=SECRETBODY
"#
        .to_vec(),
    );
    files.insert("rules.ini".into(), b"[General]\n".to_vec());
    let source = MapSource { files };
    let mut paint = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini").drop_documents();
    assert!(paint.documents_sealed());

    // seal 后对新类型 ensure：只能得到名称回退，不能再读到 SECRETBODY。
    paint.ensure_structure_hint(&TechnoName::parse("GAPOWR"));
    paint.ensure_cameo_hint("GAPOWR");
    let names = paint.cameo_asset_names("GAPOWR");
    assert!(names.pcx.is_empty());
    assert_eq!(names.shp[0], "GAPOWRicon.shp");
    assert!(paint.documents_sealed());
}
