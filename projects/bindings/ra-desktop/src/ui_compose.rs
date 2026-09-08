//! 将已解码壳层精灵合成整页 RGBA（上传 `set_ui_page` 之前）。
//!
//! 合成 ≠ atlas/instance 终态；当前只为验证颜色、原尺寸与粗略位置。

use ra_assets::{CsfFile, FntFile};
use ra_renderer::RgbaImage;

use crate::{
    ui_decode::{DecodedUiSprite, PageDecodeReport},
    ui_layout::{
        LOBBY_MAP_ROW_MAX, MAIN_MENU_BUTTON_IDS, MainMenuLayout, OPTIONS_BUTTON_IDS, RectPx, SINGLE_PLAYER_BUTTON_IDS,
        SKIRMISH_LOBBY_BUTTON_IDS, main_menu_layout, options_layout, single_player_layout, skirmish_lobby_layout,
        skirmish_map_row_rect,
    },
    ui_text::{
        MENU_TEXT_DISABLED, MENU_TEXT_ENABLED, blit_caption_in_cell, blit_text_colored, main_menu_csf_label, main_menu_csf_tooltip,
        options_csf_label, resolve_caption, resolve_csf_text, single_player_csf_label, skirmish_lobby_csf_label,
    },
};

/// 壳层按钮文案来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuCaptionKind {
    Main,
    SinglePlayer,
    SkirmishLobby,
    Options,
}

impl MenuCaptionKind {
    fn label(self, entry_id: &str) -> Option<&'static str> {
        match self {
            Self::Main => main_menu_csf_label(entry_id),
            Self::SinglePlayer => single_player_csf_label(entry_id),
            Self::SkirmishLobby => skirmish_lobby_csf_label(entry_id),
            Self::Options => options_csf_label(entry_id),
        }
    }
}

/// Alpha over 将 `src` 画到 `dst` 的 `(x,y)`（可裁剪）。
pub fn blit_rgba(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
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

fn blit_stretched(dst: &mut RgbaImage, src: &RgbaImage, rect: RectPx) {
    if rect.w <= 0 || rect.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    // 面板条允许纵向/横向铺满目标格；用最近邻，避免模糊。
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
            if src.as_raw()[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
        }
    }
}

fn fill_rect(dst: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    for row in 0..rect.h {
        let dy = rect.y + row;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..rect.w {
            let dx = rect.x + col;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&rgba);
        }
    }
}

fn draw_trackbar(dst: &mut RgbaImage, track: RectPx, pos: u8, max: u8) {
    fill_rect(dst, track, [40, 40, 48, 255]);
    let inner = RectPx::new(track.x + 2, track.y + 2, (track.w - 4).max(1), (track.h - 4).max(1));
    fill_rect(dst, inner, [18, 18, 22, 255]);
    let max = max.max(1);
    let travel = (inner.w - 10).max(1);
    let thumb_x = inner.x + (i32::from(pos) * travel) / i32::from(max);
    let thumb = RectPx::new(thumb_x, inner.y - 1, 10, inner.h + 2);
    fill_rect(dst, thumb, [220, 180, 40, 255]);
}

fn draw_checkbox(dst: &mut RgbaImage, rect: RectPx, checked: bool) {
    let box_r = RectPx::new(rect.x, rect.y + 2, 16, 16);
    fill_rect(dst, box_r, [40, 40, 48, 255]);
    fill_rect(
        dst,
        RectPx::new(box_r.x + 2, box_r.y + 2, 12, 12),
        [18, 18, 22, 255],
    );
    if checked {
        fill_rect(
            dst,
            RectPx::new(box_r.x + 4, box_r.y + 4, 8, 8),
            [220, 180, 40, 255],
        );
    }
}

