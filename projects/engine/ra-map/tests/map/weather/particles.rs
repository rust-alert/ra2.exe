//! 自 `engine/ra-map/src/weather_particles.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-map/src/weather_particles.rs :: tests
use image::RgbaImage;
use ra_map::{Theater, weather_particles::*};

#[test]
fn snow_theater_spawns_active_field() {
    let field = WeatherParticleField::for_theater(Theater::Snow, 320, 240);
    assert_eq!(field.kind(), WeatherKind::Snow);
    assert!(field.is_active());
    assert!(field.particles.len() >= 48);
}

#[test]
fn temperate_theater_has_no_weather() {
    let field = WeatherParticleField::for_theater(Theater::Temperate, 320, 240);
    assert!(!field.is_active());
}

#[test]
fn snow_paint_marks_dark_canvas() {
    let mut field = WeatherParticleField::new(WeatherKind::Snow, 80, 60, 42);
    let mut img = RgbaImage::from_pixel(80, 60, image::Rgba([10, 10, 20, 255]));
    field.tick(500);
    field.paint_onto(&mut img);
    let bright = img.pixels().filter(|p| p.0[0] > 40 || p.0[2] > 40).count();
    assert!(bright > 20, "expected snow highlights, bright={bright}");
}

#[test]
fn fog_paint_softens_canvas() {
    let field = WeatherParticleField::new(WeatherKind::Fog, 120, 90, 7);
    let mut img = RgbaImage::from_pixel(120, 90, image::Rgba([20, 30, 40, 255]));
    field.paint_onto(&mut img);
    let lifted = img.pixels().filter(|p| p.0[0] > 25).count();
    assert!(lifted > 50, "expected fog lift, lifted={lifted}");
}

#[test]
fn tick_moves_snow_downward() {
    let mut field = WeatherParticleField::new(WeatherKind::Snow, 100, 100, 99);
    let before: Vec<f32> = field.particles.iter().map(|p| p.y).collect();
    field.tick(200);
    let moved = field.particles.iter().zip(before.iter()).filter(|(p, y0)| (p.y - *y0).abs() > 1.0).count();
    assert!(moved > 10, "expected particles to fall, moved={moved}");
}
