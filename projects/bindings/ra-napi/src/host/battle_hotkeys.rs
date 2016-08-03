//! 对局热键：读安装根 / mod 的 `keyboard.ini`，宿主查表分发。
//!
//! 语义与 Win32 编码属桌面输入层，不进引擎 crate。仅借用 `IniDocument` 解析字节。
//! 缺文件时用零售默认的 Rust 常量表回退（不内嵌 `.ini` 文本）。

use std::collections::HashMap;

use ra_assets::IniDocument;
use ra_types::AssetSource;
use winit::keyboard::KeyCode;

/// 一条热键和弦：Win32 VK + 修饰键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HotkeyChord {
    /// Win32 虚拟键码（低 8 位）。
    pub vk: u8,
    /// Shift。
    pub shift: bool,
    /// Ctrl。
    pub ctrl: bool,
    /// Alt。
    pub alt: bool,
}

impl HotkeyChord {
    /// 从零售整型编码解码：`vk + Shift*256 + Ctrl*512 + Alt*1024`（可带 `2048` 标记，匹配时忽略）。
    pub fn from_encoded(value: i32) -> Option<Self> {
        if value < 0 {
            return None;
        }
        let mut n = value as u32;
        n &= !2048;
        let alt = (n & 1024) != 0;
        let ctrl = (n & 512) != 0;
        let shift = (n & 256) != 0;
        let vk = (n & 0xff) as u8;
        if vk == 0 {
            return None;
        }
        Some(Self { vk, shift, ctrl, alt })
    }
}

/// `[Hotkey]` 动作（编队槽 `1..=10`，其中 `10` 对应数字键 `0`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyAction {
    CenterView,
    Options,
    CenterOnRadarEvent,
    RightSidebarUp,
    RightSidebarDown,
    LeftSidebarDown,
    LeftSidebarUp,
    Delete,
    TeamSelect(u8),
    TeamAddSelect(u8),
    TeamCreate(u8),
    TeamCenter(u8),
    ToggleAlliance,
    PlaceBeacon,
    AllToCheer,
    DeployObject,
    InfantryTab,
    Follow,
    GuardObject,
    CenterBase,
    ToggleRepair,
    ToggleSell,
    NextObject,
    CombatantSelect,
    StructureTab,
    UnitTab,
    StopObject,
    TypeSelect,
    PageUser,
    DefenseTab,
    ScatterObject,
    PlanningMode,
    View(u8),
    SetView(u8),
    Taunt(u8),
    ScreenCapture,
    SidebarPageUp,
    SidebarUp,
    SidebarPageDown,
    SidebarDown,
}

/// 解析后的热键表。
#[derive(Debug, Clone, Default)]
pub struct HotkeyMap {
    by_action: HashMap<HotkeyAction, HotkeyChord>,
    by_chord: HashMap<HotkeyChord, HotkeyAction>,
}

impl HotkeyMap {
    /// 空表。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 零售默认键位（与原版 `keyboard.ini` 数值一致，以 Rust 常量表达，不读内嵌文件）。
    pub fn stock_ra2() -> Self {
        let mut map = Self::empty();
        for &(name, encoded) in STOCK_RA2_HOTKEYS {
            if let Some(action) = parse_hotkey_action(name) {
                if let Some(chord) = HotkeyChord::from_encoded(encoded) {
                    map.insert(action, chord);
                }
            }
        }
        map
    }

    /// 是否无绑定。
    pub fn is_empty(&self) -> bool {
        self.by_action.is_empty()
    }

    /// 绑定条数。
    pub fn len(&self) -> usize {
        self.by_action.len()
    }

    /// 查动作和弦。
    pub fn chord(&self, action: HotkeyAction) -> Option<HotkeyChord> {
        self.by_action.get(&action).copied()
    }

    /// 按当前按键和弦反查动作。
    pub fn action_for(&self, vk: u8, shift: bool, ctrl: bool, alt: bool) -> Option<HotkeyAction> {
        self.by_chord
            .get(&HotkeyChord {
                vk,
                shift,
                ctrl,
                alt,
            })
            .copied()
    }

