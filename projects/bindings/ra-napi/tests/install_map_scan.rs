//! 本机安装扫图计数（需工作区根目录 `RustAlert.toml` 的 `ra2_dir`）。
//! `cargo test -p ra-napi --test install_map_scan -- --ignored --nocapture`

use std::path::PathBuf;

use ra_adaptor::detect_edition;
use ra_map::{decode_preview_from_map_bytes, list_parseable_maps_from_names};
use ra_types::{AssetSource, GameEdition};
use ra_widgets::fs_source::GameAssetSource;

fn workspace_rust_alert_toml() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..").join("RustAlert.toml")
}

#[test]
#[ignore = "需要本机安装目录"]
fn install_discover_and_parse_skirmish_maps() {
    let cfg_path = workspace_rust_alert_toml();
    let text = std::fs::read_to_string(&cfg_path).unwrap_or_else(|e| panic!("读取 {}: {e}", cfg_path.display()));
    let mut ra2_dir = None;
    let mut edition = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("ra2_dir") {
            let v = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            ra2_dir = Some(v.to_string());
        }
        if let Some(rest) = line.strip_prefix("edition") {
            let v = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            edition = Some(v.to_string());
        }
    }
    let root = PathBuf::from(ra2_dir.expect("ra2_dir"));
    let explicit = edition.as_deref().and_then(|s| GameEdition::parse(s).ok());
    let manifest = detect_edition(&root, explicit).expect("detect_edition");
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    let names = source.discover_skirmish_map_names();
    eprintln!("discovered={}", names.len());
    for n in names.iter().take(20) {
        eprintln!("  {n}");
    }
    if names.len() > 20 {
        eprintln!("  …");
    }
    let maps = list_parseable_maps_from_names(manifest.chain.edition, &source, &names);
    eprintln!("parseable={}", maps.len());
    assert!(names.len() > 4, "expected more than boot table, got {}", names.len());
    assert!(!maps.is_empty(), "expected at least one parseable map");

    let sample = maps.iter().find(|m| m.file_name.eq_ignore_ascii_case("mp03t4.map")).unwrap_or(&maps[0]);
    let bytes = source.read(&sample.file_name).expect("read map");
    let preview = decode_preview_from_map_bytes(&bytes).expect("parse preview").expect("PreviewPack present");
    eprintln!(
        "preview_pack map={} {}x{} rgba={}",
        sample.file_name,
        preview.width,
        preview.height,
        preview.rgba.len()
    );
    assert!(preview.width >= 32 && preview.height >= 32);
    assert_eq!(preview.rgba.len(), (preview.width as usize) * (preview.height as usize) * 4);
}
