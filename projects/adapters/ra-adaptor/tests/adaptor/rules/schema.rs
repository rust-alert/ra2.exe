//! adaptor techno 字段合并 schema。

use ra_adaptor::techno_section_field_overrides;
use ra_assets::*;

#[test]
fn techno_schema_appends_owner_and_prereq_across_layers() {
    let base = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nOwner=Americans\nCost=700\nPrerequisite=GAWEAP\n",
    )
    .unwrap();
    let top = IniDocument::parse(
        b"[MTNK]\nOwner=Alliance\nCost=800\nPrerequisite=POWER\n",
    )
    .unwrap();
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let overrides = techno_section_field_overrides();
    let docs = [base, top];
    let reg = TechnoTypeRegistry::from_layered_with_overrides(LayeredIniView::new(&docs, &policy), Some(&overrides));
    let m = reg.get("MTNK").unwrap();
    assert!(m.owner.owner_allows("Americans"));
    assert!(m.owner.owner_allows("Alliance"));
    assert_eq!(m.cost, 800);
    assert_eq!(
        m.prerequisite.iter().cloned().collect::<Vec<_>>(),
        vec![
            ra_types::PrerequisiteToken::UnboundType("GAWEAP".into()),
            ra_types::PrerequisiteToken::Group(ra_types::PrerequisiteGroupKind::Power),
        ]
    );
}
