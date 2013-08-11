//! 弹头 Verses 解析。

use ra_assets::{IniDocument, WarheadRegistry, armor_index};

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
