//! `GameAssetSource::discover_skirmish_map_names` 松散文件扫描。

use std::fs;

use ra_widgets::fs_source::GameAssetSource;

#[test]
fn discover_lists_loose_map_and_mpr_case_insensitively() {
    let dir = tempfile_dir("ra_discover_maps");
    fs::write(dir.join("Custom.MAP"), b"[Map]\n").unwrap();
    fs::write(dir.join("other.mpr"), b"[Map]\n").unwrap();
    fs::write(dir.join("readme.txt"), b"nope").unwrap();

    let source = GameAssetSource::new(dir.clone());
    let names = source.discover_skirmish_map_names();
    assert_eq!(names, vec!["custom.map".to_string(), "other.mpr".to_string()]);

    let _ = fs::remove_dir_all(&dir);
}

fn tempfile_dir(prefix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("{prefix}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}
