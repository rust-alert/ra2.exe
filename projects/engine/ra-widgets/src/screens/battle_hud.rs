//! 对局侧栏 chrome：按阵营从 `sidec01`/`sidec02` 解码并合成。
//!
//! 文件名与菜单壳层分离；同名 SHP 靠阵营嵌套包区分盟军 / 苏军外观。
//! 战术区保持透明铺到屏底；chrome 仅占用右侧栏。

use ra_assets::{Palette, ShpFile};
use ra_layout::{
    battle_hud_layout_tree, battle_hud_layout_with_metrics, BattleHudChromeMetrics, BattleHudLayout,
    LayoutEngine, Point2, RectPx, Size2, Viewport,
};
use ra_renderer::RgbaImage;

use crate::{
    fs_source::GameAssetSource,
    skirmish_setup::sidebar_chrome_mix,
    ui_decode::{DecodedUiSprite, frame_to_canvas_rgba},
    ui_page::UiAssetRef,
};

/// 对局侧栏调色板。
pub const BATTLE_HUD_PAL: &str = "sidebar.pal";

/// 已解码的对局 HUD chrome（仅右侧栏素材）。
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
    /// `radar.shp`（雷达未开时用末帧）。
    pub radar: Option<DecodedUiSprite>,
    /// `side1.shp`。
    pub side1: Option<DecodedUiSprite>,
    /// `side2.shp`（平铺）。
    pub side2: Option<DecodedUiSprite>,
    /// `side3.shp`。
    pub side3: Option<DecodedUiSprite>,
    /// `addon.shp`。
    pub addon: Option<DecodedUiSprite>,
    /// `repair.shp`。
    pub repair: Option<DecodedUiSprite>,
    /// `sell.shp`。
    pub sell: Option<DecodedUiSprite>,
    /// `powerp.shp`（电表）。
    pub powerp: Option<DecodedUiSprite>,
    /// `tab00`…`tab03`。
    pub tabs: [Option<DecodedUiSprite>; 4],
    /// `optbtn.shp`。
    pub optbtn: Option<DecodedUiSprite>,
    /// `diplobtn.shp`。
    pub diplobtn: Option<DecodedUiSprite>,
    /// 解码失败说明。
    pub errors: Vec<String>,
}

impl BattleHudChrome {
    /// 是否至少解出右栏主体。
    pub fn has_sidebar_body(&self) -> bool {
        self.side1.is_some() || self.credits.is_some() || self.radar.is_some() || self.side2.is_some()
    }
}

fn decode_asset_ref_preferring(source: &GameAssetSource, asset: &UiAssetRef, prefer_mix: &str) -> Result<DecodedUiSprite, String> {
    let frame_idx = asset.frame.unwrap_or(0) as usize;
    let hit = source
        .resolve_preferring(&asset.name, prefer_mix)
        .ok_or_else(|| format!("{}: 不可读", asset.name))?;
    let shp = ShpFile::parse(&hit.bytes).map_err(|e| format!("{}: SHP 解析失败 · {e}", asset.name))?;
    if shp.frames.is_empty() {
        return Err(format!("{}: SHP 无帧", asset.name));
    }
    if frame_idx >= shp.frames.len() {
        return Err(format!("{}: 帧 {} 越界 · 共 {} 帧", asset.name, frame_idx, shp.frames.len()));
    }
    let pal_name = asset.palette.as_deref().ok_or_else(|| format!("{}: 未指定调色板", asset.name))?;
    // 必须与 SHP 同档案取 `sidebar.pal`：`sidec01`/`sidec02` 各有一份，
    // 全局 `resolve` 常被后挂载的苏军包抢走，盟军 SHP + 苏军调色板会整栏发红。
    let pal_hit = source
        .resolve_preferring(pal_name, prefer_mix)
        .or_else(|| source.resolve(pal_name))
        .ok_or_else(|| format!("{pal_name}: 调色板不可读"))?;
    let palette = Palette::parse(&pal_hit.bytes).map_err(|e| format!("{pal_name}: 解析失败 · {e}"))?;
    let frame = &shp.frames[frame_idx];
    let image = frame_to_canvas_rgba(&shp, frame, &palette).ok_or_else(|| format!("{}#{}: 画布 RGBA 构造失败", asset.name, frame_idx))?;
    Ok(DecodedUiSprite {
        label: format!("{}#{}", asset.name, frame_idx),
        image,
        origin: format!("{} · pal {}", hit.explain(), pal_hit.explain()),
        frame: frame_idx as u16,
        canvas: (shp.width, shp.height),
        frame_rect: (frame.frame_x, frame.frame_y, frame.frame_width, frame.frame_height),
    })
}

fn try_decode(source: &GameAssetSource, mix: &str, name: &str, frame: u16, errors: &mut Vec<String>) -> Option<DecodedUiSprite> {
    let asset = UiAssetRef::with_palette_frame(name, BATTLE_HUD_PAL, frame);
    match decode_asset_ref_preferring(source, &asset, mix) {
        Ok(s) => Some(s),
        Err(e) => {
            errors.push(e);
            None
        }
    }
}

