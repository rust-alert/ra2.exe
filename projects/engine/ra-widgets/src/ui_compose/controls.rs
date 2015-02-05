//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub(super) fn draw_trackbar(dst: &mut RgbaImage, track: RectPx, pos: u8, max: u8) {
    fill_rect(dst, track, [64, 16, 16, 255]);
    let inner = RectPx::new(track.x + 2, track.y + 2, (track.w - 4).max(1), (track.h - 4).max(1));
    fill_rect(dst, inner, [12, 12, 16, 255]);
    let max = max.max(1);
    let travel = (inner.w - 10).max(1);
    let thumb_x = inner.x + (i32::from(pos) * travel) / i32::from(max);
    let thumb = RectPx::new(thumb_x, inner.y - 1, 10, inner.h + 2);
    fill_rect(dst, thumb, [220, 40, 40, 255]);
}

pub(super) fn draw_checkbox(dst: &mut RgbaImage, rect: RectPx, checked: bool) {
    let box_r = RectPx::new(rect.x, rect.y + 2, 16, 16);
    fill_rect(dst, box_r, [80, 16, 16, 255]);
    fill_rect(dst, RectPx::new(box_r.x + 2, box_r.y + 2, 12, 12), [12, 12, 16, 255]);
    if checked {
        fill_rect(dst, RectPx::new(box_r.x + 4, box_r.y + 4, 8, 8), [220, 40, 40, 255]);
    }
}

pub(super) fn draw_section_rule(dst: &mut RgbaImage, section: RectPx) {
    let y = section.y + section.h + 2;
    fill_rect(dst, RectPx::new(section.x, y, section.w, 2), [180, 24, 24, 255]);
}

