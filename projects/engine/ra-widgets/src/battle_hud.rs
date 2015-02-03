//! 对局侧栏 / 底栏 chrome：按阵营从 `sidec01`/`sidec02` 解码并合成。
//!
//! 文件名与菜单壳层分离；同名 SHP 靠阵营嵌套包区分盟军 / 苏军外观。

use ra_assets::{Palette, ShpFile};
use ra_layout::{BattleHudLayout, RectPx, battle_hud_layout};
use ra_renderer::RgbaImage;

use crate::{
    fs_source::GameAssetSource,
    skirmish_setup::sidebar_chrome_mix,
    ui_decode::{DecodedUiSprite, frame_to_canvas_rgba},
    ui_page::UiAssetRef,
};

/// 对局侧栏调色板。
pub const BATTLE_HUD_PAL: &str = "sidebar.pal";

/// 已解码的对局 HUD chrome（右栏 + 底栏素材）。
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
    let pal_hit = source
        .resolve(pal_name)
        .or_else(|| source.resolve_preferring(pal_name, prefer_mix))
        .ok_or_else(|| format!("{pal_name}: 调色板不可读"))?;
    let palette = Palette::parse(&pal_hit.bytes).map_err(|e| format!("{pal_name}: 解析失败 · {e}"))?;
    let frame = &shp.frames[frame_idx];
    let image = frame_to_canvas_rgba(&shp, frame, &palette).ok_or_else(|| format!("{}#{}: 画布 RGBA 构造失败", asset.name, frame_idx))?;
    Ok(DecodedUiSprite {
        label: format!("{}#{}", asset.name, frame_idx),
        image,
        origin: hit.explain(),
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

fn radar_frame_index(source: &GameAssetSource, mix: &str) -> u16 {
    let Some(hit) = source.resolve_preferring("radar.shp", mix)
    else {
        return 0;
    };
    let Ok(shp) = ShpFile::parse(&hit.bytes)
    else {
        return 0;
    };
    shp.frames.len().saturating_sub(1) as u16
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
    if let Some(s) = &chrome.side3 {
        blit_stretched(page, &s.image, layout.side3);
    }
    if let Some(s) = &chrome.addon {
        blit_stretched(page, &s.image, layout.addon);
    }
    if let Some(s) = &chrome.repair {
        blit_stretched(page, &s.image, layout.repair);
    }
    if let Some(s) = &chrome.sell {
        blit_stretched(page, &s.image, layout.sell);
    }
    if let Some(s) = &chrome.powerp {
        let meter = RectPx::new(layout.sidebar.x, layout.cameo_band.y, 16.min(layout.sidebar.w), layout.cameo_band.h.max(1));
        blit_stretched(page, &s.image, meter);
    }
    let tab_y = layout.side1.y + layout.side1.h - 18;
    let mut tab_x = layout.sidebar.x + 20;
    for tab in chrome.tabs.iter().flatten() {
        blit_rgba(page, &tab.image, tab_x, tab_y);
        tab_x += tab.image.width() as i32 + 2;
    }

    let bottom_fill = chrome
        .addon
        .as_ref()
        .or(chrome.side3.as_ref())
        .and_then(|s| sample_opaque_rgb(&s.image))
        .unwrap_or(sidebar_fill);
    fill_rect(page, layout.bottom_strip, bottom_fill);
    if let Some(s) = &chrome.optbtn {
        blit_stretched(page, &s.image, layout.opt_btn);
    }
    if let Some(s) = &chrome.diplobtn {
        blit_stretched(page, &s.image, layout.diplo_btn);
    }
}

/// 便捷：按视口生成布局并绘制 chrome。
pub fn paint_battle_hud_chrome(page: &mut RgbaImage, chrome: &BattleHudChrome) {
    let layout = battle_hud_layout(page.width(), page.height());
    blit_battle_hud_chrome(page, chrome, layout);
}
