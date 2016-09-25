//! 对局内暂停菜单（原版 Esc 菜单）。
//!
//! 几何权威为 [`ra_layout::solve_battle_pause_at`]（专属 layout，**不是** battle HUD，
//! **也不是**主菜单 shell）。合成只消费 snapshot：全屏 `dim` + 右缘 `sidebttn` 四钮。
//! 禁止再画主菜单 `sdtp` / `sdbtnanm`，禁止把 `radar.shp` 放大成中心徽。

use ra_assets::{Palette, ShpFile};
use ra_layout::{BATTLE_PAUSE_MENU_BUTTON_IDS, LayoutSnapshot, RectPx, rect_px_from_snapshot, solve_battle_pause_at};

use crate::{
    fs_source::GameAssetSource,
    screens::page::UiAssetRef,
    skin::decode::{DecodedUiSprite, frame_to_canvas_rgba},
    skirmish_setup::UiFactionChrome,
};

pub use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS as BUTTON_IDS;

/// 对局侧栏调色板（与 HUD chrome 同包）。
const BATTLE_PAUSE_PAL: &str = "sidebar.pal";

/// 暂停菜单命中结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePauseMenuHit {
    /// 打开选项。
    Options,
    /// 切换全屏。
    Fullscreen,
    /// 放弃任务。
    Abort,
    /// 回到游戏。
    Resume,
}

impl BattlePauseMenuHit {
    /// 由入口 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "options" => Some(Self::Options),
            "fullscreen" => Some(Self::Fullscreen),
            "abort" => Some(Self::Abort),
            "resume" => Some(Self::Resume),
            _ => None,
        }
    }

    /// 稳定入口 id（与 [`BUTTON_IDS`] 一致）。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Options => "options",
            Self::Fullscreen => "fullscreen",
            Self::Abort => "abort",
            Self::Resume => "resume",
        }
    }
}

/// 已解码的暂停菜单阵营素材（跟本地 house 绑定）。
#[derive(Debug, Clone)]
pub struct BattlePauseChrome {
    /// 阵营短名（如 `Americans` / `Russians`）。
    pub side: String,
    /// 实际优先读取的嵌套包名。
    pub mix: String,
    /// `sidebttn.shp` 常态帧。
    pub button_normal: Option<DecodedUiSprite>,
    /// `sidebttn.shp` 按下帧。
    pub button_pressed: Option<DecodedUiSprite>,
    /// `sidebttn.shp` 高亮/悬停帧（缺则回常态）。
    pub button_hover: Option<DecodedUiSprite>,
    /// 解码失败说明。
    pub errors: Vec<String>,
}

fn decode_preferring(
    source: &GameAssetSource,
    mix: &str,
    name: &str,
    pal_name: &str,
    frame: u16,
    errors: &mut Vec<String>,
) -> Option<DecodedUiSprite> {
    let asset = UiAssetRef::with_palette_frame(name, pal_name, frame);
    let frame_idx = asset.frame.unwrap_or(0) as usize;
    let hit = match source.resolve_preferring(&asset.name, mix) {
        Some(h) => h,
        None => {
            errors.push(format!("{name}: 不可读（prefer {mix}）"));
            return None;
        }
    };
    let shp = match ShpFile::parse(&hit.bytes) {
        Ok(s) => s,
        Err(e) => {
            errors.push(format!("{name}: SHP 解析失败 · {e}"));
            return None;
        }
    };
    if shp.frames.is_empty() || frame_idx >= shp.frames.len() {
        errors.push(format!("{name}: 帧 {frame_idx} 不可用"));
        return None;
    }
    // 与 HUD 同：调色板必须来自 prefer 档案，禁止全局回退串阵营。
    let pal_hit = match source.resolve_preferring(pal_name, mix) {
        Some(h) => h,
        None => {
            errors.push(format!("{pal_name}: 调色板不可读（prefer {mix}）"));
            return None;
        }
    };
    let palette = match Palette::parse(&pal_hit.bytes) {
        Ok(p) => p,
        Err(e) => {
            errors.push(format!("{pal_name}: 解析失败 · {e}"));
            return None;
        }
    };
    let frame_ref = &shp.frames[frame_idx];
    let image = match frame_to_canvas_rgba(&shp, frame_ref, &palette) {
        Some(img) => img,
        None => {
            errors.push(format!("{name}#{frame_idx}: 画布 RGBA 构造失败"));
            return None;
        }
    };
    Some(DecodedUiSprite {
        label: format!("{name}#{frame_idx}"),
        image,
        origin: format!("{} · pal {}", hit.explain(), pal_hit.explain()),
        frame: frame_idx as u16,
        canvas: (shp.width, shp.height),
        frame_rect: (frame_ref.frame_x, frame_ref.frame_y, frame_ref.frame_width, frame_ref.frame_height),
    })
}

