//! 将 [`RenderPlan`] 占位命令栅格化到 RGBA（调试 / 未绑资源路径）。

use ra_layout::Rect;
use ra_renderer::RgbaImage;

use super::plan::{RenderCommand, RenderPlan};

impl RenderPlan {
    /// 把 `SolidRect` 命令画进已有图像（越界裁剪；透明命令跳过）。
    pub fn paint_solids_into(&self, dst: &mut RgbaImage) {
        for cmd in &self.commands {
            match cmd {
                RenderCommand::SolidRect { rect, color, .. } => {
                    if color[3] == 0 {
                        continue;
                    }
                    fill_rect_f(dst, *rect, *color);
                }
                // 精灵槽留给绑资源 present；诊断栅格化不画假色块。
                RenderCommand::SpriteRect { .. } => {}
            }
        }
    }

    /// 按 `SpriteRect.slot` 解析精灵并 1:1 贴到命令矩形原点（alpha 混合）。
    pub fn paint_sprites_into<'a, F>(&self, dst: &mut RgbaImage, mut resolve: F)
    where
        F: FnMut(&str) -> Option<&'a RgbaImage>,
    {
        for cmd in &self.commands {
            match cmd {
                RenderCommand::SpriteRect { rect, slot, .. } => {
                    let Some(src) = resolve(slot.as_str())
                    else {
                        continue;
                    };
                    blit_rgba_1to1(dst, src, rect.x.round() as i32, rect.y.round() as i32);
                }
                RenderCommand::SolidRect { .. } => {}
            }
        }
    }

    /// 新建透明画布并绘制全部 `SolidRect` 占位。
    pub fn rasterize_solids(&self, width: u32, height: u32) -> Option<RgbaImage> {
        let w = width.max(1);
        let h = height.max(1);
        let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;
        self.paint_solids_into(&mut page);
        Some(page)
    }
}

fn fill_rect_f(dst: &mut RgbaImage, rect: Rect, rgba: [u8; 4]) {
    let x0 = rect.x.floor() as i32;
    let y0 = rect.y.floor() as i32;
    let x1 = (rect.x + rect.width).ceil() as i32;
    let y1 = (rect.y + rect.height).ceil() as i32;
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let left = x0.max(0);
    let top = y0.max(0);
    let right = x1.min(dw);
    let bottom = y1.min(dh);
    if left >= right || top >= bottom {
        return;
    }
    for y in top..bottom {
        for x in left..right {
            let di = ((y as u32 * dst.width() + x as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&rgba);
        }
    }
}

fn blit_rgba_1to1(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
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
            let sa = src.as_raw()[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
                continue;
            }
            let inv = 255 - sa;
            for c in 0..3 {
                let s = src.as_raw()[si + c] as u32;
                let d = dst.as_mut()[di + c] as u32;
                dst.as_mut()[di + c] = ((s * sa + d * inv) / 255) as u8;
            }
            dst.as_mut()[di + 3] = 255;
        }
    }
}
