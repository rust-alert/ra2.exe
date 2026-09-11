//! 集成测试：原 `src/color_schemes.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn parse_colors_and_house() {
    let doc = IniDocument::parse(
        b"[Colors]\nGold=41,240,230\nDarkRed=0,230,255\nGrey=0,0,131\n\
[Americans]\nColor=Gold\n\
[Russians]\nColor=DarkRed\n\
[Neutral]\nColor=Grey\n",
    )
    .unwrap();
    let schemes = ColorSchemes::from_rules(&doc);
    assert_eq!(schemes.len(), 3);
    assert_eq!(schemes.hsv_for_house(&doc, "Americans"), Some(Hsv { h: 41, s: 240, v: 230 }));
    assert_eq!(schemes.hsv_for_house(&doc, "Neutral"), Some(Hsv { h: 0, s: 0, v: 131 }));
}

#[test]
fn from_layered_overrides_color_hsv() {
    let base = IniDocument::parse(b"[Colors]\nGold=41,240,230\n").unwrap();
    let top = IniDocument::parse(b"[Colors]\nGold=10,20,30\nDarkRed=0,230,255\n").unwrap();
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let docs = [base, top];
    let schemes = ColorSchemes::from_layered(LayeredIniView::new(&docs, &policy));
    assert_eq!(schemes.get("Gold"), Some(Hsv { h: 10, s: 20, v: 30 }));
    assert_eq!(schemes.get("DarkRed"), Some(Hsv { h: 0, s: 230, v: 255 }));
}

#[test]
fn palette_falls_back_without_hsv() {
    let doc = IniDocument::parse(b"[Colors]\nGrey=0,0,131\n[Neutral]\nColor=Grey\n").unwrap();
    let schemes = ColorSchemes::from_rules(&doc);
    let mut base = Palette { colors: [Rgba::transparent(); 256] };
    // 模拟 unittem 默认 remap 带偏红。
    for i in 16..32 {
        base.colors[i] = Rgba::rgb(200, 40, 40);
    }
    let out = schemes.palette_for_house(&doc, &base, "Neutral");
    // Grey=0,0,131 → 无饱和灰斜坡，不应再是纯红主色。
    assert!(out.colors[16].r == out.colors[16].g && out.colors[16].g == out.colors[16].b);
}
