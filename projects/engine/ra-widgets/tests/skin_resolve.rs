//! 页面资源可读性探测集成测试。

use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    screens::page::{UiAssetRef, UiButtonResources, UiPageResources, page_resources_from_slots},
    skin::resolve::*,
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
    // Network 仍为零命名占位页；LoadScreen 已声明壳层资源。
    let page = page_resources_from_slots(OriginalScreen::Network).unwrap();
    let src = MemSource(HashMap::new());
    let report = resolve_page(&src, &page);
    assert_eq!(report.named, 0);
    assert!(!report.all_named_readable());
    assert!(report.banner_note().contains("0"));
}

#[test]
fn load_screen_declares_country_art() {
    let page = page_resources_from_slots(OriginalScreen::LoadScreen).unwrap();
    assert_eq!(page.background.as_ref().map(|b| b.name.as_str()), Some("ls800ustates.shp"));
    assert!(!page.panels.is_empty());
    assert!(!page.fonts.is_empty());
    let retry = page.buttons.iter().find(|b| b.entry_id == "retry").unwrap();
    assert!(retry.normal.is_some());
    let src = MemSource(HashMap::new());
    let report = resolve_page(&src, &page);
    assert!(report.named > 0);
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