    fn insert(&mut self, action: HotkeyAction, chord: HotkeyChord) {
        if let Some(old) = self.by_action.insert(action, chord) {
            self.by_chord.remove(&old);
        }
        if let Some(prev) = self.by_chord.insert(chord, action) {
            if prev != action {
                self.by_action.remove(&prev);
            }
        }
    }

    /// 用另一张表覆盖同名动作（mod 可只改部分键）。
    pub fn merge_overlay(&mut self, overlay: HotkeyMap) {
        for (action, chord) in overlay.by_action {
            self.insert(action, chord);
        }
    }
}

/// 从资源源读 `keyboard.ini`：有则叠在零售默认上，无则纯默认。
pub fn load_hotkey_map(source: &dyn AssetSource) -> HotkeyMap {
    let mut map = HotkeyMap::stock_ra2();
    if let Ok(bytes) = source.read("keyboard.ini") {
        match parse_keyboard_ini(&bytes) {
            Ok(overlay) if !overlay.is_empty() => map.merge_overlay(overlay),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "keyboard.ini 解析失败，沿用零售默认键位"),
        }
    }
    map
}

/// 解析 `[Hotkey]` 字节。
pub fn parse_keyboard_ini(bytes: &[u8]) -> Result<HotkeyMap, String> {
    let doc = IniDocument::parse(bytes).map_err(|e| e.to_string())?;
    let mut map = HotkeyMap::empty();
    let Some(section) = doc.sections.iter().find(|s| s.name_key.eq_ignore_ascii_case("Hotkey"))
    else {
        return Err("keyboard.ini 缺少 [Hotkey]".into());
    };
    for (key, value) in section.pairs() {
        let Some(action) = parse_hotkey_action(key.trim())
        else {
            continue;
        };
        let Ok(encoded) = value.trim().parse::<i32>()
        else {
            continue;
        };
        let Some(chord) = HotkeyChord::from_encoded(encoded)
        else {
            continue;
        };
        map.insert(action, chord);
    }
    Ok(map)
}

/// `TeamSelect_1` → 槽 `0`，…，`TeamSelect_10` → 槽 `9`。
pub fn team_slot_index(team_number_1_to_10: u8) -> Option<usize> {
    match team_number_1_to_10 {
        1..=10 => Some(usize::from(team_number_1_to_10 - 1)),
        _ => None,
    }
}

/// winit 物理键 → Win32 VK（热键表用）。
pub fn key_code_to_vk(code: KeyCode) -> Option<u8> {
    Some(match code {
        KeyCode::Escape => 0x1B,
        KeyCode::Space => 0x20,
        KeyCode::PageUp => 0x21,
        KeyCode::PageDown => 0x22,
        KeyCode::End => 0x23,
        KeyCode::Home => 0x24,
        KeyCode::ArrowLeft => 0x25,
        KeyCode::ArrowUp => 0x26,
        KeyCode::ArrowRight => 0x27,
        KeyCode::ArrowDown => 0x28,
        KeyCode::Delete => 0x2E,
        KeyCode::Digit0 | KeyCode::Numpad0 => 0x30,
        KeyCode::Digit1 | KeyCode::Numpad1 => 0x31,
        KeyCode::Digit2 | KeyCode::Numpad2 => 0x32,
        KeyCode::Digit3 | KeyCode::Numpad3 => 0x33,
        KeyCode::Digit4 | KeyCode::Numpad4 => 0x34,
        KeyCode::Digit5 | KeyCode::Numpad5 => 0x35,
        KeyCode::Digit6 | KeyCode::Numpad6 => 0x36,
        KeyCode::Digit7 | KeyCode::Numpad7 => 0x37,
        KeyCode::Digit8 | KeyCode::Numpad8 => 0x38,
        KeyCode::Digit9 | KeyCode::Numpad9 => 0x39,
        KeyCode::KeyA => 0x41,
        KeyCode::KeyB => 0x42,
        KeyCode::KeyC => 0x43,
        KeyCode::KeyD => 0x44,
        KeyCode::KeyE => 0x45,
        KeyCode::KeyF => 0x46,
        KeyCode::KeyG => 0x47,
        KeyCode::KeyH => 0x48,
        KeyCode::KeyI => 0x49,
        KeyCode::KeyJ => 0x4A,
        KeyCode::KeyK => 0x4B,
        KeyCode::KeyL => 0x4C,
        KeyCode::KeyM => 0x4D,
        KeyCode::KeyN => 0x4E,
        KeyCode::KeyO => 0x4F,
        KeyCode::KeyP => 0x50,
        KeyCode::KeyQ => 0x51,
        KeyCode::KeyR => 0x52,
        KeyCode::KeyS => 0x53,
        KeyCode::KeyT => 0x54,
        KeyCode::KeyU => 0x55,
        KeyCode::KeyV => 0x56,
        KeyCode::KeyW => 0x57,
        KeyCode::KeyX => 0x58,
        KeyCode::KeyY => 0x59,
        KeyCode::KeyZ => 0x5A,
        KeyCode::F1 => 0x70,
        KeyCode::F2 => 0x71,
        KeyCode::F3 => 0x72,
        KeyCode::F4 => 0x73,
        KeyCode::F5 => 0x74,
        KeyCode::F6 => 0x75,
        KeyCode::F7 => 0x76,
        KeyCode::F8 => 0x77,
        KeyCode::F9 => 0x78,
        KeyCode::F10 => 0x79,
        KeyCode::F11 => 0x7A,
        KeyCode::F12 => 0x7B,
        _ => return None,
    })
}