fn decode_candidates(
    source: &GameAssetSource,
    mixes: &[&str],
    name: &str,
    pal_name: &str,
    frame: u16,
    errors: &mut Vec<String>,
) -> Option<DecodedUiSprite> {
    let mut last: Option<DecodedUiSprite> = None;
    let mut local_errors = Vec::new();
    for mix in mixes {
        local_errors.clear();
        if let Some(sprite) = decode_preferring(source, mix, name, pal_name, frame, &mut local_errors) {
            return Some(sprite);
        }
        last = None;
    }
    errors.extend(local_errors);
    last
}

/// 按本地阵营解码暂停菜单素材（须先 MD 后基座，避免全局落到错误阵营包）。
pub fn decode_battle_pause_chrome(source: &GameAssetSource, side: &str) -> BattlePauseChrome {
    decode_battle_pause_chrome_resolved(source, side, None)
}

/// 同 [`decode_battle_pause_chrome`]，可带 rules `Side=`。
pub fn decode_battle_pause_chrome_resolved(source: &GameAssetSource, side: &str, faction_id: Option<&str>) -> BattlePauseChrome {
    decode_battle_pause_chrome_with(source, side, faction_id, None)
}

/// 同 [`decode_battle_pause_chrome_resolved`]，可注入已解析的 [`UiFactionChrome`]。
pub fn decode_battle_pause_chrome_with(
    source: &GameAssetSource,
    side: &str,
    _faction_id: Option<&str>,
    side_chrome: Option<&UiFactionChrome>,
) -> BattlePauseChrome {
    let Some(chrome) = UiFactionChrome::resolve(side_chrome)
    else {
        return BattlePauseChrome {
            side: side.to_string(),
            mix: String::new(),
            button_normal: None,
            button_pressed: None,
            button_hover: None,
            errors: vec!["缺少 Side chrome（无 MixFileIndex）".into()],
        };
    };
    let mixes_owned = chrome.sidebar_mix_candidates();
    let mixes: Vec<&str> = mixes_owned.iter().map(String::as_str).collect();
    let mix = chrome.sidebar_mix();
    let mut errors = Vec::new();
    let button_normal = decode_candidates(source, &mixes, "sidebttn.shp", BATTLE_PAUSE_PAL, 0, &mut errors);
    let button_pressed = decode_candidates(source, &mixes, "sidebttn.shp", BATTLE_PAUSE_PAL, 1, &mut errors);
    let button_hover = decode_candidates(source, &mixes, "sidebttn.shp", BATTLE_PAUSE_PAL, 2, &mut errors).or_else(|| button_normal.clone());
    BattlePauseChrome { side: side.to_string(), mix, button_normal, button_pressed, button_hover, errors }
}

/// 暂停菜单 snapshot（与合成 / 命中同口径）。
pub fn pause_snapshot(viewport_w: u32, viewport_h: u32) -> LayoutSnapshot {
    solve_battle_pause_at(viewport_w, viewport_h)
}

/// 全屏压暗矩形。
pub fn dim_rect(viewport_w: u32, viewport_h: u32) -> RectPx {
    rect_px_from_snapshot(&pause_snapshot(viewport_w, viewport_h), "dim")
}

/// 暂停四钮在窗口像素中的矩形（来自专属 layout snapshot）。
pub fn button_rects(viewport_w: u32, viewport_h: u32) -> [RectPx; 4] {
    let snap = pause_snapshot(viewport_w, viewport_h);
    let mut rects = [RectPx::new(0, 0, 1, 1); 4];
    for (i, id) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().enumerate() {
        rects[i] = rect_px_from_snapshot(&snap, id);
    }
    rects
}

/// 窗口像素命中（与合成同口径）。
pub fn hit_at(viewport_w: u32, viewport_h: u32, x: i32, y: i32) -> Option<BattlePauseMenuHit> {
    let snap = pause_snapshot(viewport_w, viewport_h);
    let hit = snap.hit_test(ra_layout::Point2 { x: x as f32, y: y as f32 })?;
    BattlePauseMenuHit::from_entry_id(hit.id.0.as_str())
}

/// 入口表（测试 / 诊断用）。
pub fn button_ids() -> &'static [&'static str; 4] {
    &BATTLE_PAUSE_MENU_BUTTON_IDS
}

/// 供合成选帧。
pub fn resolve_sidebttn<'a>(chrome: &'a BattlePauseChrome, pressed: bool, hovered: bool) -> Option<&'a DecodedUiSprite> {
    if pressed {
        chrome.button_pressed.as_ref().or(chrome.button_normal.as_ref())
    }
    else if hovered {
        chrome.button_hover.as_ref().or(chrome.button_normal.as_ref())
    }
    else {
        chrome.button_normal.as_ref()
    }
}

impl BattlePauseChrome {
    /// 是否有可用菜单钮面。
    pub fn has_art(&self) -> bool {
        self.button_normal.is_some()
    }
}
