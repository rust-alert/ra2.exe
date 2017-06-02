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
    files.insert("rules.ini".into(), br#"[General]
"#.to_vec());
    let source = MapSource { files };

    let mut map = MapInfo::empty(GameEdition::Ra2, "diag");
    map.entities = vec![structure("GAPOWR", 1, 1), structure("GACNST", 2, 2)];

    let paint = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini").seal_with_runtime(&RuntimeDefinitions::default(), &map);
    let missing = paint.structure_types_missing_art();
    assert!(missing.iter().any(|k| k.eq_ignore_ascii_case("GACNST")), "{missing:?}");
    assert!(!missing.iter().any(|k| k.eq_ignore_ascii_case("GAPOWR")), "{missing:?}");
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
    files.insert("rules.ini".into(), br#"[General]
"#.to_vec());
    let source = MapSource { files };

    let mut map = MapInfo::empty(GameEdition::Ra2, "diag-mt");
    map.entities = vec![
        mobile(MapEntityKind::Infantry, "E1", 1, 1),
        mobile(MapEntityKind::Unit, "MTNK", 2, 2),
    ];
    map.terrain_objects = vec![
        TerrainObject { x: 3, y: 3, name: "TREE01".into() },
        TerrainObject { x: 4, y: 4, name: "TREE99".into() },
    ];

    let paint = PaintDefinitionsLoader::load(&source, "art.ini", "rules.ini").seal_with_runtime(&RuntimeDefinitions::default(), &map);
    let missing_mobile = paint.mobile_types_missing_art();
    assert!(missing_mobile.iter().any(|k| k.eq_ignore_ascii_case("MTNK")), "{missing_mobile:?}");
    assert!(!missing_mobile.iter().any(|k| k.eq_ignore_ascii_case("E1")), "{missing_mobile:?}");

    let missing_terrain = paint.terrain_types_missing_art();
    assert!(missing_terrain.iter().any(|k| k.eq_ignore_ascii_case("TREE99")), "{missing_terrain:?}");
    assert!(!missing_terrain.iter().any(|k| k.eq_ignore_ascii_case("TREE01")), "{missing_terrain:?}");
}
