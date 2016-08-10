//! 自顶层 `protocol.rs`。

use ra_net::{MAX_PAYLOAD_BYTES, MatchFingerprint, PROTOCOL_VERSION};

#[test]
fn protocol_version_is_nonzero() {
    assert!(PROTOCOL_VERSION >= 1);
    assert!(MAX_PAYLOAD_BYTES >= 1024);
}

#[test]
fn fingerprint_stable_for_same_bytes() {
    let a = MatchFingerprint::build("ra2", "mp01t4.map", b"[General]\n");
    let b = MatchFingerprint::build("ra2", "mp01t4.map", b"[General]\n");
    assert_eq!(a, b);
    let c = MatchFingerprint::build("ra2", "mp01t4.map", b"[General]\nX=1\n");
    assert_ne!(a.rules_hash, c.rules_hash);
}
