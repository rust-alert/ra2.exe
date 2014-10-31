//! 进程启动闪屏：独立 presentation owner，不是主菜单壳层槽。
//!
//! 职责：在 MIX 挂载后尽快呈现 `GLSS`/`GLSL` + `GLS.PAL`（宽正好 640 用小图，其余用大图），
//! 将画面**最近邻放大铺满**客户区；最短展示从**首次成功 present** 起算。
//! 期间可继续预处理，到期后再把控制权交给主菜单。

use std::time::{Duration, Instant};

use ra_assets::{CsfFile, FntFile, Palette, ShpFile};
use ra_renderer::RgbaImage;

use crate::{
    fs_source::GameAssetSource,
    ui_compose::blit_rgba,
    ui_decode::frame_to_canvas_rgba,
    ui_text::blit_text_colored,
};

const SMALL_SPLASH_SHP: &str = "GLSS.SHP";
const LARGE_SPLASH_SHP: &str = "GLSL.SHP";
const SPLASH_PALETTE: &str = "GLS.PAL";
const SMALL_SPLASH_SHP_MD: &str = "GLSSMD.SHP";
const LARGE_SPLASH_SHP_MD: &str = "GLSLMD.SHP";
const SPLASH_PALETTE_MD: &str = "GLSMD.PAL";

/// 默认最短展示时长（首次成功 present 后起算；壳层可用 `splash_min_secs` 覆盖）。
pub const DEFAULT_MINIMUM_VISIBLE_SECS: f64 = 3.0;

const TEXT_COLOR: [u8; 4] = [255, 255, 255, 255];

const COPYRIGHT_KEY: &str = "TXT_COPYRIGHT";
const COPYRIGHT_FALLBACK: &str = "© 2000, 2001 ELECTRONIC ARTS INC. ALL RIGHTS RESERVED";
const BRAND_KEY: &str = "GUI:WWBrand";
const BRAND_FALLBACK: &str = "WESTWOOD STUDIOS™ IS AN ELECTRONIC ARTS™ BRAND";
const LOADING_KEY: &str = "GUI:LoadingEx";
const LOADING_FALLBACK: &str = "Loading...";
const TRADEMARK_TOP_KEY: &str = "GUI:TradeMarkTop";
const TRADEMARK_TOP_FALLBACK: &str =
    "Command & Conquer and Red Alert 2 are trademarks or registered";
const TRADEMARK_BOTTOM_KEY: &str = "GUI:TradeMarkBottom";
const TRADEMARK_BOTTOM_FALLBACK: &str =
    "trademarks of Electronic Arts Inc. in the U.S. and/or other countries.";

/// 最短展示期限：仅在首次成功 present 时武装一次。
#[derive(Debug)]
struct VisibleHold {
    minimum: Duration,
    deadline: Option<Instant>,
}

impl VisibleHold {
    fn new(minimum: Duration) -> Self {
        Self {
            minimum,
            deadline: None,
        }
    }

    fn mark_presented(&mut self, now: Instant) {
        if self.deadline.is_none() {
            self.deadline = Some(now + self.minimum);
        }
    }

    /// 未武装时仍视为 active，便于首帧 present 失败后重试。
    fn is_active(&self, now: Instant) -> bool {
        match self.deadline {
            None => true,
            Some(deadline) => now < deadline,
        }
    }
}

/// 进程启动闪屏 presentation（与 `OriginalScreen::Splash` 产品页正交：本结构持有画面与期限）。
pub struct StartupSplashPresentation {
    image: RgbaImage,
    shp_name: &'static str,
    pal_name: &'static str,
    hold: VisibleHold,
}

impl StartupSplashPresentation {
    /// 按客户区宽度选择 SHP，合成黑底居中画面与启动文案。
    ///
    /// `minimum_visible` 为最短展示时长（首次成功 present 后起算）。
    pub fn build(
        source: &GameAssetSource,
        csf: Option<&CsfFile>,
        font: Option<&FntFile>,
        client_width: u32,
        client_height: u32,
        minimum_visible: Duration,
    ) -> Result<Self, String> {
        if client_width == 0 || client_height == 0 {
            return Err("启动闪屏需要非零客户区尺寸".into());
        }
        let (shp_name, pal_name, image) =
            compose_startup_splash(source, csf, font, client_width, client_height)?;
        Ok(Self {
            image,
            shp_name,
            pal_name,
            hold: VisibleHold::new(minimum_visible),
        })
    }

    /// 资源不可用时的黑底占位（仍遵守最短展示）。
    pub fn placeholder(client_width: u32, client_height: u32, minimum_visible: Duration) -> Result<Self, String> {
        if client_width == 0 || client_height == 0 {
            return Err("启动闪屏需要非零客户区尺寸".into());
        }
        let image = opaque_black(client_width, client_height)
            .ok_or_else(|| "启动闪屏占位画布构造失败".to_string())?;
        Ok(Self {
            image,
            shp_name: splash_shp_for_width(client_width),
            pal_name: SPLASH_PALETTE,
            hold: VisibleHold::new(minimum_visible),
        })
    }

