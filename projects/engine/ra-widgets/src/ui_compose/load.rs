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

/// 合成进战斗装载页：国家 `ls*` 全幅 + CSF 文案 + 中下 `progbarm`；失败时重试/取消。
///
/// 遭遇战与战役共用本合成入口；战役简报外观后续按 [`crate::LoadKind`] 分支。
/// 文案与按钮几何来自 `load_screen_layout`（`load_screen_layout_tree` 投影）。
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
    let layout = load_screen_layout(viewport_w, viewport_h);
    let mut page = RgbaImage::from_raw(
        layout.canvas.w as u32,
        layout.canvas.h as u32,
        vec![0u8; (layout.canvas.w as usize) * (layout.canvas.h as usize) * 4],
    )?;
    fill_rect(&mut page, layout.canvas, [0, 0, 0, 255]);
    if let Some(bg) = decoded.background.as_ref() {
        blit_stretched(&mut page, &bg.image, layout.canvas);
    }

    if let Some(fnt) = fnt {
        let special_key = load_screen_special_unit_csf_key(paint.side);
        if let Some(special) = resolve_csf_text(csf, special_key) {
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                &special,
                layout.special.x,
                layout.special.y,
                layout.special.w,
                layout.special.h,
                LOAD_SCREEN_TEXT_TITLE,
            );
        }

        let brief_key = load_screen_brief_csf_key(paint.side);
        if let Some(brief) = resolve_csf_text(csf, &brief_key) {
            blit_caption_wrapped(
                &mut page,
                fnt,
                &brief,
                layout.brief.x,
                layout.brief.y,
                layout.brief.w,
                layout.brief.h,
                LOAD_SCREEN_TEXT,
            );
        }

        let name_key = load_screen_name_csf_key(paint.side);
        if let Some(name) = resolve_csf_text(csf, &name_key) {
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                &name,
                layout.name.x,
                layout.name.y,
                layout.name.w,
                layout.name.h,
                LOAD_SCREEN_TEXT_TITLE,
            );
        }

        if !paint.allow_retry {
            let loading =
                resolve_csf_text(csf, load_screen_loading_csf_key()).unwrap_or_else(|| "Loading..".into());
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                &loading,
                layout.status.x,
                layout.status.y,
                layout.status.w,
                layout.status.h,
                LOAD_SCREEN_TEXT,
            );
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                paint.player_name,
                layout.player_name.x,
                layout.player_name.y,
                layout.player_name.w,
                layout.player_name.h,
                [80, 220, 80, 255],
            );
        }
    }

    if !paint.allow_retry {
        if let Some(flag) = paint.side_flag {
            blit_rgba(
                &mut page,
                flag,
                layout.player_flag.x,
                layout.player_flag.y,
            );
        }
    }

    let ratio = paint.progress.clamp(0.0, 1.0);
    if let Some(bar) = find_panel(decoded, "progbarm.shp", 0) {
        let clip_w = ((bar.image.width() as f32) * ratio).round() as u32;
        blit_rgba_clipped_width(
            &mut page,
            &bar.image,
            layout.progress.x,
            layout.progress.y,
            clip_w,
        );
    }

    // 失败时只露操作钮；不再叠中区假对话框（状态在窗口标题）。
    if paint.allow_retry {
        for (i, entry_id) in LOAD_SCREEN_BUTTON_IDS.iter().enumerate() {
            let rect = layout.buttons[i];
            let enabled = *entry_id != "retry" || paint.allow_retry;
            let normal = find_button_normal(decoded, entry_id);
            let sprite = if !enabled {
                find_button_pressed(decoded, entry_id).or(normal)
            } else if pressed_entry_id == Some(*entry_id) {
                find_button_pressed(decoded, entry_id).or(normal)
            } else if hovered_entry_id == Some(*entry_id) {
                find_button_hover(decoded, entry_id).or(normal)
            } else {
                normal
            };
            if let Some(sprite) = sprite {
                let bx = rect.x + (rect.w - sprite.image.width() as i32) / 2;
                let by = rect.y + (rect.h - sprite.image.height() as i32) / 2;
                blit_rgba(&mut page, &sprite.image, bx, by);
                if let Some(fnt) = fnt {
                    let label = if *entry_id == "retry" { "重试" } else { "取消" };
                    let color = if enabled {
                        MENU_TEXT_ENABLED
                    } else {
                        MENU_TEXT_DISABLED
                    };
                    let pressed = enabled && pressed_entry_id == Some(*entry_id);
                    let cell = RectPx::new(
                        bx,
                        by,
                        sprite.image.width() as i32,
                        sprite.image.height() as i32,
                    );
                    let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
                    blit_caption_in_cell(&mut page, fnt, label, tx, ty, tw, th, color);
                }
                if !enabled {
                    dim_rect(&mut page, rect, 110);
                }
            } else {
                let fill = if enabled {
                    [120, 24, 24, 255]
                } else {
                    [48, 40, 40, 255]
                };
                fill_rect(&mut page, rect, fill);
                stroke_rect(&mut page, rect, [200, 40, 40, 255]);
                if let Some(fnt) = fnt {
                    let label = if *entry_id == "retry" { "重试" } else { "取消" };
                    let color = if enabled {
                        MENU_TEXT_ENABLED
                    } else {
                        MENU_TEXT_DISABLED
                    };
                    blit_caption_in_cell(
                        &mut page,
                        fnt,
                        label,
                        rect.x + 8,
                        rect.y + 8,
                        rect.w - 16,
                        rect.h - 16,
                        color,
                    );
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
