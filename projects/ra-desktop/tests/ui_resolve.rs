//! 集成测试：原 `src/ui_resolve.rs` 内联测试迁出。

use ra_desktop::{
    menu_action::MenuAction,
    screen::OriginalScreen,
    ui_page::{UiAssetRef, UiButtonResources, UiPageResources, page_resources_from_slots},
    ui_resolve::*,
};
use ra_types::{AssetSource, RaError};
use std::collections::HashMap;

struct MemSource(HashMap<String, Vec<u8>>);

impl AssetSource for MemSource {
    fn read(&self, name: &str) -> Result<Vec<u8>, RaError> {
        self.0.get(&name.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::Msg(format!("missing {name}")))
    }
}

#[test]
fn empty_slots_report_zero_named() {
    // LoadScreen 仅有空按钮槽，无背景/面板/字体名；用于验证「零命名」契约。
    let page = page_resources_from_slots(OriginalScreen::LoadScreen).unwrap();
    let src = MemSource(HashMap::new());
    let report = resolve_page(&src, &page);
    assert_eq!(report.named, 0);
    assert!(!report.all_named_readable());
    assert!(report.banner_note().contains("0"));
}

#[test]
fn main_menu_names_are_counted_even_when_missing() {
    let page = page_resources_from_slots(OriginalScreen::MainMenu).unwrap();
    let src = MemSource(HashMap::new());
    let report = resolve_page(&src, &page);
    assert!(report.named > 0);
    assert!(!report.missing.is_empty());
    assert!(!report.all_named_readable());
}

#[test]
fn counts_readable_and_missing() {
    let page = UiPageResources {
        screen: OriginalScreen::MainMenu,
        background: Some(UiAssetRef::named("bg.shp")),
        background_palette: Some("bg.pal".into()),
        movie: None,
        panels: Vec::new(),
        buttons: vec![UiButtonResources {
            entry_id: "single_player",
            action: MenuAction::OpenSinglePlayer,
            enabled: true,
            hit: (0.0, 0.0, 1.0, 1.0),
            normal: Some(UiAssetRef::named("btn.shp")),
            hover: Some(UiAssetRef::named("missing.shp")),
            pressed: None,
            disabled: None,
            focused: None,
        }],
        fonts: Vec::new(),
    };
    let mut map = HashMap::new();
    map.insert("bg.shp".into(), vec![1]);
    map.insert("bg.pal".into(), vec![1]);
    map.insert("btn.shp".into(), vec![1]);
    let report = resolve_page(&MemSource(map), &page);
    assert_eq!(report.named, 4);
    assert_eq!(report.readable, 3);
    assert_eq!(report.missing, vec!["missing.shp".to_string()]);
}
