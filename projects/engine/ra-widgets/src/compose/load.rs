//! 装载页合成。

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
    /// 简报 CSF 覆盖（战役 `DESC:*`）；`None` 时按 `side` 走国家 `LOADBRIEF`。
    pub brief_csf_override: Option<&'a str>,
    /// 特色兵种 CSF 键（来自 [`ra_assets::CountryDef::special_ui_name`]，经资源链 rules）。
    ///
    /// `None` / 空串：不画特色名。原版与模组都允许缺失，禁止回退国家→兵种写死表。
    pub special_ui_name: Option<&'a str>,
}

/// 合成进战斗装载页：国家 `ls*` 全幅 + CSF 文案 + 中下 `progbarm`；失败时重试/取消。
///
/// 遭遇战与战役共用本合成入口；战役简报外观后续按 [`crate::LoadKind`] 分支。
/// 文案与按钮几何来自 `solve_load_screen` snapshot。
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
    let _ = (viewport_w, viewport_h);
    let snap = ra_layout::solve_load_screen();
    let canvas = RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H);
    let special = rect_px_from_snapshot(&snap, "special");
    let brief = rect_px_from_snapshot(&snap, "brief");
    let name = rect_px_from_snapshot(&snap, "name");
    let status = rect_px_from_snapshot(&snap, "status");
    let progress = rect_px_from_snapshot(&snap, "progress");
    let player_flag = rect_px_from_snapshot(&snap, "player_flag");
    let player_name = rect_px_from_snapshot(&snap, "player_name");

    let mut page = RgbaImage::from_raw(canvas.w as u32, canvas.h as u32, vec![0u8; (canvas.w as usize) * (canvas.h as usize) * 4])?;
    fill_rect(&mut page, canvas, [0, 0, 0, 255]);
    if let Some(bg) = decoded.background.as_ref() {
        blit_stretched(&mut page, &bg.image, canvas);
    }

    if let Some(fnt) = fnt {
        // 特色名：rules 派生 UIName → 胜出 CSF。键空或 CSF 无文案则整行省略。
        if let Some(special_key) = paint.special_ui_name.map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(special_text) = resolve_csf_text(csf, special_key) {
                blit_caption_top_left_clipped(
                    &mut page,
                    fnt,
                    &special_text,
                    special.x,
                    special.y,
                    special.w,
                    special.h,
                    LOAD_SCREEN_TEXT_TITLE,
                );
            }
        }

        let brief_key = paint.brief_csf_override.map(str::to_string).unwrap_or_else(|| load_screen_brief_csf_key(paint.side));
        if let Some(brief_text) = resolve_csf_text(csf, &brief_key) {
            blit_caption_wrapped(&mut page, fnt, &brief_text, brief.x, brief.y, brief.w, brief.h, LOAD_SCREEN_TEXT);
        }

        let name_key = load_screen_name_csf_key(paint.side);
        if let Some(name_text) = resolve_csf_text(csf, &name_key) {
            blit_caption_top_left_clipped(&mut page, fnt, &name_text, name.x, name.y, name.w, name.h, LOAD_SCREEN_TEXT_TITLE);
        }

        if !paint.allow_retry {
            let loading = resolve_csf_text(csf, load_screen_loading_csf_key()).unwrap_or_else(|| "Loading..".into());
            blit_caption_top_left_clipped(&mut page, fnt, &loading, status.x, status.y, status.w, status.h, LOAD_SCREEN_TEXT);
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                paint.player_name,
                player_name.x,
                player_name.y,
                player_name.w,
                player_name.h,
                [80, 220, 80, 255],
            );
        }
    }

    if !paint.allow_retry {
        if let Some(flag) = paint.side_flag {
            blit_rgba(&mut page, flag, player_flag.x, player_flag.y);
        }
    }

    let ratio = paint.progress.clamp(0.0, 1.0);
    if let Some(bar) = find_panel(decoded, "progbarm.shp", 0) {
        let clip_w = ((bar.image.width() as f32) * ratio).round() as u32;
        blit_rgba_clipped_width(&mut page, &bar.image, progress.x, progress.y, clip_w);
    }

    // 失败时只露操作钮；不再叠中区假对话框（状态在窗口标题）。
    if paint.allow_retry {
        let btn_plan = crate::RenderPlan::load_screen_placeholders().button_sprite_plan(&LOAD_SCREEN_BUTTON_IDS);
        for entry_id in LOAD_SCREEN_BUTTON_IDS.iter() {
            let Some(rect) = btn_plan.rect_px_of(entry_id)
            else {
                continue;
            };
            let enabled = *entry_id != "retry" || paint.allow_retry;
            let normal = find_button_normal(decoded, entry_id);
            let sprite = if !enabled {
                find_button_pressed(decoded, entry_id).or(normal)
            }
            else if pressed_entry_id == Some(*entry_id) {
                find_button_pressed(decoded, entry_id).or(normal)
            }
            else if hovered_entry_id == Some(*entry_id) {
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
                    let label = if *entry_id == "retry" { "重试" } else { "取消" };
                    let color = if enabled { MENU_TEXT_ENABLED } else { MENU_TEXT_DISABLED };
                    let pressed = enabled && pressed_entry_id == Some(*entry_id);
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
                    let label = if *entry_id == "retry" { "重试" } else { "取消" };
                    let color = if enabled { MENU_TEXT_ENABLED } else { MENU_TEXT_DISABLED };
                    blit_caption_in_cell(&mut page, fnt, label, rect.x + 8, rect.y + 8, rect.w - 16, rect.h - 16, color);
                }
            }
        }
    }

    Some(page)
}
