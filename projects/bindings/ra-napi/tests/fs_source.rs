//! 集成测试：原 `src/fs_source.rs` 内联测试迁出。

use ra_assets::mix_hash;
use ra_components::fs_source::*;
use ra_types::AssetSource;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn old_mix(id: i32, body: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(&id.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(body.len() as u32).to_le_bytes());
    data.extend_from_slice(body);
    data
}

fn scratch(tag: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("ra-napi-fs-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn loose_overrides_mix_and_explain_matches_read() {
    let dir = scratch("loose");
    let mix_bytes = old_mix(mix_hash("rules.ini"), b"FROM-MIX");
    std::fs::write(dir.join("base.mix"), &mix_bytes).unwrap();
    std::fs::write(dir.join("rules.ini"), b"FROM-LOOSE").unwrap();

    let mut src = GameAssetSource::new(dir.clone());
    src.vfs.mount_bytes_with_meta("base.mix", mix_bytes, 0, None, Some("base".into())).unwrap();

    let hit = src.resolve("rules.ini").unwrap();
    assert!(matches!(hit.origin, AssetOrigin::Loose { .. }));
    assert_eq!(hit.bytes, b"FROM-LOOSE");
    assert_eq!(src.read("rules.ini").unwrap(), hit.bytes);
    assert!(hit.explain().starts_with("loose:"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nested_expand_leaf_overlays_base_leaf() {
    let nested_base = old_mix(mix_hash("leaf.bin"), b"BASE-LEAF");
    let nested_exp = old_mix(mix_hash("leaf.bin"), b"EXP-LEAF");
    let outer_base = old_mix(mix_hash("cache.mix"), &nested_base);
    let outer_exp = old_mix(mix_hash("cache.mix"), &nested_exp);

    let dir = scratch("nested");
    let mut src = GameAssetSource::new(dir.clone());
    src.vfs.mount_bytes_with_meta("base.mix", outer_base, 0, None, Some("base".into())).unwrap();
    src.vfs.mount_bytes_with_meta("expand01.mix", outer_exp, 101, None, Some("expansion.plain.01".into())).unwrap();
    assert_eq!(src.mount_nested_names(&["cache.mix"]), (2, 0));

    let hit = src.resolve("leaf.bin").unwrap();
    assert_eq!(hit.bytes, b"EXP-LEAF");
    match hit.origin {
        AssetOrigin::Mix { parent: Some(p), priority, .. } => {
            assert_eq!(p, "expand01.mix");
            assert_eq!(priority, 101);
        }
        other => panic!("expected mix hit, got {other:?}"),
    }
    assert_eq!(src.read("leaf.bin").unwrap(), b"EXP-LEAF");

    let _ = std::fs::remove_dir_all(&dir);
}
