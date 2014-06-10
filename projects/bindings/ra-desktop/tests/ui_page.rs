//! 集成测试：原 `src/ui_page.rs` 内联测试迁出。

use ra_desktop::{menu_action::MenuAction, screen::OriginalScreen, ui_page::*};
#[test]
fn main_menu_and_single_player_are_declared_complete() {
    let pages = catalog_pre_game_pages();
    assert!(!pages.is_empty());
    for page in &pages {
        match page.screen {
            OriginalScreen::MainMenu | OriginalScreen::SinglePlayerMenu => {
                assert!(page.declared_refs_complete(), "{} 应已声明背景与可点按钮资源名", page.screen.as_str());
                assert!(page.buttons.iter().any(|b| b.enabled && b.normal.is_some()));
                assert_eq!(page.fonts, vec!["game.fnt".to_string()]);
                assert!(!page.panels.is_empty());
                assert_eq!(page.movie.as_ref().map(|m| m.name.as_str()), Some("ra2ts_l.bik"));
            }
            OriginalScreen::SkirmishLobby => {
                // 与主菜单共用壳层 chrome；无循环影片槽。
                assert!(page.declared_refs_complete(), "遭遇战大厅应已声明背景与可点按钮资源名");
                assert!(page.buttons.iter().any(|b| b.enabled && b.normal.is_some()));
                assert_eq!(page.fonts, vec!["game.fnt".to_string()]);
                assert!(!page.panels.is_empty());
                assert!(page.movie.is_none());
            }
            _ => {
                assert!(!page.declared_refs_complete(), "{} 仍无完整背景/按钮资源名", page.screen.as_str());
            }
        }
    }
}

#[test]
fn main_menu_index_keeps_entry_ids() {
    let page = page_resources_from_slots(OriginalScreen::MainMenu).expect("main menu");
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["single_player", "ww_online", "network", "movies", "options", "exit"]);
    assert!(!page.buttons[1].enabled);
    assert!(!page.buttons[2].enabled);
    assert!(!page.buttons[3].enabled);
}

#[test]
fn button_asset_for_falls_back_to_normal() {
    let btn = UiButtonResources {
        entry_id: "x",
        action: MenuAction::Noop,
        enabled: true,
        hit: (0.0, 0.0, 1.0, 1.0),
        normal: Some(UiAssetRef::named("a.shp")),
        hover: None,
        pressed: None,
        disabled: None,
        focused: None,
    };
    assert_eq!(btn.asset_for(UiButtonVisualState::Hover).unwrap().name, "a.shp");
}