/// 零售 RA2 `[Hotkey]` 默认值（名称 → 编码）。来源：原版安装根 `keyboard.ini`。
const STOCK_RA2_HOTKEYS: &[(&str, i32)] = &[
    ("CenterView", 12),
    ("Options", 27),
    ("CenterOnRadarEvent", 32),
    ("RightSidebarUp", 33),
    ("RightSidebarDown", 34),
    ("LeftSidebarDown", 35),
    ("LeftSidebarUp", 36),
    ("Delete", 46),
    ("TeamSelect_10", 48),
    ("TeamSelect_1", 49),
    ("TeamSelect_2", 50),
    ("TeamSelect_3", 51),
    ("TeamSelect_4", 52),
    ("TeamSelect_5", 53),
    ("TeamSelect_6", 54),
    ("TeamSelect_7", 55),
    ("TeamSelect_8", 56),
    ("TeamSelect_9", 57),
    ("ToggleAlliance", 65),
    ("PlaceBeacon", 66),
    ("AllToCheer", 67),
    ("DeployObject", 68),
    ("InfantryTab", 69),
    ("Follow", 70),
    ("GuardObject", 71),
    ("CenterBase", 72),
    ("ToggleRepair", 75),
    ("ToggleSell", 76),
    ("NextObject", 78),
    ("CombatantSelect", 80),
    ("StructureTab", 81),
    ("UnitTab", 82),
    ("StopObject", 83),
    ("TypeSelect", 84),
    ("PageUser", 85),
    ("DefenseTab", 87),
    ("ScatterObject", 88),
    ("PlanningMode", 90),
    ("View1", 112),
    ("View2", 113),
    ("View3", 114),
    ("View4", 115),
    ("Taunt_1", 116),
    ("Taunt_2", 117),
    ("Taunt_3", 118),
    ("Taunt_4", 119),
    ("Taunt_5", 120),
    ("Taunt_6", 121),
    ("Taunt_7", 122),
    ("Taunt_8", 123),
    ("TeamAddSelect_10", 304),
    ("TeamAddSelect_1", 305),
    ("TeamAddSelect_2", 306),
    ("TeamAddSelect_3", 307),
    ("TeamAddSelect_4", 308),
    ("TeamAddSelect_5", 309),
    ("TeamAddSelect_6", 310),
    ("TeamAddSelect_7", 311),
    ("TeamAddSelect_8", 312),
    ("TeamAddSelect_9", 313),
    ("TeamCreate_10", 560),
    ("TeamCreate_1", 561),
    ("TeamCreate_2", 562),
    ("TeamCreate_3", 563),
    ("TeamCreate_4", 564),
    ("TeamCreate_5", 565),
    ("TeamCreate_6", 566),
    ("TeamCreate_7", 567),
    ("TeamCreate_8", 568),
    ("TeamCreate_9", 569),
    ("ScreenCapture", 579),
    ("SetView1", 624),
    ("SetView2", 625),
    ("SetView3", 626),
    ("SetView4", 627),
    ("TeamCenter_10", 1072),
    ("TeamCenter_1", 1073),
    ("TeamCenter_2", 1074),
    ("TeamCenter_3", 1075),
    ("TeamCenter_4", 1076),
    ("TeamCenter_5", 1077),
    ("TeamCenter_6", 1078),
    ("TeamCenter_7", 1079),
    ("TeamCenter_8", 1080),
    ("TeamCenter_9", 1081),
    ("SidebarPageUp", 2085),
    ("SidebarUp", 2086),
    ("SidebarPageDown", 2087),
    ("SidebarDown", 2088),
];

