//! 资源层发现与稳定排序。

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use ra_adaptor::{
    ExpansionFamily, PRIORITY_BASE_GAME, PRIORITY_EXPANSION_BASE, ResourceChain, ResourceLayerKind, compose_resource_layers,
    discover_expansions, parse_expansion_file_name,
};
use ra_types::GameEdition;

fn scratch_dir(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("ra-adaptor-layers-{tag}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn touch(dir: &std::path::Path, name: &str) {
    fs::write(dir.join(name), b"x").unwrap();
}

fn ra2_chain() -> ResourceChain {
    ResourceChain::for_edition(GameEdition::Ra2)
}

#[test]
fn base_only_yields_base_portrait() {
    let dir = scratch_dir("base");
    touch(&dir, "language.mix");
    touch(&dir, "ra2.mix");
    touch(&dir, "multi.mix");
    touch(&dir, "theme.mix");
    touch(&dir, "maps01.mix");
    touch(&dir, "maps02.mix");

    let c = compose_resource_layers(&dir, &ra2_chain());
    assert!(c.diagnostics.detected_expansions.is_empty());
    assert_eq!(c.layers.len(), 1);
    assert_eq!(c.layers[0].kind, ResourceLayerKind::BaseGame);
    assert!(c.root_mount_plan.iter().all(|s| s.priority == PRIORITY_BASE_GAME));
    assert!(c.nested_mount_plan.iter().any(|n| n.name.eq_ignore_ascii_case("neutral.mix")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn expand01_raises_priority_above_base() {
    let dir = scratch_dir("e01");
    touch(&dir, "ra2.mix");
    touch(&dir, "language.mix");
    touch(&dir, "expand01.mix");

    let c = compose_resource_layers(&dir, &ra2_chain());
    assert_eq!(c.diagnostics.detected_expansions, vec!["expand01.mix".to_string()]);
    let expand = c.root_mount_plan.iter().find(|s| s.name.eq_ignore_ascii_case("expand01.mix")).expect("expand01 in plan");
    assert_eq!(expand.priority, PRIORITY_EXPANSION_BASE + 1);
    assert!(expand.priority > PRIORITY_BASE_GAME);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn multi_expand_stable_order_ignores_discovery_noise() {
    let dir = scratch_dir("multi");
    // 故意以乱序创建。
    touch(&dir, "expand03.mix");
    touch(&dir, "expand01.mix");
    touch(&dir, "expand02.mix");
    touch(&dir, "ra2.mix");

    let (a, _) = discover_expansions(&dir);
    let indexes: Vec<_> = a.iter().map(|e| e.index).collect();
    assert_eq!(indexes, vec![1, 2, 3]);

    let c = compose_resource_layers(&dir, &ra2_chain());
    let expand_prios: Vec<_> =
        c.root_mount_plan.iter().filter(|s| s.name.to_ascii_lowercase().starts_with("expand")).map(|s| s.priority).collect();
    assert_eq!(expand_prios, vec![PRIORITY_EXPANSION_BASE + 1, PRIORITY_EXPANSION_BASE + 2, PRIORITY_EXPANSION_BASE + 3]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn malformed_expand_name_is_diagnosed() {
    let dir = scratch_dir("bad");
    touch(&dir, "expandfoo.mix");
    touch(&dir, "ra2.mix");
    let c = compose_resource_layers(&dir, &ra2_chain());
    assert!(!c.diagnostics.malformed.is_empty());
    assert!(c.diagnostics.detected_expansions.is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn md_family_parses() {
    let e = parse_expansion_file_name("expandmd01.mix").unwrap().unwrap();
    assert_eq!(e.family, ExpansionFamily::Md);
    assert_eq!(e.index, 1);
}

#[test]
fn ra2_edition_skips_expandmd_on_combo_disk() {
    let dir = scratch_dir("ra2-skip-md");
    touch(&dir, "ra2.mix");
    touch(&dir, "language.mix");
    touch(&dir, "expand01.mix");
    touch(&dir, "expandmd01.mix");

    let c = compose_resource_layers(&dir, &ra2_chain());
    assert_eq!(c.diagnostics.detected_expansions, vec!["expand01.mix".to_string()]);
    assert!(c.root_mount_plan.iter().any(|s| s.name.eq_ignore_ascii_case("expand01.mix")));
    assert!(!c.root_mount_plan.iter().any(|s| s.name.to_ascii_lowercase().starts_with("expandmd")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn yr_edition_keeps_expandmd_on_combo_disk() {
    let dir = scratch_dir("yr-keep-md");
    touch(&dir, "ra2md.mix");
    touch(&dir, "langmd.mix");
    touch(&dir, "expandmd01.mix");
    touch(&dir, "expand01.mix");

    let c = compose_resource_layers(&dir, &ResourceChain::for_edition(GameEdition::Yr));
    let names: Vec<_> = c.diagnostics.detected_expansions.iter().map(|s| s.to_ascii_lowercase()).collect();
    assert!(names.iter().any(|n| n == "expandmd01.mix"));
    assert!(names.iter().any(|n| n == "expand01.mix"));
    let _ = fs::remove_dir_all(&dir);
}
