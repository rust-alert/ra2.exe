//! `[OverlayTypes]`：内部 id 按声明顺序，不按数字键留空洞。

use ra_assets::*;

#[test]
fn declaration_order_not_numeric_keys() {
    // 键乱序且有空洞时，id 仍是「第几个值」，不是键上的数字。
    let doc = IniDocument::parse(b"[OverlayTypes]\n3=GAWALL\n105=TIB01\n1=GASAND\n").unwrap();
    let reg = overlay_types_from_rules(&doc);
    assert_eq!(reg.len(), 3);
    assert_eq!(reg.name(0), Some("GAWALL"));
    assert_eq!(reg.name(1), Some("TIB01"));
    assert_eq!(reg.name(2), Some("GASAND"));
    assert_eq!(reg.name(3), None);
    assert_eq!(reg.name(105), None);
}

#[test]
fn missing_key_zero_shifts_gems_and_ore_like_retail() {
    // 零售常见：无 `0=`，且中段缺 `40=`/`41=` → 按序后 GEM01=27、TIB01=102。
    let mut body = String::from("[OverlayTypes]\n");
    let early = [
        "GASAND", "CYCL", "GAWALL", "BARB", "WOOD", "DUMMY", "DUMMY2", "DUMMY3", "DUMMY4", "DUMMY5", "DUMMY6", "DUMMY7", "DUMMY8", "DUMMY9",
        "DUMMY10", "DUMMY11", "DUMMY12", "V16", "V17", "V18", "DUMMY13", "DUMMY14", "FENC", "DUMMY15", "BRIDGE1", "BRIDGE2", "NAWALL",
    ];
    for (i, name) in early.iter().enumerate() {
        body.push_str(&format!("{}={name}\n", i + 1)); // keys 1..27，无 0
    }
    for i in 1..=12 {
        body.push_str(&format!("{}=GEM{i:02}\n", 27 + i)); // keys 28..39
    }
    // 跳过键 40、41；再填到按序下标 101，使下一档 TIB01 落在 102。
    for i in 0..63 {
        body.push_str(&format!("{}=FILL{i:03}\n", 42 + i));
    }
    for i in 1..=20 {
        body.push_str(&format!("{}=TIB{i:02}\n", 105 + i - 1));
    }
    let doc = IniDocument::parse(body.as_bytes()).unwrap();
    let reg = overlay_types_from_rules(&doc);
    assert_eq!(reg.name(0), Some("GASAND"));
    assert_eq!(reg.name(26), Some("NAWALL"));
    assert_eq!(reg.name(27), Some("GEM01"));
    assert_eq!(reg.name(102), Some("TIB01"));
}
