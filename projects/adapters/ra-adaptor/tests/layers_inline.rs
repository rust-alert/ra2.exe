//! 集成测试：原 `src/layers.rs` 内联测试迁出。

use ra_adaptor::*;

#[test]
fn parse_plain_expand_names() {
    let e = parse_expansion_file_name("expand.mix").unwrap().unwrap();
    assert_eq!(e.index, 0);
    assert_eq!(e.family, ExpansionFamily::Plain);

    let e = parse_expansion_file_name("Expand01.MIX").unwrap().unwrap();
    assert_eq!(e.index, 1);
    assert_eq!(e.family, ExpansionFamily::Plain);
}

#[test]
fn parse_md_mo_and_reject_junk() {
    let e = parse_expansion_file_name("expandmd02.mix").unwrap().unwrap();
    assert_eq!((e.index, e.family), (2, ExpansionFamily::Md));
    let e = parse_expansion_file_name("expandmo99.mix").unwrap().unwrap();
    assert_eq!((e.index, e.family), (99, ExpansionFamily::Mo));
    assert!(parse_expansion_file_name("expandfoo.mix").is_err());
    assert!(parse_expansion_file_name("ra2.mix").unwrap().is_none());
}