/// 在选项页上绘制左栏控件（滑条 / 勾选 / 分辨率）与分区文案。
pub fn paint_options_dialog_controls(
    page: &mut RgbaImage,
    layout: &crate::options_dialog::OptionsDialogLayout,
    state: &crate::options_dialog::OptionsDialogState,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
) {
    // 左板：深色底板（原版黑底 + 地图水印未接前用纯色占位）。
    fill_rect(page, layout.content, [8, 10, 14, 255]);
    fill_rect(page, RectPx::new(layout.content.x + 2, layout.content.y + 2, layout.content.w - 4, layout.content.h - 4), [18, 22, 32, 255]);

    let label = |kind: &str, fallback: &str| resolve_caption(csf, fallback, options_dialog_csf_key(kind));

    if let Some(fnt) = fnt {
        blit_text_colored(page, fnt, &label("display", "Display Options"), layout.sec_display.x, layout.sec_display.y, MENU_TEXT_SECTION);
        draw_section_rule(page, layout.sec_display);
        blit_text_colored(page, fnt, &label("detail", "Visual Details"), layout.track_detail.x, layout.track_detail.y - 16, MENU_TEXT_ACCENT);
        blit_text_colored(
            page,
            fnt,
            &label("resolution", "Set Game Resolution"),
            layout.resolution.x,
            layout.resolution.y - 16,
            MENU_TEXT_ACCENT,
        );
        blit_text_colored(
            page,
            fnt,
            &label("high", "High"),
            layout.track_detail.x + layout.track_detail.w + 8,
            layout.track_detail.y + 2,
            MENU_TEXT_ACCENT,
        );

        blit_text_colored(page, fnt, &label("game", "Game Options"), layout.sec_game.x, layout.sec_game.y, MENU_TEXT_SECTION);
        draw_section_rule(page, layout.sec_game);
        blit_text_colored(
            page,
            fnt,
            &label("difficulty", "Difficulty"),
            layout.track_difficulty.x,
            layout.track_difficulty.y - 16,
            MENU_TEXT_ACCENT,
        );
        blit_text_colored(
            page,
            fnt,
            &label("hard", "Hard"),
            layout.track_difficulty.x + layout.track_difficulty.w + 8,
            layout.track_difficulty.y + 2,
            MENU_TEXT_ACCENT,
        );

        blit_text_colored(page, fnt, &label("ui", "UI Options"), layout.sec_ui.x, layout.sec_ui.y, MENU_TEXT_SECTION);
        draw_section_rule(page, layout.sec_ui);
        blit_text_colored(page, fnt, &label("scroll", "Scroll Rate"), layout.track_scroll.x, layout.track_scroll.y - 16, MENU_TEXT_ACCENT);
        blit_text_colored(
            page,
            fnt,
            &label("fastest", "Fastest"),
            layout.track_scroll.x + layout.track_scroll.w + 8,
            layout.track_scroll.y + 2,
            MENU_TEXT_ACCENT,
        );

        blit_text_colored(page, fnt, &label("present", "Present Feel"), layout.sec_present.x, layout.sec_present.y, MENU_TEXT_SECTION);
        draw_section_rule(page, layout.sec_present);
        blit_text_colored(page, fnt, &label("audio", "Audio Options"), layout.sec_audio.x, layout.sec_audio.y, MENU_TEXT_SECTION);
        draw_section_rule(page, layout.sec_audio);
    }

    draw_trackbar(page, layout.track_detail, state.detail, crate::options_dialog::OptionsTrackbar::Detail.max());
    draw_trackbar(page, layout.track_difficulty, state.difficulty, crate::options_dialog::OptionsTrackbar::Difficulty.max());
    draw_trackbar(page, layout.track_scroll, state.scroll, crate::options_dialog::OptionsTrackbar::Scroll.max());
    draw_trackbar(page, layout.track_music, state.music, crate::options_dialog::OptionsTrackbar::Music.max());
    draw_trackbar(page, layout.track_sound, state.sound, crate::options_dialog::OptionsTrackbar::Sound.max());
    draw_trackbar(page, layout.track_voice, state.voice, crate::options_dialog::OptionsTrackbar::Voice.max());

    draw_checkbox(page, layout.checks[0], state.tooltips);
    draw_checkbox(page, layout.checks[1], state.scanlines);
    draw_checkbox(page, layout.checks[2], state.show_damage);
    draw_checkbox(page, layout.check_present, state.present.is_active());
    if let Some(fnt) = fnt {
        let tx = layout.checks[0].x + 22;
        blit_text_colored(page, fnt, &label("tooltips", "Tooltips"), tx, layout.checks[0].y + 4, MENU_TEXT_ACCENT);
        blit_text_colored(page, fnt, &label("scanlines", "Target Lines"), tx, layout.checks[1].y + 4, MENU_TEXT_ACCENT);
        blit_text_colored(page, fnt, &label("damage", "See Hidden Objects"), tx, layout.checks[2].y + 4, MENU_TEXT_ACCENT);
        blit_text_colored(
            page,
            fnt,
            &label("present_16bit", "16-bit Present"),
            layout.check_present.x + 22,
            layout.check_present.y + 4,
            MENU_TEXT_ACCENT,
        );
        blit_text_colored(page, fnt, &label("music", "Music Volume"), layout.track_music.x, layout.track_music.y - 16, MENU_TEXT_ACCENT);
        blit_text_colored(page, fnt, &label("sound", "Sound Volume"), layout.track_sound.x, layout.track_sound.y - 16, MENU_TEXT_ACCENT);
        blit_text_colored(page, fnt, &label("voice", "Voice Volume"), layout.track_voice.x, layout.track_voice.y - 16, MENU_TEXT_ACCENT);
    }

    fill_rect(page, layout.resolution, [120, 24, 24, 255]);
    fill_rect(
        page,
        RectPx::new(layout.resolution.x + 2, layout.resolution.y + 2, layout.resolution.w - 4, layout.resolution.h - 4),
        [8, 8, 12, 255],
    );
    if let Some(fnt) = fnt {
        blit_text_colored(page, fnt, state.display_mode.as_str(), layout.resolution.x + 8, layout.resolution.y + 6, MENU_TEXT_ACCENT);
    }
    if state.resolution_open {
        for (i, mode) in ra_types::DisplayMode::ALL.iter().enumerate() {
            let row = layout.resolution_row(i);
            let bg = if *mode == state.display_mode { [90, 40, 20, 255] } else { [28, 28, 34, 255] };
            fill_rect(page, row, bg);
            if let Some(fnt) = fnt {
                blit_text_colored(page, fnt, mode.as_str(), row.x + 8, row.y + 4, MENU_TEXT_ACCENT);
            }
        }
    }
}


