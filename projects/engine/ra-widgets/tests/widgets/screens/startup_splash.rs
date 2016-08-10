//! 集成测试：原 `src/startup_splash.rs` 内联测试迁出。

use ra_renderer::RgbaImage;
use ra_types::GameEdition;
use ra_widgets::startup_splash::*;
use std::time::{Duration, Instant};

#[test]
fn exact_640_selects_small_every_other_width_selects_large() {
    assert_eq!(splash_shp_for_width(640), SMALL_SPLASH_SHP);
    assert_eq!(splash_shp_for_width(639), LARGE_SPLASH_SHP);
    assert_eq!(splash_shp_for_width(800), LARGE_SPLASH_SHP);
    assert_eq!(splash_shp_for_width(1920), LARGE_SPLASH_SHP);
}

#[test]
fn yr_and_mo3_prefer_md_splash_ra2_does_not() {
    assert!(prefer_md_splash(GameEdition::Yr));
    assert!(prefer_md_splash(GameEdition::Mo3));
    assert!(!prefer_md_splash(GameEdition::Ra2));
}

#[test]
fn md_preference_reorders_splash_candidates() {
    let ra2_first = splash_candidates(800, false);
    assert_eq!(ra2_first[0], (LARGE_SPLASH_SHP, SPLASH_PALETTE));
    assert_eq!(ra2_first[1], (LARGE_SPLASH_SHP_MD, SPLASH_PALETTE_MD));

    let md_first = splash_candidates(800, true);
    assert_eq!(md_first[0], (LARGE_SPLASH_SHP_MD, SPLASH_PALETTE_MD));
    assert_eq!(md_first[1], (LARGE_SPLASH_SHP, SPLASH_PALETTE));

    assert_eq!(splash_names_for_width(640, true), (SMALL_SPLASH_SHP_MD, SPLASH_PALETTE_MD));
}

#[test]
fn nearest_fill_covers_destination() {
    let mut dst = opaque_black(4, 2).unwrap();
    let src = RgbaImage::from_raw(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255]).unwrap();
    blit_nearest_fill(&mut dst, &src);
    assert_eq!(&dst.as_raw()[0..4], &[10, 20, 30, 255]);
    assert_eq!(&dst.as_raw()[4..8], &[10, 20, 30, 255]);
    assert_eq!(&dst.as_raw()[8..12], &[40, 50, 60, 255]);
    assert_eq!(&dst.as_raw()[12..16], &[40, 50, 60, 255]);
}

#[test]
fn hold_anchors_at_first_present_and_never_rearms() {
    let start = Instant::now();
    let minimum = Duration::from_secs_f64(DEFAULT_MINIMUM_VISIBLE_SECS);
    let mut hold = VisibleHold::new(minimum);
    assert!(hold.is_active(start + Duration::from_secs(600)));

    hold.mark_presented(start);
    hold.mark_presented(start + Duration::from_secs(2));
    hold.mark_presented(start + minimum);

    assert!(hold.is_active(start));
    assert!(hold.is_active(start + minimum - Duration::from_millis(1)));
    assert!(!hold.is_active(start + minimum));
    assert!(!hold.is_active(start + minimum + Duration::from_secs(1)));
}

#[test]
fn hold_respects_custom_minimum() {
    let start = Instant::now();
    let mut hold = VisibleHold::new(Duration::from_secs(1));
    hold.mark_presented(start);
    assert!(hold.is_active(start + Duration::from_millis(999)));
    assert!(!hold.is_active(start + Duration::from_secs(1)));
}

#[test]
fn missing_csf_uses_english_fallback() {
    assert_eq!(csf_text(None, COPYRIGHT_KEY, COPYRIGHT_FALLBACK), COPYRIGHT_FALLBACK);
}