fn radar_frame_index(_source: &GameAssetSource, _mix: &str) -> u16 {
    // 未建雷达时原版显示阵营徽（盟军鹰 / 苏军镰锤），在 `radar.shp` 首帧。
    // 末帧多为关屏黑块，不能当默认态。
    0
}

/// 按本地阵营解码对局 HUD chrome。
pub fn decode_battle_hud_chrome(source: &GameAssetSource, side: &str) -> BattleHudChrome {
    let mix = sidebar_chrome_mix(side).to_string();
    let mut errors = Vec::new();
    let radar_frame = radar_frame_index(source, &mix);
    let mut tabs = [None, None, None, None];
    for (i, slot) in tabs.iter_mut().enumerate() {
        let name = format!("tab{i:02}.shp");
        *slot = try_decode(source, &mix, &name, 0, &mut errors);
    }
    BattleHudChrome {
        side: side.to_string(),
        mix: mix.clone(),
        credits: try_decode(source, &mix, "credits.shp", 0, &mut errors),
        top: try_decode(source, &mix, "top.shp", 0, &mut errors),
        radar: try_decode(source, &mix, "radar.shp", radar_frame, &mut errors),
        side1: try_decode(source, &mix, "side1.shp", 0, &mut errors),
        side2: try_decode(source, &mix, "side2.shp", 0, &mut errors),
        side3: try_decode(source, &mix, "side3.shp", 0, &mut errors),
        addon: try_decode(source, &mix, "addon.shp", 0, &mut errors),
        repair: try_decode(source, &mix, "repair.shp", 0, &mut errors),
        sell: try_decode(source, &mix, "sell.shp", 0, &mut errors),
        powerp: try_decode(source, &mix, "powerp.shp", 0, &mut errors),
        tabs,
        optbtn: try_decode(source, &mix, "optbtn.shp", 0, &mut errors),
        diplobtn: try_decode(source, &mix, "diplobtn.shp", 0, &mut errors),
        errors,
    }
}

fn blit_rgba(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    if src.width() == 0 || src.height() == 0 || dst.width() == 0 || dst.height() == 0 {
        return;
    }
    for row in 0..src.height() {
        let dy = y + row as i32;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..src.width() {
            let dx = x + col as i32;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let si = ((row * src.width() + col) * 4) as usize;
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            let sa = src.as_raw()[si + 3];
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
                continue;
            }
            let inv = 255u32 - sa as u32;
            for c in 0..3 {
                let s = src.as_raw()[si + c] as u32;
                let d = dst.as_mut()[di + c] as u32;
                dst.as_mut()[di + c] = ((s * sa as u32 + d * inv) / 255) as u8;
            }
            dst.as_mut()[di + 3] = 255;
        }
    }
}

