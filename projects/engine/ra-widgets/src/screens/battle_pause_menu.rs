//! 对局内暂停菜单（原版 Esc 菜单）。
//!
//! 原版暂停态：
//! - **保留**侧栏 chrome（`credits`/`top`/`radar`/`side2`/`side3`/`addon`）
//! - **保留**底边命令条端盖与轨（`lendcap`/`lspacer`/`rendcap`），不整条涂黑、不盖钮槽
//! - **不出现**修理 / 出售 / QWER 页签 / cameo 生产线 / 命令钮 / 顶栏选项外交钮
//! - 战术区压暗；侧栏清空带内列暂停菜单项
//! - 中心阵营徽：雷达 SHP 首帧裁掉左右金属侧轨后的徽芯放大（不是整块「雷达槽」）
//!
//! 几何跟对局 HUD 同口径（窗口像素）。阵营包须按 [`crate::skirmish_setup::sidebar_chrome_mix_candidates`] 先 MD 后基座。

use ra_assets::{Palette, ShpFile};
use ra_layout::{
    battle_hud_world_viewport, rect_px_from_snapshot, solve_battle_hud_with_metrics, BattleHudChromeMetrics,
    LayoutSnapshot, RectPx, BATTLE_PAUSE_MENU_BUTTON_IDS,
};
use ra_renderer::RgbaImage;

use crate::{
    fs_source::GameAssetSource,
    screens::page::UiAssetRef,
    skin::decode::{frame_to_canvas_rgba, DecodedUiSprite},
    skirmish_setup::{UiFactionFamily, sidebar_radar_pal_resolved, sidebar_radar_shp_resolved},
};

pub use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS as BUTTON_IDS;

/// 对局侧栏调色板（与 HUD chrome 同包）。
const BATTLE_PAUSE_PAL: &str = "sidebar.pal";

