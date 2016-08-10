//! `MixVfs` 优先级覆盖语义。

use ra_assets::{MixArchive, MixVfs, mix_hash};

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

/// 外层 MIX：内含一个名为 `cache.mix` 的嵌套档案字节。
fn outer_with_nested_cache(nested_body: &[u8]) -> Vec<u8> {
    let nested = old_mix(mix_hash("leaf.bin"), nested_body);
    let cache_id = mix_hash("cache.mix");
    old_mix(cache_id, &nested)
}

fn rules_id() -> i32 {
    mix_hash("rules.ini")
}

#[test]
fn higher_priority_overrides_lower() {
    let id = rules_id();
    let mut vfs = MixVfs::new();
    vfs.mount_bytes_with_priority("base.mix", old_mix(id, b"BASE"), 0).unwrap();
    vfs.mount_bytes_with_priority("expand01.mix", old_mix(id, b"EXP1"), 101).unwrap();

    let (src, bytes) = vfs.resolve("rules.ini").unwrap();
    assert_eq!(src, "expand01.mix");
    assert_eq!(bytes, b"EXP1");
    assert_eq!(vfs.read("rules.ini").unwrap(), b"EXP1");
}

#[test]
fn same_priority_later_nested_archive_wins_leaf() {
    let leaf = mix_hash("ls800ustates.shp");
    let mut vfs = MixVfs::new();
    vfs.mount_bytes_with_priority("load.mix", old_mix(leaf, b"RA2-SHP"), 0).unwrap();
    vfs.mount_bytes_with_priority("loadmd.mix", old_mix(leaf, b"YR-SHP"), 0).unwrap();
    let hit = vfs.resolve_hit("ls800ustates.shp").unwrap();
    assert_eq!(hit.archive_name, "loadmd.mix");
    assert_eq!(hit.bytes, b"YR-SHP");
}

#[test]
fn missing_in_high_falls_back_to_low() {
    let id = rules_id();
    let other = rules_id().wrapping_add(1);
    let mut vfs = MixVfs::new();
    vfs.mount_bytes_with_priority("base.mix", old_mix(id, b"BASE"), 0).unwrap();
    // 高优先级档案不含 rules.ini
    vfs.mount_bytes_with_priority("expand01.mix", old_mix(other, b"OTHER"), 101).unwrap();
    assert_eq!(vfs.read("rules.ini").unwrap(), b"BASE");
}

#[test]
fn parse_helper_roundtrip() {
    let _ = MixArchive::parse(old_mix(1, b"AAAA")).unwrap();
}

#[test]
fn nested_inherits_parent_priority_and_layer() {
    let mut vfs = MixVfs::new();
    vfs.mount_bytes_with_meta("expand01.mix", outer_with_nested_cache(b"FROM-EXPAND"), 101, None, Some("expansion.plain.01".into())).unwrap();
    assert_eq!(vfs.mount_nested_all_from_parents("cache.mix").unwrap(), 1);

    let hit = vfs.resolve_hit("leaf.bin").unwrap();
    assert_eq!(hit.bytes, b"FROM-EXPAND");
    assert_eq!(hit.archive_name, "cache.mix");
    assert_eq!(hit.parent, Some("expand01.mix"));
    assert_eq!(hit.layer_id, Some("expansion.plain.01"));
    assert_eq!(hit.priority, 101);
}

#[test]
fn nested_from_all_parents_keeps_file_level_overlay() {
    let mut vfs = MixVfs::new();
    vfs.mount_bytes_with_priority("base.mix", outer_with_nested_cache(b"BASE-LEAF"), 0).unwrap();
    vfs.mount_bytes_with_priority("expand01.mix", outer_with_nested_cache(b"EXP-LEAF"), 101).unwrap();

    assert_eq!(vfs.mount_nested_all_from_parents("cache.mix").unwrap(), 2);
    assert_eq!(vfs.read("leaf.bin").unwrap(), b"EXP-LEAF");

    let hit = vfs.resolve_hit("leaf.bin").unwrap();
    assert_eq!(hit.parent, Some("expand01.mix"));
    assert_eq!(hit.priority, 101);
}

#[test]
fn preferring_missing_archive_does_not_steal_other_side_chrome() {
    let side1 = mix_hash("side1.shp");
    let mut vfs = MixVfs::new();
    // 模拟对局侧栏：盟军 / 苏军各有一份同名 SHP；后挂的苏军包在全局解析中胜出。
    vfs.mount_bytes_with_priority("sidec01.mix", old_mix(side1, b"ALLIED"), 0).unwrap();
    vfs.mount_bytes_with_priority("sidec02.mix", old_mix(side1, b"SOVIET"), 0).unwrap();

    assert_eq!(vfs.resolve_hit("side1.shp").unwrap().bytes, b"SOVIET");

    // 候选先试未挂载的 md 包：不得静默落到苏军档。
    assert!(vfs.resolve_hit_preferring("side1.shp", "sidec01md.mix").is_none());

    let allied = vfs.resolve_hit_preferring("side1.shp", "sidec01.mix").unwrap();
    assert_eq!(allied.archive_name, "sidec01.mix");
    assert_eq!(allied.bytes, b"ALLIED");

    let soviet = vfs.resolve_hit_preferring("side1.shp", "sidec02.mix").unwrap();
    assert_eq!(soviet.archive_name, "sidec02.mix");
    assert_eq!(soviet.bytes, b"SOVIET");
}

#[test]
fn preferring_empty_archive_falls_back_to_global() {
    let side1 = mix_hash("side1.shp");
    let mut vfs = MixVfs::new();
    vfs.mount_bytes_with_priority("sidec01.mix", old_mix(side1, b"ALLIED"), 0).unwrap();
    vfs.mount_bytes_with_priority("sidec02.mix", old_mix(side1, b"SOVIET"), 0).unwrap();
    let hit = vfs.resolve_hit_preferring("side1.shp", "").unwrap();
    assert_eq!(hit.bytes, b"SOVIET");
}
