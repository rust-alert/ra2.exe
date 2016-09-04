//! 弹头 Verses 一次解码为 `WarheadVerses`。

use ra_assets::*;
use ra_types::WarheadVerses;

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
fn missing_verses_defaults_all_full() {
    let doc = IniDocument::parse(b"[AP]\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    assert_eq!(reg.get("AP").unwrap().verses, WarheadVerses::all_full());
}

#[test]
fn empty_verses_defaults_all_full() {
    let doc = IniDocument::parse(b"[AP]\nVerses=\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    assert_eq!(reg.get("AP").unwrap().verses, WarheadVerses::all_full());
}

#[test]
fn illegal_token_keeps_slot_default() {
    let doc = IniDocument::parse(b"[AP]\nVerses=100%,bogus,25%\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    let ap = reg.get("AP").unwrap();
    assert_eq!(ap.verses[0], 100);
    assert_eq!(ap.verses[1], 100);
    assert_eq!(ap.verses[2], 25);
}

#[test]
fn short_list_pads_remaining_slots() {
    let doc = IniDocument::parse(b"[AP]\nVerses=10%,20%\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    let ap = reg.get("AP").unwrap();
    assert_eq!(ap.verses[0], 10);
    assert_eq!(ap.verses[1], 20);
    assert_eq!(ap.verses[10], 100);
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

#[test]
fn parse_spread_and_prone_damage_once() {
    let doc = IniDocument::parse(b"[HE]\nSpread=2\nProneDamage=50\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["HE"]);
    let he = reg.get("HE").unwrap();
    assert_eq!(he.spread, 2);
    assert_eq!(he.prone_damage, 50);
    assert_eq!(he.verses, WarheadVerses::all_full());
}

#[test]
fn missing_spread_and_prone_use_defaults() {
    let doc = IniDocument::parse(b"[AP]\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    let ap = reg.get("AP").unwrap();
    assert_eq!(ap.spread, 0);
    assert_eq!(ap.prone_damage, 100);
}
