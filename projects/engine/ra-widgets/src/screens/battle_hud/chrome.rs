//! 对局侧栏 chrome：按已解析 [`UiFactionChrome`] 从 `sidecNN` 解码并合成。
//!
//! 文件名与菜单壳层分离；同名 SHP 靠 `MixFileIndex` 嵌套包区分外观。
//! 战术区铺到命令条顶边；chrome 含右侧栏与底边命令条。

use crate::{
    skin::decode::DecodedUiSprite,
    skirmish_setup::UiFactionChrome,
};

/// 对局侧栏调色板。

/// 对局侧栏调色板。
pub const BATTLE_HUD_PAL: &str = "sidebar.pal";

/// 命令条按钮槽位数（`button00`…`button11`；更高编号在零售包中常缺）。
pub const COMMAND_BUTTON_SLOTS: usize = 12;

/// 已解码的对局 HUD chrome（右侧栏 + 底边命令条）。
#[derive(Debug, Clone)]
pub struct BattleHudChrome {
    /// 阵营短名（如 `Americans` / `Russians`）。
    pub side: String,
    /// 实际优先读取的嵌套包名。
    pub mix: String,
    /// `credits.shp`。
    pub credits: Option<DecodedUiSprite>,
    /// `top.shp`。
    pub top: Option<DecodedUiSprite>,
    /// `radar.shp` / `radary.shp` 关图帧（首帧阵营徽）。
    pub radar: Option<DecodedUiSprite>,
    /// 雷达开图动画帧（色帧中间段；末帧关屏黑块不收录）。
    pub radar_open: Vec<DecodedUiSprite>,
    /// `side1.shp`。
    pub side1: Option<DecodedUiSprite>,
    /// `side2.shp`（平铺）。
    pub side2: Option<DecodedUiSprite>,
    /// `side3.shp`。
    pub side3: Option<DecodedUiSprite>,
    /// `addon.shp`。
    pub addon: Option<DecodedUiSprite>,
    /// `repair.shp` 常态帧（frame 0）。
    pub repair: Option<DecodedUiSprite>,
    /// `repair.shp` 按下高亮帧（frame 1；缺帧时回退常态）。
    pub repair_pressed: Option<DecodedUiSprite>,
    /// `sell.shp` 常态帧（frame 0）。
    pub sell: Option<DecodedUiSprite>,
    /// `sell.shp` 按下高亮帧（frame 1；缺帧时回退常态）。
    pub sell_pressed: Option<DecodedUiSprite>,
    /// `powerp.shp`（电表）。
    pub powerp: Option<DecodedUiSprite>,
    /// `tab00`…`tab03` 常态帧（frame 0）。
    pub tabs: [Option<DecodedUiSprite>; 4],
    /// `tab00`…`tab03` 按下高亮帧（frame 1；缺帧时回退常态）。
    pub tabs_pressed: [Option<DecodedUiSprite>; 4],
    /// `optbtn.shp`。
    pub optbtn: Option<DecodedUiSprite>,
    /// `diplobtn.shp`。
    pub diplobtn: Option<DecodedUiSprite>,
    /// `lendcap.shp`（命令条左端盖）。
    pub lendcap: Option<DecodedUiSprite>,
    /// `rendcap.shp`（命令条右端盖）。
    pub rendcap: Option<DecodedUiSprite>,
    /// `lspacer.shp`（命令条中段金属轨，按视口横向拉伸）。
    pub lspacer: Option<DecodedUiSprite>,
    /// `button00`…`button11` 常态帧（frame 0）。
    pub command_buttons: [Option<DecodedUiSprite>; COMMAND_BUTTON_SLOTS],
    /// `button00`…`button11` 按下高亮帧（frame 1；缺帧时回退常态）。
    pub command_buttons_pressed: [Option<DecodedUiSprite>; COMMAND_BUTTON_SLOTS],
    /// 解码失败说明。
    pub errors: Vec<String>,
}


impl BattleHudChrome {
    /// 是否至少解出右栏主体。
    pub fn has_sidebar_body(&self) -> bool {
        self.side1.is_some() || self.credits.is_some() || self.radar.is_some() || self.side2.is_some()
    }
}
