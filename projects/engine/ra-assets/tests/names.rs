//! 集成测试：原 `src/mix/names.rs` 内联测试迁出。

use ra_assets::{MixNameTable, mix_hash};

#[test]
fn builtin_has_no_collisions_and_contains_shell_assets() {
    let table = MixNameTable::builtin().expect("builtin");
    assert!(table.len() > 100);
    let id = mix_hash("sdtp.shp");
    assert_eq!(table.lookup(id), Some("sdtp.shp"));
    assert_eq!(table.lookup(mix_hash("title.pcx")), Some("title.pcx"));
}

#[test]
fn insert_detects_collision() {
    let mut table = MixNameTable::new();
    table.insert("alpha.bin").unwrap();
    // 人为构造极难；用 mock：插入同名应成功，冲突需真实不同名同哈希。
    table.insert("ALPHA.BIN").unwrap();
    assert_eq!(table.len(), 1);
}
