//! `DisplayMode` 解析与尺寸。

use ra_types::DisplayMode;

#[test]
fn default_is_1024x768() {
    assert_eq!(DisplayMode::DEFAULT, DisplayMode::W1024H768);
    assert_eq!(DisplayMode::default(), DisplayMode::W1024H768);
    assert_eq!(DisplayMode::DEFAULT.size(), (1024, 768));
}

#[test]
fn parse_accepts_common_aliases() {
    assert_eq!(DisplayMode::parse("800x600").unwrap(), DisplayMode::W800H600);
    assert_eq!(DisplayMode::parse("800X600").unwrap(), DisplayMode::W800H600);
    assert_eq!(DisplayMode::parse("1024").unwrap(), DisplayMode::W1024H768);
    assert_eq!(DisplayMode::parse("640×480").unwrap(), DisplayMode::W640H480);
}

#[test]
fn parse_rejects_free_resolution() {
    assert!(DisplayMode::parse("1920x1080").is_err());
    assert!(DisplayMode::parse("1280x720").is_err());
}

#[test]
fn cycle_next_wraps_all_modes() {
    assert_eq!(DisplayMode::W640H480.cycle_next(), DisplayMode::W800H600);
    assert_eq!(DisplayMode::W800H600.cycle_next(), DisplayMode::W1024H768);
    assert_eq!(DisplayMode::W1024H768.cycle_next(), DisplayMode::W640H480);
}

#[test]
fn all_modes_have_unique_labels() {
    let labels: Vec<_> = DisplayMode::ALL.iter().map(|m| m.as_str()).collect();
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(labels.len(), sorted.len());
}
