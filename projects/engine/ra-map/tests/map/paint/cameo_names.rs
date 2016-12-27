//! `PaintDefinitions::cameo_asset_names`：候选名解析，不暴露 `IniDocument`。

use ra_map::PaintDefinitions;
use ra_types::{AssetSource, RaError, RaResult};
use std::collections::HashMap;

struct MapSource {
    files: HashMap<String, Vec<u8>>,
}

impl AssetSource for MapSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files
            .get(&relative.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| RaError::MissingFile(relative.to_string()))
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
    let mut paint = PaintDefinitions::load(&source, "art.ini", "rules.ini");
    let names = paint.cameo_asset_names("GACNST");
    assert_eq!(names.pcx, vec!["gaicon.pcx".to_string(), "gaiconx.pcx".to_string()]);
    assert!(names.shp.iter().any(|n| n == "GAICON.shp"));
    assert!(names.shp.iter().any(|n| n == "GAICONX.shp"));
    assert!(names.shp.iter().any(|n| n == "GAICONA.shp"));
    assert!(names.shp.ends_with(&["GACNSTicon.shp".to_string(), "GACNST.shp".to_string()]));
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
    let mut paint = PaintDefinitions::load_files(&source, &["art.ini", "artmd.ini"], &["rules.ini"]);
    let names = paint.cameo_asset_names("E1");
    assert_eq!(names.pcx, vec!["md.pcx".to_string()]);
    assert!(names.shp.iter().any(|n| n == "BASEICON.shp"));
}