/// `sidebttn.shp` 画布（安装内实测）。
const SIDEBTTN_W: i32 = 125;
const SIDEBTTN_H: i32 = 25;
/// 钮列上下间距。
const SIDEBTTN_GAP: i32 = 4;
/// 中心阵营徽相对战术区短边的占比（保持徽芯宽高比）。
const LOGO_FIT: f32 = 0.72;
/// cameo 内侧清空边距，留给 `side2` 金属边轨透出。
const CAMEO_RAIL_INSET: i32 = 12;
/// `radar.shp` 左右金属侧轨宽度（裁掉后只留鹰徽+放射底，避免「放大雷达槽」观感）。
const RADAR_SIDE_RAIL_PX: u32 = 22;

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
    /// `radar.shp` 首帧（侧栏顶徽；中心板取其裁轨后的徽芯）。
    pub radar: Option<DecodedUiSprite>,
    /// 中心阵营徽画布（`radar` 裁掉左右侧轨，未放大）。
    pub center_panel: Option<RgbaImage>,
    /// `sidebttn.shp` 常态帧（仅 cameo 菜单列）。
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
    let pal_hit = match source
        .resolve_preferring(pal_name, mix)
        .or_else(|| source.resolve(pal_name))
    {
        Some(h) => h,
        None => {
            errors.push(format!("{pal_name}: 调色板不可读"));
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
        frame_rect: (
            frame_ref.frame_x,
            frame_ref.frame_y,
            frame_ref.frame_width,
            frame_ref.frame_height,
        ),
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
pub fn decode_battle_pause_chrome_resolved(
    source: &GameAssetSource,
    side: &str,
    faction_id: Option<&str>,
) -> BattlePauseChrome {
    let family = UiFactionFamily::resolve(side, faction_id);
    let mixes = family.sidebar_mix_candidates();
    let mix = family.sidebar_mix().to_string();
    let mut errors = Vec::new();
    let radar_shp = sidebar_radar_shp_resolved(side, faction_id);
    let radar_pal = sidebar_radar_pal_resolved(side, faction_id);
    let radar = decode_candidates(source, mixes, radar_shp, radar_pal, 0, &mut errors);
    let center_panel = radar
        .as_ref()
        .map(|s| crop_radar_emblem(&s.image))
        .or_else(|| radar.as_ref().map(|s| s.image.clone()));
    let button_normal = decode_candidates(source, mixes, "sidebttn.shp", BATTLE_PAUSE_PAL, 0, &mut errors);
    let button_pressed = decode_candidates(source, mixes, "sidebttn.shp", BATTLE_PAUSE_PAL, 1, &mut errors);
    let button_hover = decode_candidates(source, mixes, "sidebttn.shp", BATTLE_PAUSE_PAL, 2, &mut errors)
        .or_else(|| button_normal.clone());
    BattlePauseChrome {
        side: side.to_string(),
        mix,
        radar,
        center_panel,
        button_normal,
        button_pressed,
        button_hover,
        errors,
    }
}

fn pause_snap(viewport_w: u32, viewport_h: u32, metrics: BattleHudChromeMetrics) -> LayoutSnapshot {
    solve_battle_hud_with_metrics(viewport_w, viewport_h, metrics)
}

/// 裁掉雷达 SHP 左右侧栏金属轨，只留中央阵营徽+放射底。
fn crop_radar_emblem(src: &RgbaImage) -> RgbaImage {
    let w = src.width().max(1);
    let h = src.height().max(1);
    let rail = RADAR_SIDE_RAIL_PX.min(w / 4);
    let x0 = rail;
    let x1 = w.saturating_sub(rail).max(x0 + 1);
    let nw = x1 - x0;
    let mut out = RgbaImage::from_raw(nw, h, vec![0u8; (nw as usize) * (h as usize) * 4])
        .unwrap_or_else(|| src.clone());
    let raw = src.as_raw();
    for y in 0..h {
        for x in 0..nw {
            let si = ((y * w + (x0 + x)) * 4) as usize;
            let di = ((y * nw + x) * 4) as usize;
            out.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
    out
}

/// 暂停时菜单钮落点带：`side1` + `cameo_band`（chrome 仍由 HUD 保留，不在此整带涂黑）。
pub fn menu_strip_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> RectPx {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    let side1 = rect_px_from_snapshot(&snap, "side1");
    let cameo = rect_px_from_snapshot(&snap, "cameo_band");
    let y0 = side1.y;
    let y1 = cameo.y + cameo.h;
    RectPx::new(side1.x, y0, side1.w.max(cameo.w), (y1 - y0).max(1))
}

/// 暂停时仅盖住的 cameo 内芯（留边轨给 `side2`）。
pub fn cameo_clear_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> RectPx {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    let cameo = rect_px_from_snapshot(&snap, "cameo_band");
    let inset = CAMEO_RAIL_INSET.min(cameo.w / 4).max(0);
    RectPx::new(
        cameo.x + inset,
        cameo.y,
        (cameo.w - inset * 2).max(1),
        cameo.h.max(1),
    )
}

/// 暂停四钮在窗口像素中的矩形（落在 side1+cameo 带内；resume 贴底）。
pub fn button_rects(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> [RectPx; 4] {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    let sidebar = rect_px_from_snapshot(&snap, "sidebar");
    let strip = menu_strip_rect(viewport_w, viewport_h, metrics);
    let btn_w = SIDEBTTN_W.min(sidebar.w.saturating_sub(8)).max(1);
    let btn_h = SIDEBTTN_H.min(strip.h.max(1)).max(1);
    let x = sidebar.x + ((sidebar.w - btn_w) / 2).max(0);
    let mut rects = [RectPx::new(0, 0, 1, 1); 4];
    // 前三钮靠 strip 顶向下排（Options / Fullscreen / Abort）。
    let mut y = strip.y + 8;
    for i in 0..3 {
        rects[i] = RectPx::new(x, y, btn_w, btn_h);
        y += btn_h + SIDEBTTN_GAP;
    }
    // resume 贴 strip 底（接近原版「回到任务」落点，叠在 side3 一带上方）。
    let resume_y = (strip.y + strip.h - btn_h - 8).max(strip.y);
    rects[3] = RectPx::new(x, resume_y, btn_w, btn_h);
    rects
}

/// 窗口像素命中（与合成同口径；装饰板不可点）。
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

/// 暂停时盖住命令条（战术区底栏），避免露出可点命令槽。
pub fn command_bar_cover_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> RectPx {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    rect_px_from_snapshot(&snap, "command_bar")
}

/// 中心阵营徽目标矩形（战术区居中，按短边占比缩放，保持宽高比）。
pub fn center_panel_dest_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
    panel_w: u32,
    panel_h: u32,
) -> RectPx {
    let world = dim_rect(viewport_w, viewport_h, metrics);
    let pw = panel_w.max(1) as f32;
    let ph = panel_h.max(1) as f32;
    let fit = (world.w.min(world.h) as f32) * LOGO_FIT;
    let scale = (fit / pw.max(ph)).max(1.0);
    let dw = ((pw * scale) as i32).max(1).min(world.w.max(1));
    let dh = ((ph * scale) as i32).max(1).min(world.h.max(1));
    let x = world.x + (world.w - dw) / 2;
    let y = world.y + (world.h - dh) / 2;
    RectPx::new(x, y, dw, dh)
}

/// 入口表（测试 / 诊断用）。
pub fn button_ids() -> &'static [&'static str; 4] {
    &BATTLE_PAUSE_MENU_BUTTON_IDS
}

/// 供合成选帧（仅 cameo 菜单列）。
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

impl BattlePauseChrome {
    /// 是否有可用中心板或菜单钮面。
    pub fn has_art(&self) -> bool {
        self.center_panel.is_some() || self.radar.is_some() || self.button_normal.is_some()
    }
}
