//! 集成测试：原 `src/color_schemes.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn parse_colors_and_house() {
    let doc = IniDocument::parse(
        b"[Colors]\nGold=41,240,230\nDarkRed=0,230,255\n\
[Americans]\nColor=Gold\n\
[Russians]\nColor=DarkRed\n\
[Neutral]\nColor=Grey\n",
    )
    .unwrap();
    let schemes = ColorSchemes::from_rules(&doc);
    assert_eq!(schemes.len(), 2);
    assert_eq!(schemes.hsv_for_house(&doc, "Americans"), Some(Hsv { h: 41, s: 240, v: 230 }));
    assert!(schemes.hsv_for_house(&doc, "Neutral").is_none());
}

#[test]
fn palette_falls_back_without_hsv() {
    let doc = IniDocument::parse(b"[Neutral]\nColor=Grey\n").unwrap();
    let schemes = ColorSchemes::from_rules(&doc);
    let base = Palette { colors: [Rgba::transparent(); 256] };
    let _ = schemes.palette_for_house(&doc, &base, "Neutral");
}
