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
fn append_values_joins_layers_bottom_first() {
    let layers = docs(&[b"[MTNK]\nOwner=Americans\n", b"[MTNK]\nOwner=Alliance\n"]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::AppendValues,
    };
    let view = LayeredIniView::new(&layers, &policy);
    let sec = view.section("MTNK").unwrap();
    let all = sec.all_resolved("Owner");
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].layer, 0);
    assert_eq!(all[0].value.trimmed().raw, "Americans");
    assert_eq!(all[1].layer, 1);
    assert_eq!(all[1].value.trimmed().raw, "Alliance");
    assert_eq!(sec.effective_raw("Owner").unwrap().as_ref(), "Americans,Alliance");
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct OwnerFields {
    #[serde(rename = "Owner", default)]
    owner: ra_types::HouseAllowList,
}

#[test]
fn append_values_deserializes_joined_house_list() {
    let layers = docs(&[b"[MTNK]\nOwner=Americans\n", b"[MTNK]\nOwner=Alliance\n"]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::AppendValues,
    };
    let view = LayeredIniView::new(&layers, &policy);
    let fields: OwnerFields = view.section("MTNK").unwrap().deserialize().unwrap();
    assert!(fields.owner.owner_allows("Americans"));
    assert!(fields.owner.owner_allows("Alliance"));
    assert!(!fields.owner.owner_allows("Russians"));
}

#[test]
fn resolved_reports_top_layer_for_last_value() {
    let layers = docs(&[b"[General]\nRepairStep=8\n", b"[General]\nRepairStep=16\n"]);
    let policy = IniMergePolicy::last_wins();
    let view = LayeredIniView::new(&layers, &policy);
    let r = view.section("General").unwrap().resolved("RepairStep").unwrap();
    assert_eq!(r.layer, 1);
    assert_eq!(r.value.trimmed().raw, "16");
}

#[test]
fn section_keys_lists_unique_names_bottom_first() {
    let layers = docs(&[b"[A]\nx=1\n[B]\ny=2\n", b"[B]\ny=3\n[C]\nz=4\n"]);
    let policy = IniMergePolicy::last_wins();
    let view = LayeredIniView::new(&layers, &policy);
    assert_eq!(view.section_keys(), vec!["A", "B", "C"]);
}

#[test]
fn numbered_pack_concats_sorted_indexes() {
    let layers = docs(&[b"[IsoMapPack5]\n10=C\n2=B\n1=A\n"]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::NumberedPack,
    };
    let view = LayeredIniView::new(&layers, &policy);
    assert_eq!(view.numbered_pack_concat("IsoMapPack5").as_deref(), Some("ABC"));
}

#[test]
fn numbered_pack_top_layer_replaces_same_index() {
    let layers = docs(&[
        b"[IsoMapPack5]\n1=AA\n2=BB\n3=CC\n",
        b"[IsoMapPack5]\n2=XX\n",
    ]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::NumberedPack,
    };
    let view = LayeredIniView::new(&layers, &policy);
    let resolved = view.section("IsoMapPack5").unwrap().numbered_resolved();
    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0].0, 1);
    assert_eq!(resolved[0].1.layer, 0);
    assert_eq!(resolved[1].0, 2);
    assert_eq!(resolved[1].1.layer, 1);
    assert_eq!(resolved[1].1.value.raw, "XX");
    assert_eq!(view.numbered_pack_concat("IsoMapPack5").as_deref(), Some("AAXXCC"));
}

#[test]
fn numbered_pack_replace_section_keeps_only_top_indexes() {
    let layers = docs(&[
        b"[IsoMapPack5]\n1=AA\n2=BB\n3=CC\n",
        b"[IsoMapPack5]\n2=XX\n",
    ]);
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::ReplaceSection,
    };
    let view = LayeredIniView::new(&layers, &policy);
    assert_eq!(view.numbered_pack_concat("IsoMapPack5").as_deref(), Some("XX"));
}
