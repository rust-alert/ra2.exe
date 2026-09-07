//! `MixVfs` 优先级覆盖语义。

use ra_assets::{MixArchive, MixVfs};

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

fn rules_id() -> i32 {
    // 与 `mix_hash("rules.ini")` 一致：由档案按名查找验证覆盖即可。
    // 这里构造两个同 id 的条目，分别表示 base / expand 内容。
    use ra_assets::mix_hash;
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
fn same_priority_later_mount_wins() {
    let id = rules_id();
    let mut vfs = MixVfs::new();
    vfs.mount_bytes("a.mix", old_mix(id, b"AAAA")).unwrap();
    vfs.mount_bytes("b.mix", old_mix(id, b"BBBB")).unwrap();
    assert_eq!(vfs.read("rules.ini").unwrap(), b"BBBB");
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
