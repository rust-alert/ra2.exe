//! seal 后建筑 art 缺失诊断。

use ra_map::{MapEntity, MapEntityKind, MapInfo, PaintDefinitionsLoader};
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
