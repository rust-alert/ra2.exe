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
funds=9400
power_output=200
power_drain=50
low_power=false
queue=E1:12
last_reject=QueueFull
";
    let s = TestStatus::parse(text).unwrap();
    assert_eq!(s.tick, 12);
    assert_eq!(s.hash, 0xabc);
    assert_eq!(s.outcome, "victory:Americans");
    assert!(s.paused);
    assert_eq!(s.selected, vec![0, 2]);
    assert_eq!(s.entities, 2);
    assert_eq!(s.funds, 9400);
    assert_eq!(s.power_output, 200);
    assert_eq!(s.power_drain, 50);
    assert!(!s.low_power);
    assert_eq!(s.queue, "E1:12");
    assert_eq!(s.last_reject, "QueueFull");
    assert!(s.matches_expect("tick>=1"));
    assert!(s.matches_expect("funds>=1000"));
    assert!(s.matches_expect("queue!=none"));
    assert!(s.matches_expect("outcome!=none"));
    assert!(s.matches_expect("paused=true"));
}
