use std::path::PathBuf;

use ra_config::{ConfigLayer, ConfigTable, DesktopSettings, MergedConfig, parse_kv_toml_lite};

#[test]
fn later_layer_overrides() {
    let mut a = ConfigTable::new();
    a.insert("ra2_dir", ".");
    let mut b = ConfigTable::new();
    b.insert("ra2_dir", "C:/games/ra2");
    b.insert("edition", "yr");
    let merged =
        MergedConfig::merge_layers(&[ConfigLayer { label: "a".into(), table: a }, ConfigLayer { label: "b".into(), table: b }]);
    let s = DesktopSettings::from_merged(&merged);
    assert_eq!(s.ra2_dir, PathBuf::from("C:/games/ra2"));
    assert_eq!(s.edition.as_deref(), Some("yr"));
}

#[test]
fn parse_kv_skips_comments() {
    let (t, d) = parse_kv_toml_lite("# hi\nra2_dir = \"D:/RA2\"\nedition = 'yr'\n", "t");
    assert!(d.is_empty());
    assert_eq!(t.get("ra2_dir"), Some("D:/RA2"));
    assert_eq!(t.get("edition"), Some("yr"));
}
