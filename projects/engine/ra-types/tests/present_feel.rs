//! `PresentFeel` serde / 默认值。

use ra_types::{PresentFeel, PresentMode, PresentQuantize};

#[test]
fn present_feel_toml_roundtrip() {
    let raw = r#"
mode = "16bit"
quantize = "rgb565"
gamma = 1.25
highlight_roll_off = 0.1
dither = false
"#;
    let feel: PresentFeel = toml_edit::de::from_str(raw).unwrap();
    assert_eq!(feel.mode, PresentMode::Bit16);
    assert_eq!(feel.quantize, PresentQuantize::Rgb565);
    assert!((feel.gamma - 1.25).abs() < 1e-6);
    assert!((feel.highlight_roll_off - 0.1).abs() < 1e-6);
    assert!(!feel.dither);

    let doc = toml_edit::ser::to_document(&feel).unwrap();
    let again: PresentFeel = toml_edit::de::from_document(doc).unwrap();
    assert_eq!(again, feel);
}

#[test]
fn present_mode_aliases_via_serde() {
    let off: PresentFeel = toml_edit::de::from_str(r#"mode = "truecolor""#).unwrap();
    assert_eq!(off.mode, PresentMode::Off);
    let on: PresentFeel = toml_edit::de::from_str(r#"mode = "bit16""#).unwrap();
    assert_eq!(on.mode, PresentMode::Bit16);
}

#[test]
fn present_feel_default_is_16bit() {
    let f = PresentFeel::DEFAULT;
    assert!(f.is_active());
    assert_eq!(f.mode, PresentMode::Bit16);
    assert!((f.gamma - 1.2).abs() < 1e-6);
    assert!(f.dither);
}

#[test]
fn present_feel_sanitize_clamps() {
    let f = PresentFeel {
        mode: PresentMode::Bit16,
        quantize: PresentQuantize::Rgb565,
        gamma: 9.0,
        highlight_roll_off: 2.0,
        dither: true,
    }
    .sanitized();
    assert!((f.gamma - 3.0).abs() < 1e-6);
    assert!((f.highlight_roll_off - 1.0).abs() < 1e-6);
}
