//! 自 `bindings/ra-napi/src/host/battle_hotkeys.rs` 迁出的单元测试（集成测试 crate）。

// 自 bindings/ra-napi/src/host/battle_hotkeys.rs :: tests
use ra_napi::host::battle_hotkeys::*;
use winit::keyboard::KeyCode;

#[test]
fn stock_defaults_match_retail_codes() {
    let map = HotkeyMap::stock_ra2();
    assert_eq!(map.action_for(68, false, false, false), Some(HotkeyAction::DeployObject));
    assert_eq!(map.action_for(81, false, false, false), Some(HotkeyAction::StructureTab));
    assert_eq!(map.action_for(49, false, false, false), Some(HotkeyAction::TeamSelect(1)));
    assert_eq!(map.action_for(49, false, true, false), Some(HotkeyAction::TeamCreate(1)));
    assert_eq!(map.action_for(49, true, false, false), Some(HotkeyAction::TeamAddSelect(1)));
    assert_eq!(map.action_for(48, false, false, false), Some(HotkeyAction::TeamSelect(10)));
    assert_eq!(map.action_for(27, false, false, false), Some(HotkeyAction::Options));
    assert_eq!(map.action_for(37, false, false, false), Some(HotkeyAction::SidebarPageUp));
}

#[test]
fn mod_ini_overrides_deploy() {
    let bytes = b"[Hotkey]\nDeployObject=70\n";
    let overlay = parse_keyboard_ini(bytes).expect("parse");
    let mut map = HotkeyMap::stock_ra2();
    map.merge_overlay(overlay);
    assert_eq!(map.action_for(70, false, false, false), Some(HotkeyAction::DeployObject));
    assert_eq!(map.action_for(68, false, false, false), None);
    assert_eq!(map.action_for(71, false, false, false), Some(HotkeyAction::GuardObject));
}

#[test]
fn key_code_to_vk_letters() {
    assert_eq!(key_code_to_vk(KeyCode::KeyD), Some(68));
    assert_eq!(key_code_to_vk(KeyCode::Digit1), Some(49));
    assert_eq!(key_code_to_vk(KeyCode::Escape), Some(27));
}
