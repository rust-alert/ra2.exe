//! 原版产品页的逻辑资源索引（背景 / 按钮多状态 / 字体句柄）。
//!
//! 本模块只描述「页面需要哪些资源」，不负责解码或 GPU 上传。
//! 色块占位菜单仍由 [`crate::menu_view`] 绘制；**在 `visuals_ready` 为真之前，
//! 不得宣称 Pre-Alpha 原版 UI 已交付。**

use crate::{
    menu_view::MenuAction,
    screen::OriginalScreen,
    ui_slots::{UiButtonSlot, slots_for},
};

/// 逻辑资源引用（文件名或装载键；尚未解析为像素）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAssetRef {
    /// 资源逻辑名（通常为 MIX 内文件名，如 `mnbutton.shp`）。
    pub name: String,
    /// 可选调色板名（缺省由装载策略选择）。
    pub palette: Option<String>,
}

impl UiAssetRef {
    /// 仅文件名、无显式调色板。
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            palette: None,
        }
    }

    /// 文件名 + 调色板。
    pub fn with_palette(name: impl Into<String>, palette: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            palette: Some(palette.into()),
        }
    }
}

/// 按钮的视觉状态（对应原版 normal / hover / pressed / disabled / focused）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiButtonVisualState {
    /// 常态。
    Normal,
    /// 悬停。
    Hover,
    /// 按下。
    Pressed,
    /// 禁用。
    Disabled,
    /// 键盘焦点（预留）。
    Focused,
}

/// 单个按钮的逻辑动作、命中框与各状态资源名。
#[derive(Debug, Clone)]
pub struct UiButtonResources {
    /// 与机读 UI 状态一致的入口 id。
    pub entry_id: &'static str,
    /// 壳层导航动作。
    pub action: MenuAction,
    /// 是否可点。
    pub enabled: bool,
    /// 归一化命中框（左、上、右、下，0..1）。
    pub hit: (f32, f32, f32, f32),
    /// 常态精灵。
    pub normal: Option<UiAssetRef>,
    /// 悬停精灵。
    pub hover: Option<UiAssetRef>,
    /// 按下精灵。
    pub pressed: Option<UiAssetRef>,
    /// 禁用精灵。
    pub disabled: Option<UiAssetRef>,
    /// 焦点精灵（可与 hover 共用，预留）。
    pub focused: Option<UiAssetRef>,
}

impl UiButtonResources {
    /// 取某视觉状态的资源；缺省回退到 `normal`（disabled 优先用 `disabled`）。
    pub fn asset_for(&self, state: UiButtonVisualState) -> Option<&UiAssetRef> {
        let primary = match state {
            UiButtonVisualState::Normal => self.normal.as_ref(),
            UiButtonVisualState::Hover => self.hover.as_ref(),
            UiButtonVisualState::Pressed => self.pressed.as_ref(),
            UiButtonVisualState::Disabled => self.disabled.as_ref(),
            UiButtonVisualState::Focused => self.focused.as_ref(),
        };
        primary.or(self.normal.as_ref())
    }

    /// 可点按钮是否已具备至少常态精灵名。
    pub fn has_normal_asset(&self) -> bool {
        self.normal.is_some()
    }
}

/// 一页原版 UI 的资源索引（未解码）。
#[derive(Debug, Clone)]
pub struct UiPageResources {
    /// 原版产品页。
    pub screen: OriginalScreen,
    /// 背景图。
    pub background: Option<UiAssetRef>,
    /// 背景调色板（可与背景引用内 palette 并存；显式页级优先策略由装载层定）。
    pub background_palette: Option<String>,
    /// 面板/装饰层（预留）。
    pub panels: Vec<UiAssetRef>,
    /// 页面按钮。
    pub buttons: Vec<UiButtonResources>,
    /// 本页需要的字体逻辑名（预留）。
    pub fonts: Vec<String>,
}

impl UiPageResources {
    /// 背景与所有**可点**按钮是否都已填常态资源名（仍不表示已 GPU 绘制）。
    pub fn visuals_ready(&self) -> bool {
        if self.background.is_none() {
            return false;
        }
        self.buttons
            .iter()
            .filter(|b| b.enabled)
            .all(UiButtonResources::has_normal_asset)
    }
}

fn slot_to_button(slot: &UiButtonSlot) -> UiButtonResources {
    UiButtonResources {
        entry_id: slot.entry_id,
        action: slot.action,
        enabled: slot.enabled,
        hit: slot.hit,
        normal: slot.normal_shp.map(UiAssetRef::named),
        hover: slot.hover_shp.map(UiAssetRef::named),
        pressed: slot.pressed_shp.map(UiAssetRef::named),
        disabled: slot.disabled_shp.map(UiAssetRef::named),
        focused: None,
    }
}

/// 从现有槽位表构造页面资源索引（资产名仍可为空）。
pub fn page_resources_from_slots(screen: OriginalScreen) -> Option<UiPageResources> {
    let page = slots_for(screen)?;
    Some(UiPageResources {
        screen,
        background: page.background_shp.map(UiAssetRef::named),
        background_palette: page.background_pal.map(str::to_string),
        panels: Vec::new(),
        buttons: page.buttons.iter().map(slot_to_button).collect(),
        fonts: Vec::new(),
    })
}

/// 枚举当前有槽位定义的前置页资源索引（不含对局/结算）。
pub fn catalog_pre_game_pages() -> Vec<UiPageResources> {
    [
        OriginalScreen::MainMenu,
        OriginalScreen::SinglePlayerMenu,
        OriginalScreen::SkirmishLobby,
        OriginalScreen::LoadScreen,
        OriginalScreen::Options,
        OriginalScreen::Network,
    ]
    .into_iter()
    .filter_map(page_resources_from_slots)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_slots_are_not_visuals_ready() {
        let pages = catalog_pre_game_pages();
        assert!(!pages.is_empty());
        for page in &pages {
            assert!(
                !page.visuals_ready(),
                "{} 仍无背景/按钮资源名，不得视为原版 UI 就绪",
                page.screen.as_str()
            );
        }
    }

    #[test]
    fn main_menu_index_keeps_entry_ids() {
        let page = page_resources_from_slots(OriginalScreen::MainMenu).expect("main menu");
        let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
        assert_eq!(ids, ["single_player", "network", "options", "exit"]);
        assert!(!page.buttons[1].enabled);
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
}