/// 在已合成的选项 chrome 上绘制左栏控件（滑条 / 勾选 / 分辨率）。
pub fn paint_options_dialog_controls(
    page: &mut RgbaImage,
    layout: &crate::options_dialog::OptionsDialogLayout,
    state: &crate::options_dialog::OptionsDialogState,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
) {
    fill_rect(page, layout.content, [12, 16, 28, 220]);

    let label = |key: &str, fallback: &str| resolve_caption(csf, fallback, Some(key));

    if let Some(fnt) = fnt {
        blit_text_colored(page, fnt, &label("GUI:Display", "display"), layout.sec_display.x, layout.sec_display.y, MENU_TEXT_ENABLED);
        blit_text_colored(page, fnt, &label("GUI:Game", "game"), layout.sec_game.x, layout.sec_game.y, MENU_TEXT_ENABLED);
        blit_text_colored(page, fnt, &label("GUI:Interface", "interface"), layout.sec_ui.x, layout.sec_ui.y, MENU_TEXT_ENABLED);
        blit_text_colored(page, fnt, &label("GUI:Sound", "sound"), layout.sec_audio.x, layout.sec_audio.y, MENU_TEXT_ENABLED);
    }

    draw_trackbar(page, layout.track_detail, state.detail, crate::options_dialog::OptionsTrackbar::Detail.max());
    draw_trackbar(
        page,
        layout.track_difficulty,
        state.difficulty,
        crate::options_dialog::OptionsTrackbar::Difficulty.max(),
    );
    draw_trackbar(page, layout.track_scroll, state.scroll, crate::options_dialog::OptionsTrackbar::Scroll.max());
    draw_trackbar(page, layout.track_music, state.music, crate::options_dialog::OptionsTrackbar::Music.max());
    draw_trackbar(page, layout.track_sound, state.sound, crate::options_dialog::OptionsTrackbar::Sound.max());
    draw_trackbar(page, layout.track_voice, state.voice, crate::options_dialog::OptionsTrackbar::Voice.max());

    draw_checkbox(page, layout.checks[0], state.tooltips);
    draw_checkbox(page, layout.checks[1], state.scanlines);
    draw_checkbox(page, layout.checks[2], state.show_damage);
    if let Some(fnt) = fnt {
        let tx = layout.checks[0].x + 22;
        blit_text_colored(page, fnt, &label("GUI:Tooltips", "tooltips"), tx, layout.checks[0].y + 4, MENU_TEXT_ENABLED);
        blit_text_colored(page, fnt, &label("GUI:Scanlines", "scanlines"), tx, layout.checks[1].y + 4, MENU_TEXT_ENABLED);
        blit_text_colored(page, fnt, &label("GUI:ShowDamage", "damage"), tx, layout.checks[2].y + 4, MENU_TEXT_ENABLED);
        blit_text_colored(
            page,
            fnt,
            &label("GUI:Music", "music"),
            layout.track_music.x,
            layout.track_music.y - 16,
            MENU_TEXT_ENABLED,
        );
        blit_text_colored(
            page,
            fnt,
            &label("GUI:Sound", "sound"),
            layout.track_sound.x,
            layout.track_sound.y - 16,
            MENU_TEXT_ENABLED,
        );
        blit_text_colored(
            page,
            fnt,
            &label("GUI:Voice", "voice"),
            layout.track_voice.x,
            layout.track_voice.y - 16,
            MENU_TEXT_ENABLED,
        );
    }

    fill_rect(page, layout.resolution, [40, 40, 48, 255]);
    fill_rect(
        page,
        RectPx::new(layout.resolution.x + 2, layout.resolution.y + 2, layout.resolution.w - 4, layout.resolution.h - 4),
        [18, 18, 22, 255],
    );
    if let Some(fnt) = fnt {
        blit_text_colored(
            page,
            fnt,
            state.display_mode.as_str(),
            layout.resolution.x + 8,
            layout.resolution.y + 6,
            MENU_TEXT_ENABLED,
        );
    }
    if state.resolution_open {
        for (i, mode) in ra_types::DisplayMode::ALL.iter().enumerate() {
            let row = layout.resolution_row(i);
            let bg = if *mode == state.display_mode {
                [60, 50, 20, 255]
            } else {
                [28, 28, 34, 255]
            };
            fill_rect(page, row, bg);
            if let Some(fnt) = fnt {
                blit_text_colored(page, fnt, mode.as_str(), row.x + 8, row.y + 4, MENU_TEXT_ENABLED);
            }
        }
    }
}

fn find_panel<'a>(decoded: &'a PageDecodeReport, needle: &str) -> Option<&'a DecodedUiSprite> {
    decoded.panels.iter().find(|p| p.label.to_ascii_lowercase().starts_with(&needle.to_ascii_lowercase()))
}

fn find_button_normal<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_normals.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

fn find_button_hover<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_hovers.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

fn find_button_pressed<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_presseds.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