fn parse_hotkey_action(name: &str) -> Option<HotkeyAction> {
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "centerview" => HotkeyAction::CenterView,
        "options" => HotkeyAction::Options,
        "centeronradarevent" => HotkeyAction::CenterOnRadarEvent,
        "rightsidebarup" => HotkeyAction::RightSidebarUp,
        "rightsidebardown" => HotkeyAction::RightSidebarDown,
        "leftsidebardown" => HotkeyAction::LeftSidebarDown,
        "leftsidebarup" => HotkeyAction::LeftSidebarUp,
        "delete" => HotkeyAction::Delete,
        "togglealliance" => HotkeyAction::ToggleAlliance,
        "placebeacon" => HotkeyAction::PlaceBeacon,
        "alltocheer" => HotkeyAction::AllToCheer,
        "deployobject" => HotkeyAction::DeployObject,
        "infantrytab" => HotkeyAction::InfantryTab,
        "follow" => HotkeyAction::Follow,
        "guardobject" => HotkeyAction::GuardObject,
        "centerbase" => HotkeyAction::CenterBase,
        "togglerepair" => HotkeyAction::ToggleRepair,
        "togglesell" => HotkeyAction::ToggleSell,
        "nextobject" => HotkeyAction::NextObject,
        "combatantselect" => HotkeyAction::CombatantSelect,
        "structuretab" => HotkeyAction::StructureTab,
        "unittab" => HotkeyAction::UnitTab,
        "stopobject" => HotkeyAction::StopObject,
        "typeselect" => HotkeyAction::TypeSelect,
        "pageuser" => HotkeyAction::PageUser,
        "defensetab" => HotkeyAction::DefenseTab,
        "scatterobject" => HotkeyAction::ScatterObject,
        "planningmode" => HotkeyAction::PlanningMode,
        "screencapture" => HotkeyAction::ScreenCapture,
        "sidebarpageup" => HotkeyAction::SidebarPageUp,
        "sidebarup" => HotkeyAction::SidebarUp,
        "sidebarpagedown" => HotkeyAction::SidebarPageDown,
        "sidebardown" => HotkeyAction::SidebarDown,
        _ => {
            if let Some(n) = lower.strip_prefix("teamselect_") {
                return team_slot(n).map(HotkeyAction::TeamSelect);
            }
            if let Some(n) = lower.strip_prefix("teamaddselect_") {
                return team_slot(n).map(HotkeyAction::TeamAddSelect);
            }
            if let Some(n) = lower.strip_prefix("teamcreate_") {
                return team_slot(n).map(HotkeyAction::TeamCreate);
            }
            if let Some(n) = lower.strip_prefix("teamcenter_") {
                return team_slot(n).map(HotkeyAction::TeamCenter);
            }
            if let Some(n) = lower.strip_prefix("setview") {
                return view_slot(n).map(HotkeyAction::SetView);
            }
            if let Some(n) = lower.strip_prefix("view") {
                return view_slot(n).map(HotkeyAction::View);
            }
            if let Some(n) = lower.strip_prefix("taunt_") {
                return taunt_slot(n).map(HotkeyAction::Taunt);
            }
            return None;
        }
    })
}

fn team_slot(s: &str) -> Option<u8> {
    let n: u8 = s.parse().ok()?;
    (1..=10).contains(&n).then_some(n)
}

fn view_slot(s: &str) -> Option<u8> {
    let n: u8 = s.parse().ok()?;
    (1..=4).contains(&n).then_some(n)
}

fn taunt_slot(s: &str) -> Option<u8> {
    let n: u8 = s.parse().ok()?;
    (1..=8).contains(&n).then_some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
