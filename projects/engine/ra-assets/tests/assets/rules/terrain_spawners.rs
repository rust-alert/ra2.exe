//! 地形产矿节解码。

use ra_assets::*;

#[test]
fn scan_animated_tiberium_spawners() {
    let doc = IniDocument::parse(
        b"[TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\nAnimationProbability=0.5\nAnimationRate=3\n\
[Other]\nSpawnsTiberium=yes\nIsAnimated=no\n",
    )
    .unwrap();
    let reg = terrain_spawners_from_rules(&doc);
    assert_eq!(reg.len(), 1);
    let s = reg.get("tibtre01").unwrap();
    assert_eq!(s.animation_rate_ticks, 3);
    assert_eq!(s.animation_probability_micros, 500_000);
}

#[test]
fn from_layered_merges_spawner_fields() {
    let base = IniDocument::parse(
        b"[TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\nAnimationProbability=0.1\nAnimationRate=5\n",
    )
    .unwrap();
    let top = IniDocument::parse(b"[TIBTRE01]\nAnimationProbability=0.5\nAnimationRate=2\n").unwrap();
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let docs = [base, top];
    let reg = terrain_spawners_from_layered(LayeredIniView::new(&docs, &policy));
    let s = reg.get("TIBTRE01").unwrap();
    assert_eq!(s.animation_probability_micros, 500_000);
    assert_eq!(s.animation_rate_ticks, 2);
}
