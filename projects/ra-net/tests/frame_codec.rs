use ra_net::{MatchFingerprint, PROTOCOL_VERSION, SessionMessage, StateDigest, decode_frame, encode_frame};

#[test]
fn roundtrip_hello_and_digest() {
    let msg =
        SessionMessage::Hello { protocol: PROTOCOL_VERSION, fingerprint: MatchFingerprint::build("ra2", "m.map", b"rules") };
    let frame = encode_frame(&msg).unwrap();
    let (decoded, n) = decode_frame(&frame).unwrap();
    assert_eq!(n, frame.len());
    assert_eq!(decoded, msg);

    let d = SessionMessage::Digest(StateDigest { tick: 9, hash: 0xabc });
    let frame = encode_frame(&d).unwrap();
    let (decoded, _) = decode_frame(&frame).unwrap();
    assert_eq!(decoded, d);
}

#[test]
fn rejects_truncated() {
    assert!(decode_frame(&[0, 0, 0, 8, 1]).is_err());
}
