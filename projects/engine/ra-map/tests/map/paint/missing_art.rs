//! seal 后建筑 / 机动 / 地形 art 缺失诊断。

use ra_map::{MapEntity, MapEntityKind, MapInfo, PaintDefinitionsLoader, TerrainObject};
use ra_types::{AssetSource, GameEdition, RaError, RaResult, RuntimeDefinitions};
use std::collections::HashMap;

struct MapSource {
    files: HashMap<String, Vec<u8>>,
}

impl AssetSource for MapSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files.get(&relative.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

fn structure(type_id: &str, x: u16, y: u16) -> MapEntity {
    MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: type_id.into(),
        health: 256,
        x,
        y,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }
}

fn mobile(kind: MapEntityKind, type_id: &str, x: u16, y: u16) -> MapEntity {
    MapEntity {
        kind,
        owner: "AMERICANS".into(),
        type_id: type_id.into(),
        health: 256,
        x,
        y,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }
}

#[test]
fn seal_reports_structures_missing_art_sections() {
    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[GAPOWR]
Image=GAPOWR
"#
        .to_vec(),
    );
    files.insert(
        "rules.ini".into(),
        br#"[General]
"#
        .to_vec(),
    );
    let source = MapSource { files };

    let mut map = MapInfo::empty(GameEdition::Ra2, "diag");
    map.entities = vec![structure("GAPOWR", 1, 1), structure("GACNST", 2, 2)];

    let paint = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini").seal_with_runtime(&RuntimeDefinitions::default(), &map);
    let missing = paint.structure_types_missing_art();
    assert!(missing.iter().any(|k| k.eq_ignore_ascii_case("GACNST")), "{missing:?}");
    assert!(!missing.iter().any(|k| k.eq_ignore_ascii_case("GAPOWR")), "{missing:?}");
}

#[test]
fn seal_follows_rules_image_for_structure_without_own_art_section() {
    // 美军空指：rules `Image=GAAIRC`，art 无 `[AMRADR]`，主体几何在 `[GAAIRC]`。
    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[GAAIRC]
Foundation=3x2
Height=7
Buildup=GAAIRCMK
Cameo=HELIICON
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

    let mut map = MapInfo::empty(GameEdition::Ra2, "amradr");
    map.entities = vec![structure("AMRADR", 1, 1)];

    let paint = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini").seal_with_runtime(&RuntimeDefinitions::default(), &map);
    let missing = paint.structure_types_missing_art();
    assert!(!missing.iter().any(|k| k.eq_ignore_ascii_case("AMRADR")), "AMRADR should resolve via rules Image=GAAIRC: {missing:?}");
    let cameo = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini").cameo_asset_names("AMRADR");
    assert!(cameo.shp.iter().any(|n| n.eq_ignore_ascii_case("HELIICON.shp")), "cameo should follow GAAIRC art Cameo=: {cameo:?}");
}

#[test]
fn seal_reports_mobiles_and_terrain_missing_art_sections() {
    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[E1]
Image=E1
[TREE01]
Image=TREE01
"#
        .to_vec(),
    );
    files.insert(
        "rules.ini".into(),
        br#"[General]
"#
        .to_vec(),
    );
    let source = MapSource { files };

    let mut map = MapInfo::empty(GameEdition::Ra2, "diag-mt");
    map.entities = vec![mobile(MapEntityKind::Infantry, "E1", 1, 1), mobile(MapEntityKind::Unit, "MTNK", 2, 2)];
    map.terrain_objects = vec![TerrainObject { x: 3, y: 3, name: "TREE01".into() }, TerrainObject { x: 4, y: 4, name: "TREE99".into() }];

    let paint = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini").seal_with_runtime(&RuntimeDefinitions::default(), &map);
    let missing_mobile = paint.mobile_types_missing_art();
    assert!(missing_mobile.iter().any(|k| k.eq_ignore_ascii_case("MTNK")), "{missing_mobile:?}");
    assert!(!missing_mobile.iter().any(|k| k.eq_ignore_ascii_case("E1")), "{missing_mobile:?}");

    let missing_terrain = paint.terrain_types_missing_art();
    assert!(missing_terrain.iter().any(|k| k.eq_ignore_ascii_case("TREE99")), "{missing_terrain:?}");
    assert!(!missing_terrain.iter().any(|k| k.eq_ignore_ascii_case("TREE01")), "{missing_terrain:?}");
}

#[test]
fn seal_reports_overlays_missing_art_sections() {
    let mut files = HashMap::new();
    files.insert(
        "art.ini".into(),
        br#"[LOBRDG01]
Image=LOBRDG01
Theater=yes
"#
        .to_vec(),
    );
    files.insert(
        "rules.ini".into(),
        br#"[General]
"#
        .to_vec(),
    );
    let source = MapSource { files };

    let mut map = MapInfo::empty(GameEdition::Ra2, "diag-ov");
    map.overlays = vec![ra_map::OverlayCell { x: 1, y: 1, overlay_id: 0, data: 0 }, ra_map::OverlayCell { x: 2, y: 2, overlay_id: 1, data: 0 }];

    let paint = PaintDefinitionsLoader::load_sealed_for_overlays(
        &source,
        "art.ini",
        "rules.ini",
        &map,
        &|id| match id {
            0 => Some("LOBRDG01".into()),
            1 => Some("MISSINGOV".into()),
            _ => None,
        },
        &|_| false,
    );
    let missing = paint.overlay_types_missing_art();
    assert!(missing.iter().any(|k| k.eq_ignore_ascii_case("MISSINGOV")), "{missing:?}");
    assert!(!missing.iter().any(|k| k.eq_ignore_ascii_case("LOBRDG01")), "{missing:?}");
}
