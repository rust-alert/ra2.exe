//! 弹头 Verses 解析。

use ra_assets::*;

#[test]
fn parse_verses_percentages() {
    let doc = IniDocument::parse(b"[AP]\nVerses=100%,50%,25%,100%,100%,75%,100%,100%,100%,100%,100%\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    let ap = reg.get("ap").unwrap();
    assert_eq!(ap.verses[0], 100);
    assert_eq!(ap.verses[1], 50);
    assert_eq!(ap.verses[2], 25);
    assert_eq!(ap.verses[5], 75);
    assert_eq!(armor_index("heavy"), 5);
    assert_eq!(armor_index("unknown"), 0);
}

#[test]
fn from_names_layered_merges_verses_override() {
    let base = IniDocument::parse(b"[AP]\nVerses=100%,50%,25%,100%,100%,75%,100%,100%,100%,100%,100%\n").unwrap();
    let top = IniDocument::parse(b"[AP]\nVerses=10%,10%,10%,10%,10%,10%,10%,10%,10%,10%,10%\n").unwrap();
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let docs = [base, top];
    let reg = WarheadRegistry::from_names_layered(LayeredIniView::new(&docs, &policy), ["AP"]);
    assert_eq!(reg.get("AP").unwrap().verses[0], 10);
    assert_eq!(reg.get("AP").unwrap().verses[5], 10);
}
