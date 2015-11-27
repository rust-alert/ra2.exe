//! 对局内暂停菜单（原版 Esc 菜单）。
//!
//! 原版暂停态侧栏**不出现**修理 / 出售 / QWER 页签 / cameo 生产线 / 底边命令条。
//! 视觉分块：
//! 1. **战术区中心装饰板**：`credits` + 空 `top` + `radar` 首帧竖拼放大（纯装饰，不画钮）。
//! 2. **侧栏清空带**（`side1`+`cameo`）：盖住战术控件后，只列暂停菜单项。
//! 3. 顶栏 `credits`/`radar` 仍由对局 HUD chrome 保留。
//!
//! 几何跟对局 HUD 同口径（窗口像素）。阵营包必须 `resolve_preferring(sidec01|sidec02)`。

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
};

pub use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS as BUTTON_IDS;

/// 对局侧栏调色板（与 HUD chrome 同包）。
const BATTLE_PAUSE_PAL: &str = "sidebar.pal";

/// `sidebttn.shp` 画布（安装内实测）。
const SIDEBTTN_W: i32 = 125;
const SIDEBTTN_H: i32 = 25;
/// 钮列上下间距。
const SIDEBTTN_GAP: i32 = 4;
/// 中心装饰板放大倍数（最近邻；只放大装饰，不放大钮）。
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
    /// `credits.shp`（中心板顶条；装饰）。
    pub credits: Option<DecodedUiSprite>,
    /// `top.shp`（中心板空蓝槽；**不**叠选项/外交钮）。
    pub top: Option<DecodedUiSprite>,
    /// `radar.shp` 首帧（阵营徽）。
    pub radar: Option<DecodedUiSprite>,
    /// 已拼好的中心装饰板（credits+top+radar，未放大）。
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

fn blit_opaque(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let raw = src.as_raw();
    for row in 0..sh {
        let dy = y + row;
        if dy < 0 || dy >= dh {
            continue;
        }
        for col in 0..sw {
            let dx = x + col;
            if dx < 0 || dx >= dw {
                continue;
            }
            let si = ((row as u32 * src.width() + col as u32) * 4) as usize;
            if raw[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 竖向拼接 credits → top（空槽）→ radar，得到中心装饰板。
fn compose_center_panel(
    credits: Option<&DecodedUiSprite>,
    top: Option<&DecodedUiSprite>,
    radar: Option<&DecodedUiSprite>,
) -> Option<RgbaImage> {
    let parts: Vec<&RgbaImage> = [credits, top, radar]
        .into_iter()
        .flatten()
        .map(|s| &s.image)
        .collect();
    if parts.is_empty() {
        return None;
    }
    let w = parts.iter().map(|p| p.width()).max().unwrap_or(1).max(1);
    let h = parts.iter().map(|p| p.height()).sum::<u32>().max(1);
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;
    let mut y = 0i32;
    for part in parts {
        let x = ((w as i32) - (part.width() as i32)) / 2;
        blit_opaque(&mut page, part, x, y);
        y += part.height() as i32;
    }
    Some(page)
}

/// 按本地阵营解码暂停菜单素材（必须 `resolve_preferring`，避免全局落到苏军包）。
pub fn decode_battle_pause_chrome(source: &GameAssetSource, side: &str) -> BattlePauseChrome {
    let mix = crate::skirmish_setup::sidebar_chrome_mix(side).to_string();
    let mut errors = Vec::new();
    let credits = decode_preferring(source, &mix, "credits.shp", 0, &mut errors);
    let top = decode_preferring(source, &mix, "top.shp", 0, &mut errors);
    let radar = decode_preferring(source, &mix, "radar.shp", 0, &mut errors);
    let center_panel = compose_center_panel(credits.as_ref(), top.as_ref(), radar.as_ref());
    let button_normal = decode_preferring(source, &mix, "sidebttn.shp", 0, &mut errors);
    let button_pressed = decode_preferring(source, &mix, "sidebttn.shp", 1, &mut errors);
    let button_hover = decode_preferring(source, &mix, "sidebttn.shp", 2, &mut errors)
        .or_else(|| button_normal.clone());
    BattlePauseChrome {
        side: side.to_string(),
        mix,
        credits,
        top,
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

/// 暂停时盖住的侧栏战术控件区：`side1`（修理/出售/QWER 页签）+ `cameo_band`。
///
/// 原版暂停后这些控件不出现；菜单项画在此清空带内。
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

/// 暂停时盖住的底边命令条（编队/部署等战术钮；暂停态不显示）。
pub fn command_bar_cover_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> RectPx {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    rect_px_from_snapshot(&snap, "command_bar")
}

/// 暂停六钮在窗口像素中的矩形（落在已清空的 side1+cameo 带内）。
pub fn button_rects(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
) -> [RectPx; 6] {
    let snap = pause_snap(viewport_w, viewport_h, metrics);
    let sidebar = rect_px_from_snapshot(&snap, "sidebar");
    let strip = menu_strip_rect(viewport_w, viewport_h, metrics);
    let btn_w = SIDEBTTN_W.min(sidebar.w.saturating_sub(8)).max(1);
    let btn_h = SIDEBTTN_H.min(strip.h.max(1)).max(1);
    let x = sidebar.x + ((sidebar.w - btn_w) / 2).max(0);
    let mut rects = [RectPx::new(0, 0, 1, 1); 6];
    // 前五钮靠 strip 顶向下排。
    let mut y = strip.y + 8;
    for i in 0..5 {
        rects[i] = RectPx::new(x, y, btn_w, btn_h);
        y += btn_h + SIDEBTTN_GAP;
    }
    // resume 贴 strip 底（接近原版「回到任务」落点）。
    let resume_y = (strip.y + strip.h - btn_h - 8).max(strip.y);
    rects[5] = RectPx::new(x, resume_y, btn_w, btn_h);
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

/// 中心装饰板目标矩形（战术区居中）。
pub fn center_panel_dest_rect(
    viewport_w: u32,
    viewport_h: u32,
    metrics: BattleHudChromeMetrics,
    panel_w: u32,
    panel_h: u32,
) -> RectPx {
    let world = dim_rect(viewport_w, viewport_h, metrics);
    let dw = ((panel_w as i32) * LOGO_SCALE).max(1).min(world.w.max(1));
    let dh = ((panel_h as i32) * LOGO_SCALE).max(1).min(world.h.max(1));
    let x = world.x + (world.w - dw) / 2;
    let y = world.y + (world.h - dh) / 2;
    RectPx::new(x, y, dw, dh)
}

/// 入口表（测试 / 诊断用）。
pub fn button_ids() -> &'static [&'static str; 6] {
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