pub(super) fn draw_bevel_frame(dst: &mut RgbaImage, rect: RectPx, inset_fill: Option<[u8; 4]>) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    // 0xC5BEA7 / 0x807A68 → RGB。
    const LIGHT: [u8; 4] = [197, 190, 167, 255];
    const DARK: [u8; 4] = [128, 122, 104, 255];
    stroke_rect(dst, rect, LIGHT);
    if rect.w > 2 && rect.h > 2 {
        stroke_rect(dst, RectPx::new(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2), DARK);
    }
    if let Some(fill) = inset_fill {
        if rect.w > 4 && rect.h > 4 {
            fill_rect(dst, RectPx::new(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4), fill);
        }
    }
}

/// 编辑框面（玩家名）：斜角框，无下拉箭头。
pub(super) fn draw_edit_face(dst: &mut RgbaImage, rect: RectPx, fill: [u8; 4]) {
    draw_bevel_frame(dst, rect, Some(fill));
}

/// 下拉塌陷面：斜角框 + 右侧 `dnarrow*.pcx` 箭头。
pub(super) fn draw_combo_face(dst: &mut RgbaImage, rect: RectPx, fill: [u8; 4], chrome: Option<&SkirmishChromeSprites>, arrow_pressed: bool) {
    let body_w = (rect.w - SKIRMISH_COMBO_ARROW_RESERVE).max(1);
    draw_bevel_frame(dst, rect, Some([8, 8, 12, 255]));
    if body_w > 4 && rect.h > 4 {
        fill_rect(dst, RectPx::new(rect.x + 2, rect.y + 2, body_w - 2, rect.h - 4), fill);
    }
    let arrow = chrome.and_then(|c| if arrow_pressed { c.combo_arrow_pressed.as_ref() } else { c.combo_arrow.as_ref() });
    if let Some(img) = arrow {
        // 箭头原点：client_width - 19, y + 1。
        blit_rgba(dst, img, rect.x + rect.w - 19, rect.y + 1);
    }
}

/// 颜色下拉塌陷面：非箭头区色块 + 箭头。
pub(super) fn draw_color_combo_face(dst: &mut RgbaImage, rect: RectPx, rgb: [u8; 3], chrome: Option<&SkirmishChromeSprites>, arrow_pressed: bool) {
    let body_w = (rect.w - SKIRMISH_COMBO_ARROW_RESERVE).max(1);
    draw_bevel_frame(dst, rect, Some([8, 8, 12, 255]));
    let swatch = RectPx::new(rect.x + 2, rect.y + 2, (body_w - 2).max(1), (rect.h - 4).max(1));
    fill_rect(dst, swatch, [rgb[0], rgb[1], rgb[2], 255]);
    let arrow = chrome.and_then(|c| if arrow_pressed { c.combo_arrow_pressed.as_ref() } else { c.combo_arrow.as_ref() });
    if let Some(img) = arrow {
        blit_rgba(dst, img, rect.x + rect.w - 19, rect.y + 1);
    }
}

