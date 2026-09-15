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
fn verses_tokens_preserve_targeting_flags() {
    let doc = IniDocument::parse(b"[AP]\nVerses=100%FRP,50%F,25%,100%,100%,75%,100%,100%,100%,100%,100%\n").unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    let ap = reg.get("ap").unwrap();
    assert_eq!(ap.verses[0].multiplier, 100);
    assert!(ap.verses[0].force_fire);
    assert!(ap.verses[0].retaliate);
    assert!(ap.verses[0].passive_acquire);
    assert!(ap.verses[0].allows_passive_acquire());
    assert_eq!(ap.verses[1].multiplier, 50);
    assert!(ap.verses[1].force_fire);
    assert!(!ap.verses[1].retaliate);
    assert!(!ap.verses[1].passive_acquire);
    assert!(ap.verses[1].allows_force_fire());
    assert!(!ap.verses[2].force_fire);
}

#[test]
fn zero_multiplier_without_p_blocks_passive_acquire() {
    let entry = ra_types::VersesEntry::from_multiplier(0);
    assert!(!entry.allows_passive_acquire());
    assert!(!entry.allows_retaliate());
    assert!(!entry.allows_force_fire());
    let with_p = ra_types::VersesEntry { multiplier: 0, force_fire: false, retaliate: false, passive_acquire: true };
    assert!(with_p.allows_passive_acquire());
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
    let policy = IniMergePolicy { default_entry: EntryMergePolicy::MergeSection };
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

#[test]
fn realistic_ap_section_with_extra_keys_and_percent_prone() {
    // 零售 rules 弹头节含大量未建模键；`ProneDamage` 常带 `%`。
    let doc = IniDocument::parse(
        b"[AP]\n\
Verses=100%,100%,100%,50%,25%,25%,25%,50%,25%,100%,100%\n\
AnimList=EXPLOSB\n\
InfDeath=3\n\
TitForTat=no\n\
Bullets=no\n\
ProneDamage=100%\n\
Spread=0\n",
    )
    .unwrap();
    let reg = WarheadRegistry::from_names(&doc, ["AP"]);
    let ap = reg.get("AP").expect("AP must load despite extra keys / percent suffix");
    assert_eq!(ap.verses[0], 100);
    assert_eq!(ap.verses[3], 50);
    assert_eq!(ap.prone_damage, 100);
    assert_eq!(ap.spread, 0);
}
