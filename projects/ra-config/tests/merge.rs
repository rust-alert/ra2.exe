use std::path::PathBuf;

use ra_config::{ConfigLayer, ConfigTable, DesktopSettings, MergedConfig, RustAlertDocument, parse_toml_document};

#[test]
fn later_layer_overrides() {
    let mut a = ConfigTable::new();
    a.insert("ra2_dir", ".");
    let mut b = ConfigTable::new();
    b.insert("ra2_dir", "C:/games/ra2");
    b.insert("edition", "yr");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "a".into(), table: a }, ConfigLayer { label: "b".into(), table: b }]);
    let s = DesktopSettings::from_merged(&merged);
    assert_eq!(s.ra2_dir, PathBuf::from("C:/games/ra2"));
    assert_eq!(s.edition.as_deref(), Some("yr"));
}

#[test]
fn parse_toml_keeps_string_keys_and_skips_comments() {
    let (t, d) = parse_toml_document("# hi\nra2_dir = \"D:/RA2\"\nedition = \"yr\"\n", "t");
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(t.get("ra2_dir"), Some("D:/RA2"));
    assert_eq!(t.get("edition"), Some("yr"));
}

#[test]
fn rust_alert_document_round_trip_preserves_comment() {
    let dir = std::env::temp_dir()
        .join(format!("ra_config_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("RustAlert.toml");
    std::fs::write(&path, "# keep me\nra2_dir = \"C:/Games/RA2\"\n").unwrap();

    let mut doc = RustAlertDocument::open(&path).unwrap();
    assert_eq!(doc.get_str("ra2_dir").as_deref(), Some("C:/Games/RA2"));
    doc.set_str("edition", "yr");
    doc.save().unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# keep me"), "{text}");
    assert!(text.contains("edition"), "{text}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ensure_creates_missing_toml_once() {
    let dir = std::env::temp_dir()
        .join(format!("ra_config_ensure_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("RustAlert.toml");
    assert!(!path.is_file());

    assert!(ra_config::ensure_rust_alert_toml(&path).unwrap());
    assert!(path.is_file());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("ra2_dir"), "{text}");
    assert!(!ra_config::ensure_rust_alert_toml(&path).unwrap());

    let _ = std::fs::remove_dir_all(&dir);
}