pub(super) fn draw_skirmish_checkbox(dst: &mut RgbaImage, rect: RectPx, checked: bool, chrome: Option<&SkirmishChromeSprites>) {
    let box_r = RectPx::new(rect.x, rect.y, SKIRMISH_CHECK_W, SKIRMISH_CHECK_H.min(rect.h.max(SKIRMISH_CHECK_H)));
    if let Some(img) = chrome.and_then(|c| if checked { c.checkbox_on.as_ref() } else { c.checkbox_off.as_ref() }) {
        blit_rgba(dst, img, box_r.x, box_r.y);
        return;
    }
    fill_rect(dst, box_r, [90, 20, 20, 255]);
    stroke_rect(dst, box_r, [200, 40, 40, 255]);
    fill_rect(dst, RectPx::new(box_r.x + 2, box_r.y + 2, box_r.w - 4, box_r.h - 4), [12, 12, 16, 255]);
    if checked {
        fill_rect(dst, RectPx::new(box_r.x + 5, box_r.y + 5, 8, 8), [255, 160, 32, 255]);
    }
}

pub(super) fn track_active_width(track_w: i32) -> i32 {
    (track_w - SKIRMISH_TRACK_PLAQUE_W - SKIRMISH_TRACK_ACTIVE_PAD).max(1)
}

pub(super) fn track_plaque_rect(track: RectPx) -> RectPx {
    // 底板：本地 x = client_w - 50 + 1，y = -1。
    RectPx::new(track.x + track.w - SKIRMISH_TRACK_PLAQUE_W + 1, track.y - 1, SKIRMISH_TRACK_PLAQUE_W, 24)
}

pub(super) fn blit_track_plaque(dst: &mut RgbaImage, plaque: RectPx, chrome: Option<&SkirmishChromeSprites>) {
    let Some((cap_l, cap_m, cap_r)) =
        chrome.and_then(|c| Some((c.track_cap_l.as_ref()?, c.track_cap_m.as_ref()?, c.track_cap_r.as_ref()?)))
    else {
        fill_rect(dst, RectPx::new(plaque.x, plaque.y + 1, plaque.w, (plaque.h - 2).max(1)), [40, 12, 12, 255]);
        stroke_rect(dst, RectPx::new(plaque.x, plaque.y + 1, plaque.w, (plaque.h - 2).max(1)), [180, 40, 40, 255]);
        return;
    };
    let lw = cap_l.width() as i32;
    let rw = cap_r.width() as i32;
    blit_rgba(dst, cap_l, plaque.x, plaque.y);
    blit_rgba(dst, cap_r, plaque.x + plaque.w - rw, plaque.y);
    let mid_w = (plaque.w - lw - rw).max(1);
    blit_stretched(dst, cap_m, RectPx::new(plaque.x + lw, plaque.y, mid_w, cap_m.height() as i32));
}

/// 滑条：左轨斜角 + `trakgrip` 拇指 + 右侧 `trofl/m/r` 数值底板（`trof*` 不是轨道）。
pub(super) fn draw_skirmish_trackbar(dst: &mut RgbaImage, track: RectPx, pos: i32, max: i32, chrome: Option<&SkirmishChromeSprites>) {
    let plaque = track_plaque_rect(track);
    let rail_w = (track.w - SKIRMISH_TRACK_PLAQUE_W).max(1);
    let rail = RectPx::new(track.x, track.y, rail_w, track.h);
    blit_track_plaque(dst, plaque, chrome);
    draw_bevel_frame(dst, rail, Some([24, 10, 10, 255]));

    let max = max.max(1);
    let active_w = track_active_width(track.w);
    let offset = (pos.clamp(0, max) * active_w) / max;
    let thumb_x = track.x + 1 + offset;
    if let Some(thumb) = chrome.and_then(|c| c.track_thumb.as_ref()) {
        let ty = track.y + (track.h - thumb.height() as i32) / 2;
        blit_rgba(dst, thumb, thumb_x, ty);
    }
    else {
        fill_rect(dst, RectPx::new(thumb_x, track.y + 1, SKIRMISH_TRACK_THUMB_W, (track.h - 2).max(1)), [220, 40, 40, 255]);
    }
}