    /// 已合成的整页 RGBA（可重复上传 GPU）。
    pub fn image(&self) -> &RgbaImage {
        &self.image
    }

    /// 实际采用的 SHP 逻辑名（诊断 / 标题栏）。
    pub fn shp_name(&self) -> &'static str {
        self.shp_name
    }

    /// 实际采用的调色板逻辑名。
    pub fn pal_name(&self) -> &'static str {
        self.pal_name
    }

    /// 首次成功 present 后调用；重复调用不会重置期限。
    pub fn mark_presented(&mut self, now: Instant) {
        self.hold.mark_presented(now);
    }

    /// 是否仍应挡住主菜单绘制。
    pub fn is_active(&self, now: Instant) -> bool {
        self.hold.is_active(now)
    }
}

/// 按客户区宽度选择启动 SHP：正好 640 用小图，其余用大图。
pub fn splash_shp_for_width(client_width: u32) -> &'static str {
    if client_width == 640 {
        SMALL_SPLASH_SHP
    } else {
        LARGE_SPLASH_SHP
    }
}

fn splash_candidates(client_width: u32) -> [(&'static str, &'static str); 2] {
    let primary = if client_width == 640 {
        (SMALL_SPLASH_SHP, SPLASH_PALETTE)
    } else {
        (LARGE_SPLASH_SHP, SPLASH_PALETTE)
    };
    let md = if client_width == 640 {
        (SMALL_SPLASH_SHP_MD, SPLASH_PALETTE_MD)
    } else {
        (LARGE_SPLASH_SHP_MD, SPLASH_PALETTE_MD)
    };
    [primary, md]
}

fn compose_startup_splash(
    source: &GameAssetSource,
    csf: Option<&CsfFile>,
    font: Option<&FntFile>,
    client_width: u32,
    client_height: u32,
) -> Result<(&'static str, &'static str, RgbaImage), String> {
    let mut canvas = opaque_black(client_width, client_height)
        .ok_or_else(|| "启动闪屏画布构造失败".to_string())?;

    let mut used_shp = splash_shp_for_width(client_width);
    let mut used_pal = SPLASH_PALETTE;
    let mut art_ok = false;

    for (shp_name, pal_name) in splash_candidates(client_width) {
        match try_blit_splash_art(source, &mut canvas, shp_name, pal_name, client_width, client_height) {
            Ok(()) => {
                used_shp = shp_name;
                used_pal = pal_name;
                art_ok = true;
                break;
            }
            Err(err) => {
                tracing::debug!(shp = shp_name, pal = pal_name, %err, "启动闪屏候选未命中");
            }
        }
    }

    if !art_ok {
        tracing::warn!(
            shp = used_shp,
            pal = used_pal,
            "启动闪屏美术不可用 · 仅黑底 + 文案"
        );
    }

    if let Some(fnt) = font {
        overlay_startup_text(&mut canvas, csf, fnt);
    }

    Ok((used_shp, used_pal, canvas))
}

fn try_blit_splash_art(
    source: &GameAssetSource,
    canvas: &mut RgbaImage,
    shp_name: &str,
    pal_name: &str,
    client_width: u32,
    client_height: u32,
) -> Result<(), String> {
    let shp_hit = source
        .resolve(shp_name)
        .ok_or_else(|| format!("{shp_name}: 不可读"))?;
    let pal_hit = source
        .resolve(pal_name)
        .ok_or_else(|| format!("{pal_name}: 不可读"))?;
    let shp = ShpFile::parse(&shp_hit.bytes).map_err(|e| format!("{shp_name}: SHP 解析失败 · {e}"))?;
    let palette = Palette::parse(&pal_hit.bytes).map_err(|e| format!("{pal_name}: 解析失败 · {e}"))?;
    let frame = shp
        .frames
        .first()
        .ok_or_else(|| format!("{shp_name}: SHP 无帧"))?;
    let art = frame_to_canvas_rgba(&shp, frame, &palette)
        .ok_or_else(|| format!("{shp_name}: 画布 RGBA 构造失败"))?;

    // 原版资源固定 640×480 / 800×600；客户区更大时最近邻放大铺满，避免黑边。
    if art.width() == client_width && art.height() == client_height {
        blit_rgba(canvas, &art, 0, 0);
    } else {
        blit_nearest_fill(canvas, &art);
    }
    Ok(())
}

/// 将 `src` 最近邻拉伸铺满整个 `dst`。
fn blit_nearest_fill(dst: &mut RgbaImage, src: &RgbaImage) {
    let dw = dst.width();
    let dh = dst.height();
    let sw = src.width();
    let sh = src.height();
    if dw == 0 || dh == 0 || sw == 0 || sh == 0 {
        return;
    }
    let dst_raw = dst.as_mut();
    let src_raw = src.as_raw();
    for dy in 0..dh {
        let sy = ((dy as u64 * sh as u64) / dh as u64) as u32;
        let sy = sy.min(sh - 1);
        for dx in 0..dw {
            let sx = ((dx as u64 * sw as u64) / dw as u64) as u32;
            let sx = sx.min(sw - 1);
            let si = ((sy * sw + sx) * 4) as usize;
            let di = ((dy * dw + dx) * 4) as usize;
            dst_raw[di..di + 4].copy_from_slice(&src_raw[si..si + 4]);
        }
    }
}

