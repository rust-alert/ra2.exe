//! 对局内暂停菜单（原版 Esc 菜单）：战术区压暗 + 阵营中心徽 + 侧栏 `sidebttn` 六钮。
//!
//! 几何跟对局 HUD 同口径（窗口像素），**不**复用主菜单 `sdtp` / `sdbtnanm`。
//! 中心徽与钮面来自本地阵营 `sidec01` / `sidec02`（`radar.shp` 首帧 / `sidebttn.shp`）。

use ra_assets::{Palette, ShpFile};
use ra_layout::{
    battle_hud_world_viewport, rect_px_from_snapshot, solve_battle_hud_with_metrics, BattleHudChromeMetrics,
    LayoutSnapshot, RectPx, BATTLE_PAUSE_MENU_BUTTON_IDS,
};

use crate::{
    fs_source::GameAssetSource,
    screens::page::UiAssetRef,
    skin::decode::{frame_to_canvas_rgba, DecodedUiSprite},
};

pub use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS as BUTTON_IDS;

/// 对局侧栏调色板（与 HUD chrome 同包）。
const BATTLE_PAUSE_PAL: &str = "sidebar.pal";

/// `sidebttn.shp` 画布（安装内实测）。
const SIDEBTTN_W: i32 = 125;
const SIDEBTTN_H: i32 = 25;
/// 钮列上下间距。
const SIDEBTTN_GAP: i32 = 4;
/// 中心徽相对战术区居中时的放大倍数（最近邻）。
const LOGO_SCALE: i32 = 2;

/// 暂停菜单命中结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePauseMenuHit {
    /// 打开选项。
    Options,
    /// 载入游戏。
    Load,
    /// 保存游戏。
    Save,
    /// 重新开始。
    Restart,
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
            "load" => Some(Self::Load),
            "save" => Some(Self::Save),
            "restart" => Some(Self::Restart),
            "abort" => Some(Self::Abort),
            "resume" => Some(Self::Resume),
            _ => None,
        }
    }

    /// 稳定入口 id（与 [`BUTTON_IDS`] 一致）。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Options => "options",
            Self::Load => "load",
            Self::Save => "save",
            Self::Restart => "restart",
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
    /// `radar.shp` 首帧（阵营徽：盟军鹰 / 苏军镰锤）。
    pub logo: Option<DecodedUiSprite>,
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
    frame: u16,
    errors: &mut Vec<String>,
) -> Option<DecodedUiSprite> {
    let asset = UiAssetRef::with_palette_frame(name, BATTLE_PAUSE_PAL, frame);
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
    let pal_hit = match source
        .resolve_preferring(BATTLE_PAUSE_PAL, mix)
        .or_else(|| source.resolve(BATTLE_PAUSE_PAL))
    {
        Some(h) => h,
        None => {
            errors.push(format!("{BATTLE_PAUSE_PAL}: 调色板不可读"));
            return None;
        }
    };
    let palette = match Palette::parse(&pal_hit.bytes) {
        Ok(p) => p,
        Err(e) => {
            errors.push(format!("{BATTLE_PAUSE_PAL}: 解析失败 · {e}"));
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
        frame_rect: (
            frame_ref.frame_x,
            frame_ref.frame_y,
            frame_ref.frame_width,
            frame_ref.frame_height,
        ),
    })
}

/// 按本地阵营解码暂停菜单素材（必须 `resolve_preferring`，避免全局落到苏军包）。
pub fn decode_battle_pause_chrome(source: &GameAssetSource, side: &str) -> BattlePauseChrome {
    let mix = crate::skirmish_setup::sidebar_chrome_mix(side).to_string();
    let mut errors = Vec::new();
    let logo = decode_preferring(source, &mix, "radar.shp", 0, &mut errors);
    let button_normal = decode_preferring(source, &mix, "sidebttn.shp", 0, &mut errors);
    let button_pressed = decode_preferring(source, &mix, "sidebttn.shp", 1, &mut errors);
    let button_hover = decode_preferring(source, &mix, "sidebttn.shp", 2, &mut errors)
        .or_else(|| button_normal.clone());
    BattlePauseChrome {
        side: side.to_string(),
        mix,
        logo,
        button_normal,
        button_pressed,
        button_hover,
        errors,
    }
}

fn pause_snap(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> LayoutSnapshot {
    solve_battle_hud_with_metrics(viewport_w, viewport_h, metrics)
}

/// 暂停六钮在窗口像素中的矩形（前五钮落在 cameo 带顶向下排，`resume` 贴 cameo 底）。
pub fn button_rects(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> [RectPx; 6] {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    let sidebar = rect_px_from_snapshot(&snap, "sidebar");
    let cameo = rect_px_from_snapshot(&snap, "cameo_band");
    let btn_w = SIDEBTTN_W.min(sidebar.w.saturating_sub(8)).max(1);
    let btn_h = SIDEBTTN_H.min(cameo.h.max(1)).max(1);
    let x = sidebar.x + ((sidebar.w - btn_w) / 2).max(0);
    let mut rects = [RectPx::new(0, 0, 1, 1); 6];
    // 前五钮自上而下。
    let mut y = cameo.y + 4;
    for i in 0..5 {
        rects[i] = RectPx::new(x, y, btn_w, btn_h);
        y += btn_h + SIDEBTTN_GAP;
    }
    // resume 贴 cameo 底。
    let resume_y = (cameo.y + cameo.h - btn_h - 4).max(cameo.y);
    rects[5] = RectPx::new(x, resume_y, btn_w, btn_h);
    rects
}

/// 窗口像素命中（与合成同口径）。
pub fn hit_at(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
    x: i32,
    y: i32,
) -> Option<BattlePauseMenuHit> {
    let rects = button_rects(viewport_w, viewport_h, metrics);
    for (id, rect) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().zip(rects.iter()) {
        if x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h {
            return BattlePauseMenuHit::from_entry_id(id);
        }
    }
    None
}

/// 战术区压暗矩形（侧栏以左、命令条以上）。
pub fn dim_rect(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> RectPx {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    battle_hud_world_viewport(&snap)
}

/// 中心徽目标矩形（战术区居中，按 [`LOGO_SCALE`] 放大）。
pub fn logo_dest_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
    logo_w: u32,
    logo_h: u32,
) -> RectPx {
    let world = dim_rect(viewport_w, viewport_h, metrics);
    let dw = ((logo_w as i32) * LOGO_SCALE).max(1).min(world.w.max(1));
    let dh = ((logo_h as i32) * LOGO_SCALE).max(1).min(world.h.max(1));
    let x = world.x + (world.w - dw) / 2;
    let y = world.y + (world.h - dh) / 2;
    RectPx::new(x, y, dw, dh)
}

/// 入口表（测试 / 诊断用）。
pub fn button_ids() -> &'static [&'static str; 6] {
    &BATTLE_PAUSE_MENU_BUTTON_IDS
}

/// 供合成选帧。
pub fn resolve_sidebttn<'a>(
    chrome: &'a BattlePauseChrome,
    pressed: bool,
    hovered: bool,
) -> Option<&'a DecodedUiSprite> {
    if pressed {
        chrome.button_pressed.as_ref().or(chrome.button_normal.as_ref())
    } else if hovered {
        chrome.button_hover.as_ref().or(chrome.button_normal.as_ref())
    } else {
        chrome.button_normal.as_ref()
    }
}

/// 诊断：是否至少解出徽或钮。
impl BattlePauseChrome {
    /// 是否有可用徽或钮面。
    pub fn has_art(&self) -> bool {
        self.logo.is_some() || self.button_normal.is_some()
    }
}
