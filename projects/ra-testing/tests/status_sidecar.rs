use std::time::Duration;

use ra_testing::TestStatus;
use ra_types::EntityId;

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
difficulty=Hard
";
    let s = TestStatus::parse(text).unwrap();
    assert_eq!(s.tick, 12);
    assert_eq!(s.hash, 0xabc);
    assert_eq!(s.outcome, "victory:Americans");
    assert!(s.paused);
    assert_eq!(s.selected, vec![EntityId(0), EntityId(2)]);
    assert_eq!(s.entities, 2);
    assert_eq!(s.funds, 9400);
    assert_eq!(s.power_output, 200);
    assert_eq!(s.power_drain, 50);
    assert!(!s.low_power);
    assert_eq!(s.queue, "E1:12");
    assert_eq!(s.last_reject, "QueueFull");
    assert_eq!(s.difficulty, "Hard");
    assert!(s.matches_expect("tick>=1"));
    assert!(s.matches_expect("funds>=1000"));
    assert!(s.matches_expect("queue!=none"));
    assert!(s.matches_expect("outcome!=none"));
    assert!(s.matches_expect("paused=true"));
    assert!(s.matches_expect("difficulty=Hard"));
    assert!(!s.matches_expect("difficulty=Easy"));
}

#[test]
fn wait_until_reads_file_when_expect_matches() {
    let dir = std::env::temp_dir().join(format!(
        "ra2-status-wait-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("status.txt");
    std::fs::write(&path, "tick=3\nhash=0x1\noutcome=none\n").unwrap();
    let status = TestStatus::wait_until(&path, "tick>=2", Duration::from_secs(2)).expect("wait");
    assert!(status.tick >= 2);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn wait_until_times_out_when_expect_never_matches() {
    let dir = std::env::temp_dir().join(format!(
        "ra2-status-wait-miss-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("status.txt");
    std::fs::write(&path, "tick=0\nhash=0x0\noutcome=none\n").unwrap();
    let err = TestStatus::wait_until(&path, "tick>=99", Duration::from_millis(120)).unwrap_err();
    assert!(err.contains("超时"), "{err}");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}
