//! `IniDocument`（Westwood 方言）行为。

use ra_assets::{IniDocument, collect_shp_refs, numbered_pairs, numbered_section_concat};

#[test]
fn numbered_concat_sorts_numerically() {
    let doc = IniDocument::parse(b"[IsoMapPack5]\n10=C\n2=B\n1=A\n").unwrap();
    assert_eq!(numbered_section_concat(&doc, "IsoMapPack5").as_deref(), Some("ABC"));
}

#[test]
fn numbered_pairs_sorts_and_skips_named_keys() {
    let doc = IniDocument::parse(b"[TF]\nName=Alpha\n2=1,GI\n0=2,E1\nGroup=-1\n").unwrap();
    let sec = doc.section("TF").unwrap();
    assert_eq!(numbered_pairs(sec), vec![(0, "2,E1"), (2, "1,GI")]);
}

#[test]
fn lookup_is_case_insensitive_and_last_wins() {
    let doc = IniDocument::parse(b"[mtnk]\nStrength=1\n[Duplicate]\nKey=first\nKey=second\n").unwrap();
    assert_eq!(doc.get("MTNK", "strength"), Some("1"));
    assert_eq!(doc.get("Duplicate", "Key"), Some("second"));
    let sec = doc.section("Duplicate").unwrap();
    let values: Vec<_> = sec.pairs().map(|(_, v)| v).collect();
    assert_eq!(values, ["first", "second"]);
}

#[test]
fn preserves_list_values_with_commas() {
    let doc = IniDocument::parse(b"[MTNK]\nOwner=Britishs,Americans\n").unwrap();
    assert_eq!(doc.get("MTNK", "Owner"), Some("Britishs,Americans"));
}

#[test]
fn collect_shp_refs_dedupes_and_keeps_order() {
    let doc = IniDocument::parse(b"[A]\nBg=Menu.shp\nBtn=ok.shp, Menu.shp\nOther=readme.txt\n[B]\nX=\"Hover.SHP\"\n").unwrap();
    assert_eq!(collect_shp_refs(&doc), vec!["Menu.shp", "ok.shp", "Hover.SHP"]);
}

#[test]
fn mirage_warhead_section_with_trailing_slash_slash() {
    let doc = IniDocument::parse(b"[MirageWH]    // Supposed to be a heat ray.\nVerses=100%,100%,80%\n").unwrap();
    assert_eq!(doc.get("MirageWH", "Verses"), Some("100%,100%,80%"));
}

#[test]
fn accepts_windows_1252_ellipsis_in_value() {
    let doc = IniDocument::parse(b"[VOX]\nText=control\x85standby.\n").unwrap();
    assert_eq!(doc.get("VOX", "Text"), Some("control\u{2026}standby."));
}
