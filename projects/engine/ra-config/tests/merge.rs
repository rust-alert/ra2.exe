use std::path::PathBuf;

use ra_config::{ConfigLayer, ConfigTable, DesktopSettings, MergedConfig, RustAlertDocument, parse_toml_document, present_feel_from_toml_text};
use ra_types::DisplayMode;

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


#[test]
fn display_mode_from_merged_and_alias() {
    let mut table = ConfigTable::new();
    table.insert("ra2_dir", ".");
    table.insert("display_mode", "800x600");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert_eq!(s.display_mode, DisplayMode::W800H600);

    let mut table = ConfigTable::new();
    table.insert("resolution", "640x480");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    assert_eq!(DesktopSettings::from_merged(&merged).display_mode, DisplayMode::W640H480);
}

#[test]
fn unknown_display_mode_keeps_default() {
    let mut table = ConfigTable::new();
    table.insert("display_mode", "1920x1080");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    assert_eq!(DesktopSettings::from_merged(&merged).display_mode, DisplayMode::DEFAULT);
}

#[test]
fn audio_volumes_from_merged_and_aliases() {
    let mut table = ConfigTable::new();
    table.insert("music_volume", "0.25");
    table.insert("sound_volume", "0.9");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert!((s.music_volume - 0.25).abs() < 1e-6);
    assert!((s.sound_volume - 0.9).abs() < 1e-6);

    let mut table = ConfigTable::new();
    table.insert("score_volume", "1.5");
    table.insert("sfx_volume", "-0.2");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert!((s.music_volume - 1.0).abs() < 1e-6);
    assert!((s.sound_volume - 0.0).abs() < 1e-6);
}

#[test]
fn invalid_audio_volume_keeps_default() {
    let mut table = ConfigTable::new();
    table.insert("music_volume", "loud");
    table.insert("sound_volume", "nan");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert!((s.music_volume - 0.4).abs() < 1e-6);
    assert!((s.sound_volume - 0.7).abs() < 1e-6);
}

#[test]
fn present_table_serde_roundtrip_preserves_other_keys() {
    let dir = std::env::temp_dir()
        .join(format!("ra_config_present_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("RustAlert.toml");
    std::fs::write(
        &path,
        "# keep me\nra2_dir = \"C:/Games/RA2\"\n\n[present]\nmode = \"off\"\ngamma = 1.0\ndither = false\n",
    )
    .unwrap();

    let mut doc = RustAlertDocument::open(&path).unwrap();
    let (feel, diags) = doc.present_feel();
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(feel.mode, ra_types::PresentMode::Off);

    let mut next = feel;
    next.mode = ra_types::PresentMode::Bit16;
    next.gamma = 1.2;
    next.dither = true;
    doc.set_present_feel(&next).unwrap();
    doc.save().unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# keep me"), "{text}");
    assert!(text.contains("ra2_dir"), "{text}");
    assert!(text.contains("[present]"), "{text}");
    assert!(text.contains("16bit"), "{text}");

    let (again, diags) = present_feel_from_toml_text(&text, "t");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(again.mode, ra_types::PresentMode::Bit16);
    assert!((again.gamma - 1.2).abs() < 1e-6);
    assert!(again.dither);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn parse_toml_skips_present_table_without_diag() {
    let (t, d) = parse_toml_document("ra2_dir = \".\"\n\n[present]\nmode = \"16bit\"\n", "t");
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(t.get("ra2_dir"), Some("."));
    assert!(t.get("mode").is_none());
}
