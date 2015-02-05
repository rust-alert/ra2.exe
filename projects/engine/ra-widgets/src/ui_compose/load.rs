//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub struct LoadScreenPaint<'a> {
    /// 本地阵营短名（驱动 `NAME:` / `LOADBRIEF:` CSF 键）。
    pub side: &'a str,
    /// 本地玩家名（进度条右侧槽）。
    pub player_name: &'a str,
    /// 阵营小旗（`usai.pcx` 等，进度条右侧）。
    pub side_flag: Option<&'a RgbaImage>,
    /// 底栏状态（装载中或失败说明；失败时才强调）。
    pub status: &'a str,
    /// 是否允许「重试」（装载线程进行中为 false；失败后为 true）。
    pub allow_retry: bool,
    /// 装载进度 0..=1（驱动 `progbarm` 横向裁剪）。
    pub progress: f32,
}

/// 800×600 基准：对照原版截图像素映射。
pub(super) const LOAD_SPECIAL_X_800: i32 = 54;
pub(super) const LOAD_SPECIAL_Y_800: i32 = 106;
pub(super) const LOAD_SPECIAL_W_800: i32 = 190;
pub(super) const LOAD_SPECIAL_H_800: i32 = 22;
pub(super) const LOAD_BRIEF_X_800: i32 = 48;
pub(super) const LOAD_BRIEF_Y_800: i32 = 134;
pub(super) const LOAD_BRIEF_W_800: i32 = 340;
pub(super) const LOAD_BRIEF_H_800: i32 = 200;
pub(super) const LOAD_NAME_X_800: i32 = 648;
pub(super) const LOAD_NAME_Y_800: i32 = 538;
pub(super) const LOAD_NAME_W_800: i32 = 120;
pub(super) const LOAD_NAME_H_800: i32 = 24;
pub(super) const LOAD_STATUS_X_800: i32 = 56;
pub(super) const LOAD_STATUS_Y_800: i32 = 310;
pub(super) const LOAD_STATUS_W_800: i32 = 160;
pub(super) const LOAD_STATUS_H_800: i32 = 20;
pub(super) const LOAD_PROG_X_800: i32 = 56;
pub(super) const LOAD_PROG_Y_800: i32 = 332;
pub(super) const LOAD_PLAYER_FLAG_X_800: i32 = 150;
pub(super) const LOAD_PLAYER_FLAG_Y_800: i32 = 324;
pub(super) const LOAD_PLAYER_NAME_X_800: i32 = 202;
pub(super) const LOAD_PLAYER_NAME_Y_800: i32 = 328;
pub(super) const LOAD_PLAYER_NAME_W_800: i32 = 120;
pub(super) const LOAD_PLAYER_NAME_H_800: i32 = 20;

