use ra_testing::TestStatus;

#[test]
fn parse_sidecar_roundtrip_shape() {
    let text = "\
tick=12
hash=0xabc
outcome=victory:Americans
paused=true
selected=0,2
entities=2
";
    let s = TestStatus::parse(text).unwrap();
    assert_eq!(s.tick, 12);
    assert_eq!(s.hash, 0xabc);
    assert_eq!(s.outcome, "victory:Americans");
    assert!(s.paused);
    assert_eq!(s.selected, vec![0, 2]);
    assert_eq!(s.entities, 2);
    assert!(s.matches_expect("tick>=1"));
    assert!(s.matches_expect("outcome!=none"));
    assert!(s.matches_expect("paused=true"));
}