fn blit_stretched(dst: &mut RgbaImage, src: &RgbaImage, rect: RectPx) {
    if rect.w <= 0 || rect.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    for row in 0..rect.h as u32 {
        let sy = row * src.height() / rect.h as u32;
        for col in 0..rect.w as u32 {
            let sx = col * src.width() / rect.w as u32;
            let si = ((sy * src.width() + sx) * 4) as usize;
            let dx = rect.x + col as i32;
            let dy = rect.y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width() || dy as u32 >= dst.height() {
                continue;
            }
            let raw = src.as_raw();
            if raw[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 钮面优先按 SHP 画布原尺寸居中贴入命中格；仅当源图大于格时才拉伸，避免变形。
fn blit_button_in_cell(dst: &mut RgbaImage, src: &RgbaImage, cell: RectPx) {
    if cell.w <= 0 || cell.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    if sw <= cell.w && sh <= cell.h {
        let x = cell.x + (cell.w - sw) / 2;
        let y = cell.y + (cell.h - sh) / 2;
        blit_rgba(dst, src, x, y);
    } else {
        blit_stretched(dst, src, cell);
    }
}

fn fill_rect(dst: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    for y in rect.y..rect.y + rect.h {
        if y < 0 {
            continue;
        }
        let y = y as u32;
        if y >= dst.height() {
            break;
        }
        for x in rect.x..rect.x + rect.w {
            if x < 0 {
                continue;
            }
            let x = x as u32;
            if x >= dst.width() {
                break;
            }
            let di = ((y * dst.width() + x) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&rgba);
        }
    }
}

fn sample_opaque_rgb(img: &RgbaImage) -> Option<[u8; 4]> {
    let raw = img.as_raw();
    let mut i = 0usize;
    while i + 4 <= raw.len() {
        if raw[i + 3] > 200 {
            return Some([raw[i], raw[i + 1], raw[i + 2], 255]);
        }
        i += 4;
    }
    None
}

/// 把已解码 chrome 画进透明页（左战术区保持透明，供地图透出）。
pub fn blit_battle_hud_chrome(page: &mut RgbaImage, chrome: &BattleHudChrome, layout: BattleHudLayout) {
    let sidebar_fill = chrome
        .side2
        .as_ref()
        .or(chrome.side1.as_ref())
        .or(chrome.top.as_ref())
        .and_then(|s| sample_opaque_rgb(&s.image))
        .unwrap_or([40, 44, 52, 255]);
    fill_rect(page, layout.sidebar, sidebar_fill);

    if let Some(s) = &chrome.credits {
        blit_stretched(page, &s.image, layout.credits);
    }
    if let Some(s) = &chrome.top {
        blit_stretched(page, &s.image, layout.top);
    }
    if let Some(s) = &chrome.radar {
        blit_stretched(page, &s.image, layout.radar);
    }
    if let Some(s) = &chrome.side1 {
        blit_stretched(page, &s.image, layout.side1);
    }
    if let Some(tile) = &chrome.side2 {
        let th = tile.image.height().max(1) as i32;
        let mut y = layout.cameo_band.y;
        while y < layout.cameo_band.y + layout.cameo_band.h {
            let remain = layout.cameo_band.y + layout.cameo_band.h - y;
            let h = remain.min(th);
            blit_stretched(page, &tile.image, RectPx::new(layout.cameo_band.x, y, layout.cameo_band.w, h));
            y += th;
        }
    }
    // 底脚只在右栏内铺色，禁止横贯战术区。
    let bottom_fill = chrome
        .addon
        .as_ref()
        .or(chrome.side3.as_ref())
        .and_then(|s| sample_opaque_rgb(&s.image))
        .unwrap_or(sidebar_fill);
    fill_rect(page, layout.bottom_strip, bottom_fill);
    if let Some(s) = &chrome.side3 {
        blit_stretched(page, &s.image, layout.side3);
    }
    if let Some(s) = &chrome.addon {
        blit_stretched(page, &s.image, layout.addon);
    }
    if let Some(s) = &chrome.repair {
        blit_button_in_cell(page, &s.image, layout.repair);
    }
    if let Some(s) = &chrome.sell {
        blit_button_in_cell(page, &s.image, layout.sell);
    }
    if let Some(s) = &chrome.powerp {
        // `powerp.shp` 为窄条带，沿 cameo 左缘纵向平铺成电表，勿整帧拉高。
        let meter_w = layout.power_meter_w.min(layout.sidebar.w).max(1);
        let strip_h = s.image.height().max(1) as i32;
        let mut y = layout.cameo_band.y;
        let bottom = layout.cameo_band.y + layout.cameo_band.h;
        while y < bottom {
            let h = (bottom - y).min(strip_h);
            blit_stretched(
                page,
                &s.image,
                RectPx::new(layout.sidebar.x, y, meter_w, h),
            );
            y += strip_h;
        }
    }
    // 四分类页签贴入布局槽位，勿压住修理/出售拱钮。
    for (i, tab) in chrome.tabs.iter().enumerate() {
        if let Some(tab) = tab {
            blit_button_in_cell(page, &tab.image, layout.tabs[i]);
        }
    }
    // 底脚鹰标与双瓣蓝光已在 `side3.shp`；勿再贴 `optbtn`/`diplobtn`，
    // 否则会盖住原版光晕、露出选项/外交几何图标。命中格仍保留在 layout。
}

/// 便捷：按视口与 chrome 嵌套包度量生成布局并绘制。
pub fn paint_battle_hud_chrome(page: &mut RgbaImage, chrome: &BattleHudChrome) {
    let metrics = BattleHudChromeMetrics::for_mix(&chrome.mix);
    let layout = battle_hud_layout_with_metrics(page.width(), page.height(), metrics);
    blit_battle_hud_chrome(page, chrome, layout);
}

/// 对局 HUD 可点入口（几何权威为 `battle_hud_layout_tree` snapshot）。
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
}

impl BattleHudHit {
    /// 由 snapshot / layout 控件 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "repair" => Some(Self::Repair),
            "sell" => Some(Self::Sell),
            "opt_btn" => Some(Self::Options),
            "diplo_btn" => Some(Self::Diplomacy),
            _ => None,
        }
    }

    /// 稳定入口 id（与 layout tree 叶节点一致）。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Repair => "repair",
            Self::Sell => "sell",
            Self::Options => "opt_btn",
            Self::Diplomacy => "diplo_btn",
        }
    }
}

const BATTLE_HUD_HIT_IDS: [&str; 4] = ["repair", "sell", "opt_btn", "diplo_btn"];

fn battle_hud_snapshot(viewport_w: u32, viewport_h: u32) -> ra_layout::LayoutSnapshot {
    LayoutEngine.solve(
        Viewport {
            size: Size2 {
                width: viewport_w.max(1) as f32,
                height: viewport_h.max(1) as f32,
            },
            ..Viewport::default()
        },
        &battle_hud_layout_tree(viewport_w, viewport_h),
    )
}

/// 视口像素命中（与 `battle_hud_layout` / `RenderPlan` 同源）。
///
/// `layout` 仅用于推断视口尺寸，保持与暂停菜单 `hit_at` 签名同构。
pub fn hit_at(layout: BattleHudLayout, x: i32, y: i32) -> Option<BattleHudHit> {
    let viewport_w = (layout.sidebar.x + layout.sidebar.w).max(1) as u32;
    let viewport_h = layout.sidebar.h.max(1) as u32;
    let snap = battle_hud_snapshot(viewport_w, viewport_h);
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
    None
}
