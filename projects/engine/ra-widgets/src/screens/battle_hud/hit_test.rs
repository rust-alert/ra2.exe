//! 对局侧栏 chrome：按已解析 [`UiFactionChrome`] 从 `sidecNN` 解码并合成。
//!
//! 文件名与菜单壳层分离；同名 SHP 靠 `MixFileIndex` 嵌套包区分外观。
//! 战术区铺到命令条顶边；chrome 含右侧栏与底边命令条。

use ra_layout::{
    BattleHudChromeMetrics, COMMAND_BAR_BUTTON_COUNT, COMMAND_BAR_BUTTON_IDS, LayoutSnapshot,
    Point2, SIDEBAR_TAB_COUNT, hit_cameo_slot,
    rect_px_from_snapshot,
};
use ra_renderer::RgbaImage;

use crate::
skirmish_setup::UiFactionChrome;

/// 对局侧栏调色板。

use super::chrome::BattleHudChrome;
use super::command_bar::command_bar_shp_index_for_visual;

/// 对局 HUD 可点入口（几何权威为 `solve_battle_hud` snapshot）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleHudHit {
    /// 修理模式。
    Repair,
    /// 出售模式。
    Sell,
    /// 选项。
    Options,
    /// 外交。
    Diplomacy,
    /// 底边命令条可视槽（`cmdN` / `ButtonList` 下标）。
    CommandButton(usize),
    /// 分类页签（0=建筑 / 1=步兵 / 2=载具 / 3=飞行器）。
    SidebarTab(usize),
    /// 当前页可视 cameo 槽。
    Cameo(usize),
}


impl BattleHudHit {
    /// 由 snapshot 控件 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "repair" => Some(Self::Repair),
            "sell" => Some(Self::Sell),
            "opt_btn" => Some(Self::Options),
            "diplo_btn" => Some(Self::Diplomacy),
            "tab00" => Some(Self::SidebarTab(0)),
            "tab01" => Some(Self::SidebarTab(1)),
            "tab02" => Some(Self::SidebarTab(2)),
            "tab03" => Some(Self::SidebarTab(3)),
            _ => {
                if let Some(rest) = id.strip_prefix("cmd") {
                    if let Ok(slot) = rest.parse::<usize>() {
                        if slot < COMMAND_BAR_BUTTON_COUNT {
                            return Some(Self::CommandButton(slot));
                        }
                    }
                }
                None
            }
        }
    }

    /// 稳定入口 id。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Repair => "repair",
            Self::Sell => "sell",
            Self::Options => "opt_btn",
            Self::Diplomacy => "diplo_btn",
            Self::SidebarTab(0) => "tab00",
            Self::SidebarTab(1) => "tab01",
            Self::SidebarTab(2) => "tab02",
            Self::SidebarTab(3) => "tab03",
            Self::SidebarTab(_) => "tab00",
            Self::Cameo(_) => "cameo_band",
            Self::CommandButton(slot) => COMMAND_BAR_BUTTON_IDS
                .get(slot)
                .copied()
                .unwrap_or(COMMAND_BAR_BUTTON_IDS[0]),
        }
    }
}


const BATTLE_HUD_HIT_IDS: [&str; 8] = [
    "repair",
    "sell",
    "opt_btn",
    "diplo_btn",
    "tab00",
    "tab01",
    "tab02",
    "tab03",
];

pub fn hit_at(snap: &LayoutSnapshot, x: i32, y: i32) -> Option<BattleHudHit> {
    hit_at_with_chrome(snap, None, BattleHudChromeMetrics::sidec01().power_w, 0, x, y)
}

/// 带 chrome / cameo 槽数的命中。
pub fn hit_at_with_chrome(
    snap: &LayoutSnapshot,
    chrome: Option<&BattleHudChrome>,
    power_meter_w: i32,
    cameo_count: usize,
    x: i32,
    y: i32,
) -> Option<BattleHudHit> {
    if let Some(chrome) = chrome {
        let bar = rect_px_from_snapshot(snap, "command_bar");
        if bar.contains(x, y) {
            let point = Point2 {
                x: x as f32,
                y: y as f32,
            };
            for (visual, id) in COMMAND_BAR_BUTTON_IDS.iter().enumerate() {
                let Some(shp_i) = command_bar_shp_index_for_visual(visual) else {
                    continue;
                };
                if chrome.command_buttons.get(shp_i).and_then(|s| s.as_ref()).is_none() {
                    continue;
                }
                if snap
                    .get(id)
                    .is_some_and(|el| el.layout.rect.width > 0.0 && el.layout.rect.contains(point))
                {
                    return Some(BattleHudHit::CommandButton(visual));
                }
            }
            // 点在命令条空白（金属轨）上仍吞掉，避免穿透到地图手势。
            return None;
        }
    }

    let point = Point2 {
        x: x as f32,
        y: y as f32,
    };
    for id in BATTLE_HUD_HIT_IDS {
        if snap
            .get(id)
            .is_some_and(|el| el.layout.rect.contains(point))
        {
            return BattleHudHit::from_entry_id(id);
        }
    }

    let band = rect_px_from_snapshot(snap, "cameo_band");
    if let Some(slot) = hit_cameo_slot(band, power_meter_w, x, y) {
        if slot < cameo_count {
            return Some(BattleHudHit::Cameo(slot));
        }
        // 点在空 cameo 槽仍吞掉，避免误清选中。
        return None;
    }
    if band.contains(x, y) {
        return None;
    }
    let _ = SIDEBAR_TAB_COUNT;
    None
}

/// 单枚建造栏图标绘制描述。
#[derive(Debug, Clone, Copy)]
pub struct BattleCameoPaint<'a> {
    /// 规则类型键。
    pub type_id: &'a str,
    /// 已解码图标（缺图时画占位）。
    pub image: Option<&'a RgbaImage>,
    /// 当前是否可下单。
    pub enabled: bool,
    /// 是否处于放置选中态。
    pub selected: bool,
    /// 生产进度：`None` 空闲；`Some(0.0..1.0)` 建造中（1.0 为完工待放）。
    pub progress: Option<f32>,
}
