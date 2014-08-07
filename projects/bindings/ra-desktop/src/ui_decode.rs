//! 页面资源解码：`UiAssetRef` → 画布 RGBA。
//!
//! 解码成功 ≠ 已上传 GPU ≠ Pre-Alpha 视觉交付。
//! 帧像素按 SHP 画布尺寸放置（`frame_x` / `frame_y`），不任意拉伸。

use ra_assets::{Palette, ShpFile, ShpFrame};
use ra_renderer::RgbaImage;

use crate::{
    fs_source::GameAssetSource,
    ui_page::{UiAssetRef, UiButtonVisualState, UiPageResources},
};

/// 一帧已解码精灵（含诊断元数据）。
#[derive(Debug, Clone)]
pub struct DecodedUiSprite {
    /// 诊断标签（通常为 `name#frame`）。
    pub label: String,
    /// 画布尺寸 RGBA。
    pub image: RgbaImage,
    /// 来源说明（与 `GameAssetSource::resolve` 对齐）。
    pub origin: String,
    /// 使用的帧号。
    pub frame: u16,
    /// SHP 画布宽高。
    pub canvas: (u16, u16),
    /// 帧在画布上的矩形：x, y, w, h。
    pub frame_rect: (u16, u16, u16, u16),
}

/// 一页 chrome 解码结果。
#[derive(Debug, Clone)]
pub struct PageDecodeReport {
    /// 背景（若声明且成功）。
    pub background: Option<DecodedUiSprite>,
    /// 面板（声明顺序；失败项不进入本列表）。
    pub panels: Vec<DecodedUiSprite>,
    /// `(entry_id, 常态图)`；仅成功项。
    pub button_normals: Vec<(&'static str, DecodedUiSprite)>,
    /// `(entry_id, 悬停图)`；仅成功项（缺省时合成回退常态）。
    pub button_hovers: Vec<(&'static str, DecodedUiSprite)>,
    /// `(entry_id, 按下图)`；仅成功项（缺省时合成回退常态）。
    pub button_presseds: Vec<(&'static str, DecodedUiSprite)>,
    /// 失败说明。
    pub errors: Vec<String>,
}

impl PageDecodeReport {
    /// 标题栏 / 日志短注。
    pub fn banner_note(&self) -> String {
        let ok = usize::from(self.background.is_some())
            + self.panels.len()
            + self.button_normals.len()
            + self.button_hovers.len()
            + self.button_presseds.len();
        if self.errors.is_empty() {
            format!("UI 解码 ok · {ok} 张")
        }
        else {
            format!("UI 解码 {} 张 · 失败 {}", ok, self.errors.len())
        }
    }

    /// 背景与每个可点按钮常态是否都已解码。
    pub fn chrome_ready_for_enabled_buttons(&self, page: &UiPageResources) -> bool {
        if self.background.is_none() {
            return false;
        }
        for btn in page.buttons.iter().filter(|b| b.enabled) {
            if !self.button_normals.iter().any(|(id, _)| *id == btn.entry_id) {
                return false;
            }
        }
        true
    }
}

/// 将 SHP 单帧铺到画布尺寸 RGBA（透明底）。
pub fn frame_to_canvas_rgba(shp: &ShpFile, frame: &ShpFrame, palette: &Palette) -> Option<RgbaImage> {
    let canvas_w = u32::from(shp.width);
    let canvas_h = u32::from(shp.height);
    if canvas_w == 0 || canvas_h == 0 {
        return None;
    }
    let fw = u32::from(frame.frame_width);
    let fh = u32::from(frame.frame_height);
    if fw == 0 || fh == 0 {
        // 空帧：仍返回全透明画布，便于布局占位诊断。
        return RgbaImage::from_raw(canvas_w, canvas_h, vec![0u8; (canvas_w * canvas_h * 4) as usize]);
    }

    let frame_rgba = frame.to_rgba(palette);
    if fw == canvas_w && fh == canvas_h && frame.frame_x == 0 && frame.frame_y == 0 {
        return RgbaImage::from_raw(canvas_w, canvas_h, frame_rgba);
    }

    let mut canvas = vec![0u8; (canvas_w as usize) * (canvas_h as usize) * 4];
    let fx = u32::from(frame.frame_x);
    let fy = u32::from(frame.frame_y);
    for row in 0..fh {
        let dst_y = fy + row;
        if dst_y >= canvas_h {
            break;
        }
        let src = (row * fw * 4) as usize;
        let dst = ((dst_y * canvas_w + fx) * 4) as usize;
        let row_bytes = (fw * 4) as usize;
        if src + row_bytes > frame_rgba.len() {
            break;
        }
        let max_copy = ((canvas_w - fx) * 4) as usize;
        let copy_len = row_bytes.min(max_copy);
        if dst + copy_len > canvas.len() {
            break;
        }
        canvas[dst..dst + copy_len].copy_from_slice(&frame_rgba[src..src + copy_len]);
    }
    RgbaImage::from_raw(canvas_w, canvas_h, canvas)
}

/// 从挂载源解码单个 `UiAssetRef`。
pub fn decode_asset_ref(source: &GameAssetSource, asset: &UiAssetRef) -> Result<DecodedUiSprite, String> {
    let frame_idx = asset.frame.unwrap_or(0) as usize;
    let mut frames = decode_asset_frames(source, asset)?;
    if frame_idx >= frames.len() {
        return Err(format!("{}: 帧 {} 越界 · 共 {} 帧", asset.name, frame_idx, frames.len()));
    }
    Ok(frames.swap_remove(frame_idx))
}

/// 解码 `UiAssetRef` 指向 SHP 的全部帧（面板动画用）。
pub fn decode_asset_frames(source: &GameAssetSource, asset: &UiAssetRef) -> Result<Vec<DecodedUiSprite>, String> {
    let hit = source.resolve(&asset.name).ok_or_else(|| format!("{}: 不可读", asset.name))?;
    let shp = ShpFile::parse(&hit.bytes).map_err(|e| format!("{}: SHP 解析失败 · {e}", asset.name))?;
    if shp.frames.is_empty() {
        return Err(format!("{}: SHP 无帧", asset.name));
    }
    let pal_name = asset.palette.as_deref().ok_or_else(|| format!("{}: 未指定调色板", asset.name))?;
    let pal_hit = source.resolve(pal_name).ok_or_else(|| format!("{pal_name}: 调色板不可读"))?;
    let palette = Palette::parse(&pal_hit.bytes).map_err(|e| format!("{pal_name}: 解析失败 · {e}"))?;
    let origin = hit.explain();
    let mut out = Vec::with_capacity(shp.frames.len());
    for (frame_idx, frame) in shp.frames.iter().enumerate() {
        let image = frame_to_canvas_rgba(&shp, frame, &palette)
            .ok_or_else(|| format!("{}#{}: 画布 RGBA 构造失败", asset.name, frame_idx))?;
        out.push(DecodedUiSprite {
            label: format!("{}#{}", asset.name, frame_idx),
            image,
            origin: origin.clone(),
            frame: frame_idx as u16,
            canvas: (shp.width, shp.height),
            frame_rect: (frame.frame_x, frame.frame_y, frame.frame_width, frame.frame_height),
        });
    }
    Ok(out)
}

/// 解码一页已声明的背景、面板与可点按钮常态（及禁用按钮若有 disabled/normal）。
pub fn decode_page_chrome(source: &GameAssetSource, page: &UiPageResources) -> PageDecodeReport {
    let mut errors = Vec::new();
    let mut background = None;
    if let Some(bg) = &page.background {
        match decode_asset_ref(source, bg) {
            Ok(img) => background = Some(img),
            Err(e) => errors.push(format!("background · {e}")),
        }
    }

    let mut panels = Vec::new();
    for (i, panel) in page.panels.iter().enumerate() {
        // 面板可能含多帧（如 `sdtp.shp` 右上角 WARNING 指示）；全部解出供壳层循环。
        match decode_asset_frames(source, panel) {
            Ok(frames) => panels.extend(frames),
            Err(e) => errors.push(format!("panel[{i}] · {e}")),
        }
    }

    let mut button_normals = Vec::new();
    let mut button_hovers = Vec::new();
    let mut button_presseds = Vec::new();
    for btn in &page.buttons {
        let asset = if btn.enabled {
            btn.asset_for(UiButtonVisualState::Normal)
        }
        else {
            btn.asset_for(UiButtonVisualState::Disabled).or_else(|| btn.asset_for(UiButtonVisualState::Normal))
        };
        let Some(asset) = asset
        else {
            if btn.enabled {
                errors.push(format!("button[{}] · 无常态资源", btn.entry_id));
            }
            continue;
        };
        match decode_asset_ref(source, asset) {
            Ok(img) => button_normals.push((btn.entry_id, img)),
            Err(e) => errors.push(format!("button[{}] · {e}", btn.entry_id)),
        }

        if !btn.enabled {
            continue;
        }
        if let Some(hover) = btn.hover.as_ref() {
            if btn.normal.as_ref() != Some(hover) {
                match decode_asset_ref(source, hover) {
                    Ok(img) => button_hovers.push((btn.entry_id, img)),
                    Err(e) => errors.push(format!("button[{}] hover · {e}", btn.entry_id)),
                }
            }
        }
        let Some(pressed) = btn.pressed.as_ref()
        else {
            continue;
        };
        // 与常态同引用则不必重复解码。
        if btn.normal.as_ref() == Some(pressed) {
            continue;
        }
        match decode_asset_ref(source, pressed) {
            Ok(img) => button_presseds.push((btn.entry_id, img)),
            Err(e) => errors.push(format!("button[{}] pressed · {e}", btn.entry_id)),
        }
    }

    PageDecodeReport { background, panels, button_normals, button_hovers, button_presseds, errors }
}
