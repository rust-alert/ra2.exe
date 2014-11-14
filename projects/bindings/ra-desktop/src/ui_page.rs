//! 原版产品页的逻辑资源索引（背景 / 按钮多状态 / 字体句柄）。
//!
//! 本模块只描述「页面需要哪些资源」，不负责解码或 GPU 上传。
//! **在 `declared_refs_complete` 为真之前，不得宣称资源名已齐。**
//! 资源名齐 ≠ 可读 ≠ 已 GPU 绘制 ≠ Pre-Alpha 视觉交付。
//! 当前仅键盘与 [`crate::ui_hit`] 逻辑命中，不绘制按钮图。

use ra_types::GameEdition;

use crate::{
    menu_action::MenuAction,
    screen::OriginalScreen,
    ui_slots::{UiButtonSlot, pudlgbgn_palette, slots_for},
};

/// 逻辑资源引用（文件名或装载键；尚未解析为像素）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAssetRef {
    /// 资源逻辑名（通常为 MIX 内文件名，如 `sdbtnanm.shp`）。
    pub name: String,
    /// 可选调色板名（缺省由装载策略选择）。
    pub palette: Option<String>,
    /// 可选帧号（动画 SHP 多状态时使用）。
    pub frame: Option<u16>,
}

impl UiAssetRef {
    /// 仅文件名、无显式调色板。
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), palette: None, frame: None }
    }

    /// 文件名 + 调色板。
    pub fn with_palette(name: impl Into<String>, palette: impl Into<String>) -> Self {
        Self { name: name.into(), palette: Some(palette.into()), frame: None }
    }

    /// 文件名 + 调色板 + 帧。
    pub fn with_palette_frame(name: impl Into<String>, palette: impl Into<String>, frame: u16) -> Self {
        Self { name: name.into(), palette: Some(palette.into()), frame: Some(frame) }
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
    /// 循环影片（BIK），可空。
    pub movie: Option<UiAssetRef>,
    /// 面板/装饰层。
    pub panels: Vec<UiAssetRef>,
    /// 页面按钮。
    pub buttons: Vec<UiButtonResources>,
    /// 本页需要的字体逻辑名。
    pub fonts: Vec<String>,
}

impl UiPageResources {
    /// 背景与所有**可点**按钮是否都已填常态资源名。
    ///
    /// 只检查「引用是否写上」，不检查可读、解码或屏上绘制。
    pub fn declared_refs_complete(&self) -> bool {
        if self.background.is_none() {
            return false;
        }
        self.buttons.iter().filter(|b| b.enabled).all(UiButtonResources::has_normal_asset)
    }
}

fn slot_frame_asset(shp: Option<&'static str>, pal: Option<&'static str>, frame: Option<u16>) -> Option<UiAssetRef> {
    let frame = frame?;
    let shp = shp?;
    match pal {
        Some(pal) => Some(UiAssetRef::with_palette_frame(shp, pal, frame)),
        None => Some(UiAssetRef { name: shp.to_string(), palette: None, frame: Some(frame) }),
    }
}

fn slot_to_button(slot: &UiButtonSlot) -> UiButtonResources {
    UiButtonResources {
        entry_id: slot.entry_id,
        action: slot.action,
        enabled: slot.enabled,
        hit: slot.hit,
        normal: slot_frame_asset(slot.anim_shp, slot.anim_pal, slot.normal_frame),
        hover: slot_frame_asset(slot.anim_shp, slot.anim_pal, slot.hover_frame),
        pressed: slot_frame_asset(slot.anim_shp, slot.anim_pal, slot.pressed_frame),
        disabled: slot_frame_asset(slot.anim_shp, slot.anim_pal, slot.disabled_frame),
        focused: None,
    }
}

/// 从现有槽位表构造页面资源索引（资产名仍可为空）。
///
/// `edition` 影响退出确认底板调色板（见 [`pudlgbgn_palette`]）；缺省按 RA2。
pub fn page_resources_from_slots(screen: OriginalScreen) -> Option<UiPageResources> {
    page_resources_from_slots_with_edition(screen, None)
}

/// 同 [`page_resources_from_slots`]，并按资料片选择退出确认调色板。
pub fn page_resources_from_slots_with_edition(screen: OriginalScreen, edition: Option<GameEdition>) -> Option<UiPageResources> {
    let page = slots_for(screen)?;
    let background = match (page.background_shp, page.background_pal) {
        (Some(shp), Some(pal)) => Some(UiAssetRef::with_palette_frame(shp, pal, page.background_frame)),
        (Some(shp), None) => Some(UiAssetRef { name: shp.to_string(), palette: None, frame: Some(page.background_frame) }),
        (None, _) => None,
    };
    let exit_pal = pudlgbgn_palette(edition);
    Some(UiPageResources {
        screen,
        background,
        background_palette: page.background_pal.map(str::to_string),
        movie: page.movie_bik.map(UiAssetRef::named),
        panels: page
            .panels
            .iter()
            .map(|p| {
                let pal = if p.id == "exit_modal_bg" { exit_pal } else { p.pal };
                UiAssetRef::with_palette_frame(p.shp, pal, p.frame)
            })
            .collect(),
        buttons: page.buttons.iter().map(slot_to_button).collect(),
        fonts: page.fonts.iter().map(|s| (*s).to_string()).collect(),
    })
}

/// 枚举当前有槽位定义的前置页资源索引（不含对局/结算）。
pub fn catalog_pre_game_pages() -> Vec<UiPageResources> {
    [
        OriginalScreen::MainMenu,
        OriginalScreen::SinglePlayerMenu,
        OriginalScreen::Campaign,
        OriginalScreen::SkirmishLobby,
        OriginalScreen::ChooseMap,
        OriginalScreen::LoadScreen,
        OriginalScreen::Options,
        OriginalScreen::ExitConfirm,
        OriginalScreen::Network,
    ]
    .into_iter()
    .filter_map(page_resources_from_slots)
    .collect()
}