fn overlay_startup_text(canvas: &mut RgbaImage, csf: Option<&CsfFile>, font: &FntFile) {
    let copyright = csf_text(csf, COPYRIGHT_KEY, COPYRIGHT_FALLBACK);
    let brand = csf_text(csf, BRAND_KEY, BRAND_FALLBACK);
    let loading = csf_text(csf, LOADING_KEY, LOADING_FALLBACK);
    let trademark_top = csf_text(csf, TRADEMARK_TOP_KEY, TRADEMARK_TOP_FALLBACK);
    let trademark_bottom = csf_text(csf, TRADEMARK_BOTTOM_KEY, TRADEMARK_BOTTOM_FALLBACK);

    let client_width = canvas.width() as i32;
    let client_height = canvas.height() as i32;
    let first_bottom_y = client_height - 40;
    let second_bottom_y = first_bottom_y + 3 + font.cell_height as i32;

    blit_text_colored(
        canvas,
        font,
        &copyright,
        client_width - font.text_width(&copyright) as i32 - 10,
        first_bottom_y,
        TEXT_COLOR,
    );
    blit_text_colored(
        canvas,
        font,
        &brand,
        client_width - font.text_width(&brand) as i32 - 10,
        second_bottom_y,
        TEXT_COLOR,
    );
    blit_text_colored(canvas, font, &loading, 10, 10, TEXT_COLOR);
    blit_text_colored(canvas, font, &trademark_top, 10, first_bottom_y, TEXT_COLOR);
    blit_text_colored(canvas, font, &trademark_bottom, 10, second_bottom_y, TEXT_COLOR);
}

fn opaque_black(width: u32, height: u32) -> Option<RgbaImage> {
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    for px in pixels.chunks_exact_mut(4) {
        px[3] = 255;
    }
    RgbaImage::from_raw(width, height, pixels)
}

fn centered_offset(client_extent: i32, art_extent: i32) -> i32 {
    (client_extent - art_extent) / 2
}

fn csf_text(csf: Option<&CsfFile>, key: &str, fallback: &str) -> String {
    match csf.and_then(|table| table.get(key)) {
        Some(text) => text.to_string(),
        None => fallback.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_640_selects_small_every_other_width_selects_large() {
        assert_eq!(splash_shp_for_width(640), SMALL_SPLASH_SHP);
        assert_eq!(splash_shp_for_width(639), LARGE_SPLASH_SHP);
        assert_eq!(splash_shp_for_width(800), LARGE_SPLASH_SHP);
        assert_eq!(splash_shp_for_width(1920), LARGE_SPLASH_SHP);
    }

    #[test]
    fn nearest_fill_covers_destination() {
        let mut dst = opaque_black(4, 2).unwrap();
        let src = RgbaImage::from_raw(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255]).unwrap();
        blit_nearest_fill(&mut dst, &src);
        assert_eq!(&dst.as_raw()[0..4], &[10, 20, 30, 255]);
        assert_eq!(&dst.as_raw()[4..8], &[10, 20, 30, 255]);
        assert_eq!(&dst.as_raw()[8..12], &[40, 50, 60, 255]);
        assert_eq!(&dst.as_raw()[12..16], &[40, 50, 60, 255]);
    }

    #[test]
    fn hold_anchors_at_first_present_and_never_rearms() {
        let start = Instant::now();
        let minimum = Duration::from_secs_f64(DEFAULT_MINIMUM_VISIBLE_SECS);
        let mut hold = VisibleHold::new(minimum);
        assert!(hold.is_active(start + Duration::from_secs(600)));

        hold.mark_presented(start);
        hold.mark_presented(start + Duration::from_secs(2));
        hold.mark_presented(start + minimum);

        assert!(hold.is_active(start));
        assert!(hold.is_active(start + minimum - Duration::from_millis(1)));
        assert!(!hold.is_active(start + minimum));
        assert!(!hold.is_active(start + minimum + Duration::from_secs(1)));
    }

    #[test]
    fn hold_respects_custom_minimum() {
        let start = Instant::now();
        let mut hold = VisibleHold::new(Duration::from_secs(1));
        hold.mark_presented(start);
        assert!(hold.is_active(start + Duration::from_millis(999)));
        assert!(!hold.is_active(start + Duration::from_secs(1)));
    }

    #[test]
    fn missing_csf_uses_english_fallback() {
        assert_eq!(
            csf_text(None, COPYRIGHT_KEY, COPYRIGHT_FALLBACK),
            COPYRIGHT_FALLBACK
        );
    }
}
