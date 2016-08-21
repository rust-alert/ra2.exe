//! 层叠 INI 合并视图。

use ra_assets::*;

fn docs(layers: &[&[u8]]) -> Vec<IniDocument> {
    layers.iter().map(|b| IniDocument::parse(b).expect("ini")).collect()
}

#[test]
fn last_value_prefers_top_layer() {
    let layers = docs(&[b"[General]\nRepairStep=8\n", b"[General]\nRepairStep=16\n"]);
    let policy = IniMergePolicy::last_wins();
    let view = LayeredIniView::new(&layers, &policy);
    let v = view.get("General", "RepairStep").unwrap();
    assert_eq!(v.trimmed().raw, "16");
    assert_eq!(v.section, "General");
}

#[test]
fn first_value_keeps_bottom_layer() {
    let layers = docs(&[b"[General]\nRepairStep=8\n", b"[General]\nRepairStep=16\n"]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::FirstValue,
    };
    let view = LayeredIniView::new(&layers, &policy);
    assert_eq!(view.get("General", "RepairStep").unwrap().trimmed().raw, "8");
}

#[test]
fn merge_section_keeps_lower_keys_absent_on_top() {
    let layers = docs(&[
        b"[General]\nRepairStep=8\nRepairPercent=15\n",
        b"[General]\nRepairStep=16\n",
    ]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let view = LayeredIniView::new(&layers, &policy);
    let sec = view.section("General").unwrap();
    assert_eq!(sec.get("RepairStep").unwrap().trimmed().raw, "16");
    assert_eq!(sec.get("RepairPercent").unwrap().trimmed().raw, "15");
}

#[test]
fn replace_section_drops_lower_only_keys() {
    let layers = docs(&[
        b"[General]\nRepairStep=8\nRepairPercent=15\n",
        b"[General]\nRepairStep=16\n",
    ]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::ReplaceSection,
    };
    let view = LayeredIniView::new(&layers, &policy);
    let sec = view.section("General").unwrap();
    assert_eq!(sec.get("RepairStep").unwrap().trimmed().raw, "16");
    assert!(sec.get("RepairPercent").is_none());
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct GeneralFields {
    #[serde(rename = "RepairStep")]
    repair_step: Option<i32>,
    #[serde(rename = "RepairPercent")]
    repair_percent: Option<i32>,
}

#[test]
fn layered_section_deserializes_merged_fields() {
    let layers = docs(&[
        b"[General]\nRepairStep=8\nRepairPercent=15\n",
        b"[General]\nRepairStep=16\n",
    ]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let view = LayeredIniView::new(&layers, &policy);
    let sec = view.section("General").unwrap();
    let g: GeneralFields = from_layered_section(&sec).unwrap();
    assert_eq!(g.repair_step, Some(16));
    assert_eq!(g.repair_percent, Some(15));
}

#[test]
fn layered_section_deserialize_honors_replace_section() {
    let layers = docs(&[
        b"[General]\nRepairStep=8\nRepairPercent=15\n",
        b"[General]\nRepairStep=16\n",
    ]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::ReplaceSection,
    };
    let view = LayeredIniView::new(&layers, &policy);
    let g: GeneralFields = view.section("General").unwrap().deserialize().unwrap();
    assert_eq!(g.repair_step, Some(16));
    assert_eq!(g.repair_percent, None);
}

#[test]
fn section_keys_lists_unique_names_bottom_first() {
    let layers = docs(&[b"[A]\nx=1\n[B]\ny=2\n", b"[B]\ny=3\n[C]\nz=4\n"]);
    let policy = IniMergePolicy::last_wins();
    let view = LayeredIniView::new(&layers, &policy);
    assert_eq!(view.section_keys(), vec!["A", "B", "C"]);
}
