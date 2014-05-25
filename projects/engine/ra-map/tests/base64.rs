use ra_map::{base64_decode, base64_encode};

#[test]
fn test_empty() {
    assert_eq!(base64_decode("").unwrap(), Vec::<u8>::new());
}

#[test]
fn test_hello() {
    // "Hello" = SGVsbG8=
    assert_eq!(base64_decode("SGVsbG8=").unwrap(), b"Hello".to_vec());
}

#[test]
fn test_no_padding() {
    // "Man" = TWFu (no padding needed, length divisible by 3)
    assert_eq!(base64_decode("TWFu").unwrap(), b"Man".to_vec());
}

#[test]
fn test_two_pad() {
    // "M" = TQ==
    assert_eq!(base64_decode("TQ==").unwrap(), b"M".to_vec());
}

#[test]
fn test_whitespace_handling() {
    // Simulates how .map files split base64 across lines.
    let input = "SGVs\n  bG8=\n";
    assert_eq!(base64_decode(input).unwrap(), b"Hello".to_vec());
}

#[test]
fn test_invalid_char() {
    assert!(base64_decode("SGVs!G8=").is_err());
}

#[test]
fn test_binary_roundtrip() {
    // Verify decoding of known binary data.
    // [0x00, 0xFF, 0x80] = AP+A
    let result: Vec<u8> = base64_decode("AP+A").unwrap();
    assert_eq!(result, vec![0x00, 0xFF, 0x80]);
}

#[test]
fn test_encode_roundtrip() {
    let raw = b"Hello, ra-map";
    let encoded = base64_encode(raw);
    assert_eq!(base64_decode(&encoded).unwrap(), raw);
}