fn compose_shell_menu_page(
    decoded: &PageDecodeReport,
    layout: MainMenuLayout,
    button_ids: &[&str],
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    captions: MenuCaptionKind,
) -> Option<RgbaImage> {
    let bg = decoded.background.as_ref()?;
    let mut page = RgbaImage::from_raw(
        layout.canvas.w as u32,
        layout.canvas.h as u32,
        vec![0u8; (layout.canvas.w as usize) * (layout.canvas.h as usize) * 4],
    )?;

    blit_rgba(&mut page, &bg.image, layout.background.x, layout.background.y);
    if let Some(frame) = movie {
        blit_stretched(&mut page, frame, layout.movie);
    }

    if let Some(top) = find_panel(decoded, "sdtp.shp") {
        blit_stretched(&mut page, &top.image, layout.panel_top);
    }
    if let Some(tile) = find_panel(decoded, "sdbtnbkgd.shp") {
        for i in 0..layout.panel_tile_count {
            let r = RectPx::new(layout.panel_tile.x, layout.panel_tile.y + i * layout.panel_tile.h, layout.panel_tile.w, layout.panel_tile.h);
            blit_stretched(&mut page, &tile.image, r);
        }
    }
    if let Some(bottom) = find_panel(decoded, "sdbtm.shp") {
        blit_stretched(&mut page, &bottom.image, layout.panel_bottom);
    }
    if let Some(lower) = find_panel(decoded, "lwscrnl.shp") {
        blit_stretched(&mut page, &lower.image, layout.lower_strip);
    }

    for (i, entry_id) in button_ids.iter().enumerate() {
        let normal = find_button_normal(decoded, entry_id)?;
        let disabled = matches!(
            *entry_id,
            "ww_online" | "network" | "movies" | "campaign" | "training"
        );
        let sprite = if pressed_entry_id == Some(*entry_id) && !disabled {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        } else if hovered_entry_id == Some(*entry_id) && !disabled {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        } else {
            normal
        };
        let cell = layout.buttons[i];
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        if let Some(fnt) = fnt {
            let key = captions.label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let color = if disabled { MENU_TEXT_DISABLED } else { MENU_TEXT_ENABLED };
            blit_caption_in_cell(&mut page, fnt, &caption, cell.x, cell.y, cell.w, cell.h, color);
        }
    }

    if matches!(captions, MenuCaptionKind::Main | MenuCaptionKind::Options) {
        if let Some(fnt) = fnt {
            let title_key = match captions {
                MenuCaptionKind::Main => Some("GUI:MainMenu"),
                MenuCaptionKind::Options => Some("GUI:Options"),
                _ => None,
            };
            let title_fallback = match captions {
                MenuCaptionKind::Main => "main_menu",
                MenuCaptionKind::Options => "options",
                _ => "menu",
            };
            let title = resolve_caption(csf, title_fallback, title_key);
            blit_caption_in_cell(
                &mut page,
                fnt,
                &title,
                layout.title.x,
                layout.title.y,
                layout.title.w,
                layout.title.h,
                MENU_TEXT_ENABLED,
            );
            if captions == MenuCaptionKind::Main {
                if let Some(hovered) = hovered_entry_id {
                    if let Some(key) = main_menu_csf_tooltip(hovered) {
                        if let Some(text) = resolve_csf_text(csf, key) {
                            blit_text_colored(&mut page, fnt, &text, layout.tooltip.x, layout.tooltip.y, MENU_TEXT_ENABLED);
                        }
                    }
                }
            }
        }
    }

    Some(page)
}

/// 合成主菜单 chrome。
pub fn compose_main_menu_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
) -> Option<RgbaImage> {
    compose_shell_menu_page(
        decoded,
        main_menu_layout(viewport_w, viewport_h),
        &MAIN_MENU_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        movie,
        MenuCaptionKind::Main,
    )
}

/// 合成单人游戏页 chrome。
pub fn compose_single_player_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
) -> Option<RgbaImage> {
    compose_shell_menu_page(
        decoded,
        single_player_layout(viewport_w, viewport_h),
        &SINGLE_PLAYER_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        movie,
        MenuCaptionKind::SinglePlayer,
    )
}

/// 合成选项页 chrome（音频/视频禁用占位，返回可用）。
pub fn compose_options_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
) -> Option<RgbaImage> {
    compose_shell_menu_page(
        decoded,
        options_layout(viewport_w, viewport_h),
        &OPTIONS_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        movie,
        MenuCaptionKind::Options,
    )
}

/// 合成遭遇战大厅 chrome（可选地图预览与地图名列表）。
pub fn compose_skirmish_lobby_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    map_preview: Option<&RgbaImage>,
    map_names: &[(String, bool)],
) -> Option<RgbaImage> {
    let layout = skirmish_lobby_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout,
        &SKIRMISH_LOBBY_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        map_preview,
        MenuCaptionKind::SkirmishLobby,
    )?;
    if let Some(fnt) = fnt {
        let n = map_names.len().min(LOBBY_MAP_ROW_MAX as usize);
        for (i, (name, selected)) in map_names.iter().take(n).enumerate() {
            let row = skirmish_map_row_rect(&layout, i);
            let color = if *selected { MENU_TEXT_ENABLED } else { MENU_TEXT_DISABLED };
            let label = if *selected { format!("> {name}") } else { name.clone() };
            blit_text_colored(&mut page, fnt, &label, row.x + 4, row.y + 4, color);
        }
    }
    Some(page)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options_dialog::{OptionsDialogLayout, OptionsDialogState};
    use ra_types::DisplayMode;

    #[test]
    fn paint_options_draws_music_thumb() {
        let mut page = RgbaImage::from_raw(800, 600, vec![0u8; 800 * 600 * 4]).unwrap();
        let layout = OptionsDialogLayout::new();
        let state = OptionsDialogState::from_shell(DisplayMode::W800H600, 1.0, 0.0);
        paint_options_dialog_controls(&mut page, &layout, &state, None, None);
        let track = layout.track_music;
        let px = track.x + track.w - 8;
        let py = track.y + track.h / 2;
        let di = ((py as u32 * page.width() + px as u32) * 4) as usize;
        assert_eq!(&page.as_raw()[di..di + 3], &[220, 180, 40]);
    }
}