/// 合成遭遇战装载页：国家 `ls*` 全幅 + CSF 文案 + 中下 `progbarm`；失败时重试/取消。
pub fn compose_load_screen_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    paint: LoadScreenPaint<'_>,
) -> Option<RgbaImage> {
    let layout = main_menu_layout(viewport_w, viewport_h);
    let mut page = RgbaImage::from_raw(
        layout.canvas.w as u32,
        layout.canvas.h as u32,
        vec![0u8; (layout.canvas.w as usize) * (layout.canvas.h as usize) * 4],
    )?;
    fill_rect(&mut page, layout.canvas, [0, 0, 0, 255]);
    if let Some(bg) = decoded.background.as_ref() {
        blit_stretched(&mut page, &bg.image, layout.canvas);
    }

    let sx = layout.canvas.w as f32 / 800.0;
    let sy = layout.canvas.h as f32 / 600.0;
    let scale_box = |x: i32, y: i32, w: i32, h: i32| {
        (
            layout.canvas.x + (x as f32 * sx) as i32,
            layout.canvas.y + (y as f32 * sy) as i32,
            ((w as f32 * sx) as i32).max(1),
            ((h as f32 * sy) as i32).max(1),
        )
    };

    if let Some(fnt) = fnt {
        let special_key = load_screen_special_unit_csf_key(paint.side);
        if let Some(special) = resolve_csf_text(csf, special_key) {
            let (x, y, w, h) = scale_box(LOAD_SPECIAL_X_800, LOAD_SPECIAL_Y_800, LOAD_SPECIAL_W_800, LOAD_SPECIAL_H_800);
            blit_caption_top_left_clipped(&mut page, fnt, &special, x, y, w, h, LOAD_SCREEN_TEXT_TITLE);
        }

        let brief_key = load_screen_brief_csf_key(paint.side);
        if let Some(brief) = resolve_csf_text(csf, &brief_key) {
            let (x, y, w, h) = scale_box(LOAD_BRIEF_X_800, LOAD_BRIEF_Y_800, LOAD_BRIEF_W_800, LOAD_BRIEF_H_800);
            blit_caption_wrapped(&mut page, fnt, &brief, x, y, w, h, LOAD_SCREEN_TEXT);
        }

        let name_key = load_screen_name_csf_key(paint.side);
        if let Some(name) = resolve_csf_text(csf, &name_key) {
            let (x, y, w, h) = scale_box(LOAD_NAME_X_800, LOAD_NAME_Y_800, LOAD_NAME_W_800, LOAD_NAME_H_800);
            blit_caption_top_left_clipped(&mut page, fnt, &name, x, y, w, h, LOAD_SCREEN_TEXT_TITLE);
        }

        if !paint.allow_retry {
            let loading = resolve_csf_text(csf, load_screen_loading_csf_key()).unwrap_or_else(|| "Loading..".into());
            let (x, y, w, h) = scale_box(LOAD_STATUS_X_800, LOAD_STATUS_Y_800, LOAD_STATUS_W_800, LOAD_STATUS_H_800);
            blit_caption_top_left_clipped(&mut page, fnt, &loading, x, y, w, h, LOAD_SCREEN_TEXT);
            let (nx, ny, nw, nh) =
                scale_box(LOAD_PLAYER_NAME_X_800, LOAD_PLAYER_NAME_Y_800, LOAD_PLAYER_NAME_W_800, LOAD_PLAYER_NAME_H_800);
            blit_caption_top_left_clipped(&mut page, fnt, paint.player_name, nx, ny, nw, nh, [80, 220, 80, 255]);
        }
    }

    if !paint.allow_retry {
        if let Some(flag) = paint.side_flag {
            let fx = layout.canvas.x + (LOAD_PLAYER_FLAG_X_800 as f32 * sx) as i32;
            let fy = layout.canvas.y + (LOAD_PLAYER_FLAG_Y_800 as f32 * sy) as i32;
            blit_rgba(&mut page, flag, fx, fy);
        }
    }

    let ratio = paint.progress.clamp(0.0, 1.0);
    if let Some(bar) = find_panel(decoded, "progbarm.shp", 0) {
        let x = layout.canvas.x + (LOAD_PROG_X_800 as f32 * sx) as i32;
        let y = layout.canvas.y + (LOAD_PROG_Y_800 as f32 * sy) as i32;
        let clip_w = ((bar.image.width() as f32) * ratio).round() as u32;
        blit_rgba_clipped_width(&mut page, &bar.image, x, y, clip_w);
    }

    // 失败时只露操作钮；不再叠中区假对话框（状态在窗口标题）。
    if paint.allow_retry {
        for entry_id in ["retry", "cancel"] {
            let Some(slot) = slots_load_button_hit(entry_id)
            else {
                continue;
            };
            let enabled = entry_id != "retry" || paint.allow_retry;
            let rect = hit_to_rect(layout.canvas, slot);
            let normal = find_button_normal(decoded, entry_id);
            let sprite = if !enabled {
                find_button_pressed(decoded, entry_id).or(normal)
            }
            else if pressed_entry_id == Some(entry_id) {
                find_button_pressed(decoded, entry_id).or(normal)
            }
            else if hovered_entry_id == Some(entry_id) {
                find_button_hover(decoded, entry_id).or(normal)
            }
            else {
                normal
            };
            if let Some(sprite) = sprite {
                let bx = rect.x + (rect.w - sprite.image.width() as i32) / 2;
                let by = rect.y + (rect.h - sprite.image.height() as i32) / 2;
                blit_rgba(&mut page, &sprite.image, bx, by);
                if let Some(fnt) = fnt {
                    let label = if entry_id == "retry" { "重试" } else { "取消" };
                    let color = if enabled { MENU_TEXT_ENABLED } else { MENU_TEXT_DISABLED };
                    let pressed = enabled && pressed_entry_id == Some(entry_id);
                    let cell = RectPx::new(bx, by, sprite.image.width() as i32, sprite.image.height() as i32);
                    let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
                    blit_caption_in_cell(&mut page, fnt, label, tx, ty, tw, th, color);
                }
                if !enabled {
                    dim_rect(&mut page, rect, 110);
                }
            }
            else {
                let fill = if enabled { [120, 24, 24, 255] } else { [48, 40, 40, 255] };
                fill_rect(&mut page, rect, fill);
                stroke_rect(&mut page, rect, [200, 40, 40, 255]);
                if let Some(fnt) = fnt {
                    let label = if entry_id == "retry" { "重试" } else { "取消" };
                    let color = if enabled { MENU_TEXT_ENABLED } else { MENU_TEXT_DISABLED };
                    blit_caption_in_cell(&mut page, fnt, label, rect.x + 8, rect.y + 8, rect.w - 16, rect.h - 16, color);
                }
            }
        }
    }

    Some(page)
}

pub(super) fn blit_rgba_clipped_width(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, clip_w: u32) {
    let w = clip_w.min(src.width());
    if w == 0 || src.height() == 0 {
        return;
    }
    for row in 0..src.height() {
        let dy = y + row as i32;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..w {
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
            dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
        }
    }
}

pub(super) fn slots_load_button_hit(entry_id: &str) -> Option<(f32, f32, f32, f32)> {
    match entry_id {
        "retry" => Some((0.30, 0.88, 0.50, 0.96)),
        "cancel" => Some((0.54, 0.88, 0.74, 0.96)),
        _ => None,
    }
}

pub(super) fn hit_to_rect(canvas: RectPx, hit: (f32, f32, f32, f32)) -> RectPx {
    let x0 = canvas.x + (canvas.w as f32 * hit.0) as i32;
    let y0 = canvas.y + (canvas.h as f32 * hit.1) as i32;
    let x1 = canvas.x + (canvas.w as f32 * hit.2) as i32;
    let y1 = canvas.y + (canvas.h as f32 * hit.3) as i32;
    RectPx::new(x0, y0, (x1 - x0).max(1), (y1 - y0).max(1))
}
