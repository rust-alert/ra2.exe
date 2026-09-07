//! 旧 `IniDocument` 与 `parse_with_oak` 查找语义对照。

use ra_assets::{IniDocument, SourceId, parse_with_oak};

const SAMPLE: &[u8] = br#"; comment
[VehicleTypes]
0=MTNK
1=HTNK

[MTNK]
Strength=400
Speed=64
Armor=heavy
Cost=800
Sight=6

[Duplicate]
Key=first
Key=second
"#;

#[test]
fn oak_lookup_matches_legacy_last_wins() {
    let legacy = IniDocument::parse(SAMPLE).unwrap();
    let rich = parse_with_oak(SAMPLE, SourceId(1)).unwrap();

    assert_eq!(legacy.get("MTNK", "Strength"), rich.get("MTNK", "Strength"));
    assert_eq!(legacy.get("MTNK", "Armor"), rich.get("MTNK", "Armor"));
    assert_eq!(legacy.get("VehicleTypes", "0"), rich.get("VehicleTypes", "0"));
    assert_eq!(legacy.get("Duplicate", "Key"), rich.get("Duplicate", "Key"));
    assert_eq!(rich.get("Duplicate", "Key"), Some("second"));
}

#[test]
fn oak_preserves_duplicate_entries() {
    let rich = parse_with_oak(SAMPLE, SourceId(0)).unwrap();
    let sec = rich.section_by_key("Duplicate").unwrap();
    let keys: Vec<_> = sec.entries.iter().map(|e| e.value_raw.as_str()).collect();
    assert_eq!(keys, ["first", "second"]);
}

#[test]
fn oak_section_compare_key_is_case_insensitive() {
    let rich = parse_with_oak(b"[mtnk]\nStrength=1\n", SourceId(0)).unwrap();
    assert_eq!(rich.get("MTNK", "strength"), Some("1"));
}
