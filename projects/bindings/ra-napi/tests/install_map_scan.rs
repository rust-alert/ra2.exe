//! 本机安装扫图（**不进默认 CI**）。
//!
//! 运行：`RA2_DIR=… cargo test -p ra-napi --test install_map_scan -- --ignored --nocapture`
//! 也可读工作区根 `RustAlert.toml` 的 `ra2_dir`（该文件已 gitignore）。
//! 缺安装目录时直接 return，禁止硬编码盘符路径。

use std::path::PathBuf;

use ra_adaptor::detect_edition;
use ra_map::{decode_preview_from_map_bytes, list_parseable_maps_from_missions_pkt, list_parseable_maps_from_names};
use ra_types::{AssetSource, GameEdition};
use ra_widgets::fs_source::GameAssetSource;

fn resolve_install_root() -> Option<(PathBuf, Option<String>)> {
    if let Ok(dir) = std::env::var("RA2_DIR") {
        let root = PathBuf::from(dir.trim());
        if root.is_dir() {
            return Some((root, std::env::var("RA2_EDITION").ok()));
        }
    }
    let cfg_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..").join("RustAlert.toml");
    let text = std::fs::read_to_string(&cfg_path).ok()?;
    let mut ra2_dir = None;
    let mut edition = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("ra2_dir") {
            let v = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            if !v.is_empty() {
                ra2_dir = Some(PathBuf::from(v));
            }
        }
        if let Some(rest) = line.strip_prefix("edition") {
            let v = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            if !v.is_empty() {
                edition = Some(v.to_string());
            }
        }
    }
    let root = ra2_dir.filter(|p| p.is_dir())?;
    Some((root, edition))
}

#[test]
#[ignore = "需要本机安装：设 RA2_DIR 或本地 RustAlert.toml"]
fn install_discover_and_parse_skirmish_maps() {
    let Some((root, edition)) = resolve_install_root()
    else {
        eprintln!("skip · 未找到安装目录（RA2_DIR / RustAlert.toml）");
        return;
    };
    let explicit = edition.as_deref().and_then(|s| GameEdition::parse(s).ok());
    let manifest = detect_edition(&root, explicit).expect("detect_edition");
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    let pkt = source
        .read(manifest.chain.missions_pkt)
        .unwrap_or_else(|e| panic!("读 {}: {e}", manifest.chain.missions_pkt));
    let maps = list_parseable_maps_from_missions_pkt(manifest.chain.edition, &source, &pkt);
    eprintln!("missions_pkt={} parseable={}", manifest.chain.missions_pkt, maps.len());
    for m in maps.iter().take(8) {
        eprintln!("  {} · {} · modes={:?}", m.file_name, m.name_csf, m.game_modes);
    }
    assert!(!maps.is_empty(), "expected PKT MultiMaps rows");
    // 原版 RA2 MultiMaps 首行是 MP02T2，不是按文件名排序的 MP01T4。
    if manifest.chain.edition == GameEdition::Ra2 {
        assert_eq!(maps[0].file_name.to_ascii_lowercase(), "mp02t2.map");
    }

    let names = source.discover_skirmish_map_names();
    let scanned = list_parseable_maps_from_names(manifest.chain.edition, &source, &names);
    eprintln!("discover_scan parseable={}", scanned.len());
    assert!(scanned.len() > 4, "expected more than boot table, got {}", scanned.len());

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
