//! 将已解码壳层精灵合成整页 RGBA（上传 `set_ui_page` 之前）。
//!
//! 合成 ≠ atlas/instance 终态；当前只为验证颜色、原尺寸与粗略位置。

use ra_assets::{CsfFile, FntFile};
use ra_renderer::RgbaImage;

use crate::{
    skirmish_setup::{LOBBY_COLORS, LOBBY_DIFFICULTIES, LOBBY_SIDES},
    ui_decode::{DecodedUiSprite, PageDecodeReport},
    ui_text::{
        MENU_TEXT_ACCENT, MENU_TEXT_DISABLED, MENU_TEXT_ENABLED, MENU_TEXT_SECTION, blit_caption_in_cell, blit_caption_top_left_clipped,
        blit_text_colored, campaign_csf_label, campaign_difficulty_csf_key, campaign_title_csf_key, choose_map_csf_label,
        choose_map_static_csf_key, choose_map_title_csf_key, exit_confirm_csf_label, exit_confirm_prompt_csf_key, main_menu_csf_label,
        options_csf_label, options_dialog_csf_key, resolve_caption, single_player_csf_label, single_player_title_csf_key,
        skirmish_lobby_csf_label, skirmish_lobby_static_csf_key, skirmish_title_csf_key,
    },
};
use ra_layout::{
    BUTTON_CELL_W, CAMPAIGN_BUTTON_IDS, CHOOSE_MAP_BUTTON_IDS, EXIT_CONFIRM_BUTTON_IDS, MAIN_MENU_BUTTON_IDS, MainMenuLayout,
    OPTIONS_BUTTON_IDS, RIGHT_PANEL_W, RectPx, SDWRNANM_OFFSET_X, SDWRNANM_OFFSET_Y, SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_CHECK_H,
    SKIRMISH_CHECK_W, SKIRMISH_COMBO_FACE_H, SKIRMISH_LOBBY_BUTTON_IDS, SkirmishLobbyLayout, campaign_layout, choose_map_layout,
    exit_confirm_layout, main_menu_layout, options_layout, single_player_layout, skirmish_lobby_layout,
};

/// 切页波浪帧：有字钮进出；空格仅在出去时叠 `SDBTNANM`（进来不叠满钮，避免收束后消失）。
#[derive(Debug, Clone, Copy)]
pub struct ShellWaveFrames<'a> {
    /// 与当前页按钮 id 表对齐。
    pub buttons: &'a [u16],
    /// 与 `panel_tile_count` 对齐；仅当 `animate_empty_tiles` 时用于空格。
    pub tiles: &'a [u16],
    /// 空格是否叠钮面波浪（`SlideOut` 为 true，`SlideIn` / 间隙为 false）。
    pub animate_empty_tiles: bool,
}

/// 壳层按钮文案来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuCaptionKind {
    Main,
    SinglePlayer,
    Campaign,
    SkirmishLobby,
    ChooseMap,
}

impl MenuCaptionKind {
    fn label(self, entry_id: &str) -> Option<&'static str> {
        match self {
            Self::Main => main_menu_csf_label(entry_id),
            Self::SinglePlayer => single_player_csf_label(entry_id),
            Self::Campaign => campaign_csf_label(entry_id),
            Self::SkirmishLobby => skirmish_lobby_csf_label(entry_id),
            Self::ChooseMap => choose_map_csf_label(entry_id),
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
            let raw = src.as_raw();
            if raw[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 1:1 贴图，跳过透明与近黑（`fsscrn` 空区约 (8,8,8)，非索引 0）。
fn blit_rgba_skip_near_black(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, max_rgb_sum: u16) {
    let raw = src.as_raw();
    for row in 0..src.height() {
        for col in 0..src.width() {
            let si = ((row * src.width() + col) * 4) as usize;
            if raw[si + 3] == 0 {
                continue;
            }
            let sum = u16::from(raw[si]) + u16::from(raw[si + 1]) + u16::from(raw[si + 2]);
            if sum <= max_rgb_sum {
                continue;
            }
            let dx = x + col as i32;
            let dy = y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width() || dy as u32 >= dst.height() {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 相对静止帧差分贴图：只画动画帧相对 `base` 变化的像素（战役悬停箭头）。
fn blit_rgba_diff_from_base(dst: &mut RgbaImage, src: &RgbaImage, base: Option<&RgbaImage>, x: i32, y: i32, max_rgb_sum: u16) {
    let Some(base) = base
    else {
        blit_rgba_skip_near_black(dst, src, x, y, max_rgb_sum);
        return;
    };
    if src.width() != base.width() || src.height() != base.height() {
        blit_rgba_skip_near_black(dst, src, x, y, max_rgb_sum);
        return;
    }
    let raw = src.as_raw();
    let base_raw = base.as_raw();
    for row in 0..src.height() {
        for col in 0..src.width() {
            let si = ((row * src.width() + col) * 4) as usize;
            if raw[si + 3] == 0 {
                continue;
            }
            let sum = u16::from(raw[si]) + u16::from(raw[si + 1]) + u16::from(raw[si + 2]);
            if sum <= max_rgb_sum {
                continue;
            }
            let dr = i16::from(raw[si]).abs_diff(i16::from(base_raw[si]));
            let dg = i16::from(raw[si + 1]).abs_diff(i16::from(base_raw[si + 1]));
            let db = i16::from(raw[si + 2]).abs_diff(i16::from(base_raw[si + 2]));
            if u16::from(dr) + u16::from(dg) + u16::from(db) < 24 {
                continue;
            }
            let dx = x + col as i32;
            let dy = y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width() || dy as u32 >= dst.height() {
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

/// 将矩形区域整体压暗（`amount` 越大越暗，0..255）。
fn dim_rect(dst: &mut RgbaImage, rect: RectPx, amount: u8) {
    if rect.w <= 0 || rect.h <= 0 || amount == 0 {
        return;
    }
    let keep = 255u32.saturating_sub(u32::from(amount));
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
            for c in 0..3 {
                let v = u32::from(dst.as_raw()[di + c]);
                dst.as_mut()[di + c] = ((v * keep) / 255) as u8;
            }
        }
    }
}

fn draw_trackbar(dst: &mut RgbaImage, track: RectPx, pos: u8, max: u8) {
    fill_rect(dst, track, [64, 16, 16, 255]);
    let inner = RectPx::new(track.x + 2, track.y + 2, (track.w - 4).max(1), (track.h - 4).max(1));
    fill_rect(dst, inner, [12, 12, 16, 255]);
    let max = max.max(1);
    let travel = (inner.w - 10).max(1);
    let thumb_x = inner.x + (i32::from(pos) * travel) / i32::from(max);
    let thumb = RectPx::new(thumb_x, inner.y - 1, 10, inner.h + 2);
    fill_rect(dst, thumb, [220, 40, 40, 255]);
}

fn draw_checkbox(dst: &mut RgbaImage, rect: RectPx, checked: bool) {
    let box_r = RectPx::new(rect.x, rect.y + 2, 16, 16);
    fill_rect(dst, box_r, [80, 16, 16, 255]);
    fill_rect(dst, RectPx::new(box_r.x + 2, box_r.y + 2, 12, 12), [12, 12, 16, 255]);
    if checked {
        fill_rect(dst, RectPx::new(box_r.x + 4, box_r.y + 4, 8, 8), [220, 40, 40, 255]);
    }
}

fn draw_section_rule(dst: &mut RgbaImage, section: RectPx) {
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

fn find_panel<'a>(decoded: &'a PageDecodeReport, needle: &str, anim_frame: usize) -> Option<&'a DecodedUiSprite> {
    let needle = needle.to_ascii_lowercase();
    let mut matches: Vec<&DecodedUiSprite> = decoded
        .panels
        .iter()
        .filter(|p| {
            let lab = p.label.to_ascii_lowercase();
            lab == needle || lab.starts_with(&format!("{needle}#"))
        })
        .collect();
    if matches.is_empty() {
        return None;
    }
    matches.sort_by_key(|p| p.frame);
    Some(matches[anim_frame % matches.len()])
}

/// 右栏顶盖：先画 `sdtp` 帧 0 外壳，再把 `sdwrnanm` 当前帧 1:1 贴进窗内（不拉伸、不盖金属边框）。
fn blit_right_panel_top(page: &mut RgbaImage, decoded: &PageDecodeReport, panel_top: RectPx, warn_anim_frame: usize) {
    if let Some(top) = find_panel(decoded, "sdtp.shp", 0) {
        blit_stretched(page, &top.image, panel_top);
    }
    if let Some(warn) = find_panel(decoded, "sdwrnanm.shp", warn_anim_frame) {
        blit_rgba(page, &warn.image, panel_top.x + SDWRNANM_OFFSET_X, panel_top.y + SDWRNANM_OFFSET_Y);
    }
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

/// 主菜单 owner-draw 文案裁切：未按 `+0/+1/-2/-1`，按下 `+2/+5/-4/-5`。
fn owner_draw_caption_rect(cell: RectPx, pressed: bool) -> (i32, i32, i32, i32) {
    let (dx, dy) = if pressed { (2, 5) } else { (0, 1) };
    (cell.x + dx, cell.y + dy, (cell.w - 2 - dx).max(0), (cell.h - dy).max(0))
}

fn compose_shell_menu_page(
    decoded: &PageDecodeReport,
    layout: MainMenuLayout,
    button_ids: &[&str],
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    // 底栏状态提示可见切片（壳层打字机提供，与按钮 hover 图解耦）。
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    captions: MenuCaptionKind,
    // 切页波浪；`None` 走常态/悬停/按下。
    wave: Option<ShellWaveFrames<'_>>,
    // WARNING 窗内 `sdwrnanm` 帧（对解码帧数取模）。
    warn_anim_frame: usize,
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

    blit_right_panel_top(&mut page, decoded, layout.panel_top, warn_anim_frame);
    let btn_n = button_ids.len();
    let tile_occupied = |tile_y: i32| {
        (0..btn_n).any(|i| {
            let b = layout.buttons[i];
            b.w > 0 && b.h > 0 && b.y == tile_y
        })
    };
    if let Some(tile) = find_panel(decoded, "sdbtnbkgd.shp", 0) {
        for i in 0..layout.panel_tile_count {
            let tile_y = layout.panel_tile.y + i * layout.panel_tile.h;
            // 始终铺 `sdbtnbkgd`（含左侧红线），波浪只叠钮面，不藏底。
            let r = RectPx::new(layout.panel_tile.x, tile_y, layout.panel_tile.w, layout.panel_tile.h);
            blit_stretched(&mut page, &tile.image, r);
        }
    }
    // 波浪出去：无字平铺格叠 `SDBTNANM`；进来不叠，避免满钮收束后瞬间消失。
    if let Some(wave) = wave {
        if wave.animate_empty_tiles {
            for ti in 0..layout.panel_tile_count {
                let tile_y = layout.panel_tile.y + ti * layout.panel_tile.h;
                if tile_occupied(tile_y) {
                    continue;
                }
                let Some(&frame) = wave.tiles.get(ti as usize)
                else {
                    continue;
                };
                let Some(sprite) = decoded.sdbtnanm_frame(frame)
                else {
                    continue;
                };
                let cell_x = layout.panel_tile.x + (RIGHT_PANEL_W - BUTTON_CELL_W);
                blit_rgba(&mut page, &sprite.image, cell_x, tile_y);
            }
        }
    }
    if let Some(bottom) = find_panel(decoded, "sdbtm.shp", 0) {
        blit_stretched(&mut page, &bottom.image, layout.panel_bottom);
    }
    if let Some(lower) = find_panel(decoded, "lwscrnl.shp", 0) {
        blit_stretched(&mut page, &lower.image, layout.lower_strip);
    }

    for (i, entry_id) in button_ids.iter().enumerate() {
        let normal = find_button_normal(decoded, entry_id)?;
        // 禁用态跟入口 id：主菜单占位项 + 各页「载入」未实现；单人「新战役」已可进。
        let disabled = matches!(*entry_id, "ww_online" | "network" | "movies" | "load" | "create_random");
        let wave_frame = wave.and_then(|w| w.buttons.get(i).copied());
        let sprite = if let Some(frame) = wave_frame {
            decoded.sdbtnanm_frame(frame).unwrap_or(normal)
        }
        else if pressed_entry_id == Some(entry_id) && !disabled {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        }
        else if hovered_entry_id == Some(entry_id) && !disabled {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        }
        else {
            normal
        };
        let cell = layout.buttons[i];
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        // 切页流程：字先消 → 钮进出 → 停稳后再出字。`wave` 有值时只画钮面。
        if wave_frame.is_some() {
            continue;
        }
        // 壳层禁用：同常态 `SDBTNANM` 帧 + 暗红字，不压暗钮面（原版无整格压暗投影）。
        if let Some(fnt) = fnt {
            let key = captions.label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let color = if disabled { MENU_TEXT_DISABLED } else { MENU_TEXT_ENABLED };
            let pressed = pressed_entry_id == Some(entry_id) && !disabled;
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, color);
        }
    }

    if let Some(fnt) = fnt {
        let title = match captions {
            MenuCaptionKind::Main => Some(resolve_caption(csf, "main_menu", Some("GUI:MainMenu"))),
            MenuCaptionKind::SinglePlayer => Some(resolve_caption(csf, "single_player", Some(single_player_title_csf_key()))),
            // 战役 / 遭遇战 / 选图标题由各自 compose 按对话框锚点另画。
            MenuCaptionKind::Campaign | MenuCaptionKind::SkirmishLobby | MenuCaptionKind::ChooseMap => None,
        };
        if let Some(title) = title {
            blit_caption_in_cell(&mut page, fnt, &title, layout.title.x, layout.title.y, layout.title.w, layout.title.h, MENU_TEXT_ENABLED);
        }
        // 主菜单 / 单人页底栏：由壳层传入打字机可见切片。
        if matches!(captions, MenuCaptionKind::Main | MenuCaptionKind::SinglePlayer) {
            if let Some(text) = status_text.filter(|s| !s.is_empty()) {
                blit_text_colored(&mut page, fnt, text, layout.tooltip.x, layout.tooltip.y, MENU_TEXT_ENABLED);
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
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    compose_shell_menu_page(
        decoded,
        main_menu_layout(viewport_w, viewport_h),
        &MAIN_MENU_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        status_text,
        fnt,
        csf,
        movie,
        MenuCaptionKind::Main,
        wave,
        warn_anim_frame,
    )
}

/// 合成单人游戏页 chrome。
pub fn compose_single_player_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    compose_shell_menu_page(
        decoded,
        single_player_layout(viewport_w, viewport_h),
        &SINGLE_PLAYER_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        status_text,
        fnt,
        csf,
        movie,
        MenuCaptionKind::SinglePlayer,
        wave,
        warn_anim_frame,
    )
}

/// 战役选边绘制参数（Pre-Alpha：只记选择，不开局）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignPaint<'a> {
    /// 已选侧：`allied` / `tutorial` / `soviet`。
    pub selected_side: Option<&'static str>,
    /// 难度档：0 易 / 1 中 / 2 难。
    pub difficulty: u8,
    /// 难度滑条拇指（安装内 `trakgrip.pcx`，可空）。
    pub track_thumb: Option<&'a RgbaImage>,
    /// 侧图悬停/已选箭头动画帧（对侧图 SHP 帧数取模）。
    pub side_anim_frame: usize,
}

impl Default for CampaignPaint<'_> {
    fn default() -> Self {
        Self { selected_side: None, difficulty: 1, track_thumb: None, side_anim_frame: 1 }
    }
}

/// `fsscrn.pal` 下侧图空区近黑 RGB 和阈值（约 (8,8,8)）。
const CAMPAIGN_SIDE_NEAR_BLACK_SUM: u16 = 32;

/// 合成战役选边页：三侧图 + 难度 + 右栏载入/返回。
pub fn compose_campaign_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    paint: CampaignPaint<'_>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let layout = campaign_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
        &CAMPAIGN_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        None,
        fnt,
        csf,
        None,
        MenuCaptionKind::Campaign,
        wave,
        warn_anim_frame,
    )?;

    let sides = [("allied", "fsalg.shp", layout.allied), ("tutorial", "fsbclg.shp", layout.tutorial), ("soviet", "fsslg.shp", layout.soviet)];
    for (id, shp, rect) in sides {
        // `fsbkgdlg` 已烘焙静态徽标。勿整幅不透明拉伸侧图（近黑空区 → 黑块重影）。
        // 悬停/已选：1:1 近黑透叠箭头动画帧。
        let active = paint.selected_side == Some(id) || hovered_entry_id == Some(id);
        if active {
            // 三侧同一套：相对 frame0 差分贴像素，只叠箭头动画，不重画烘焙徽标。
            let base = find_panel(decoded, shp, 0);
            let frame = paint.side_anim_frame.max(1);
            if let Some(sprite) = find_panel(decoded, shp, frame) {
                blit_rgba_diff_from_base(&mut page, &sprite.image, base.map(|b| &b.image), rect.x, rect.y, CAMPAIGN_SIDE_NEAR_BLACK_SUM);
            }
        }
    }

    // 难度轨：底槽 + 档位拇指（优先安装内 `trakgrip.pcx`，控件 `0x50F`）。
    fill_rect(&mut page, layout.difficulty_track, [64, 16, 16, 255]);
    let inner = RectPx::new(
        layout.difficulty_track.x + 2,
        layout.difficulty_track.y + 2,
        (layout.difficulty_track.w - 4).max(1),
        (layout.difficulty_track.h - 4).max(1),
    );
    fill_rect(&mut page, inner, [12, 12, 16, 255]);
    let level = i32::from(paint.difficulty.min(2));
    let thumb_w = paint.track_thumb.map(|t| t.width() as i32).unwrap_or(10);
    let travel = (inner.w - thumb_w).max(1);
    let thumb_x = inner.x + (level * travel) / 2;
    if let Some(thumb) = paint.track_thumb {
        let ty = layout.difficulty_track.y + (layout.difficulty_track.h - thumb.height() as i32) / 2;
        blit_rgba(&mut page, thumb, thumb_x, ty);
    }
    else {
        fill_rect(&mut page, RectPx::new(thumb_x, inner.y - 1, thumb_w, inner.h + 2), [220, 40, 40, 255]);
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "campaign", Some(campaign_title_csf_key()));
        blit_caption_in_cell(&mut page, fnt, &title, layout.title.x, layout.title.y, layout.title.w, layout.title.h, MENU_TEXT_ENABLED);
        let diff_label = resolve_caption(csf, "difficulty", Some("GUI:Difficulty"));
        blit_text_colored(&mut page, fnt, &diff_label, layout.difficulty_label.x, layout.difficulty_label.y, MENU_TEXT_ENABLED);
        let diff_value = resolve_caption(csf, "difficulty_value", Some(campaign_difficulty_csf_key(paint.difficulty)));
        blit_text_colored(&mut page, fnt, &diff_value, layout.difficulty_value.x, layout.difficulty_value.y, MENU_TEXT_ENABLED);
        if let Some(text) = status_text.filter(|s| !s.is_empty()) {
            blit_text_colored(&mut page, fnt, text, layout.status_help.x, layout.status_help.y, MENU_TEXT_ENABLED);
        }
    }

    Some(page)
}

/// 合成选项页：黑底 + 右栏侧板/按钮 + 左栏对话框控件（无主菜单影片）。
pub fn compose_options_page(
    decoded: &PageDecodeReport,
    state: &crate::options_dialog::OptionsDialogState,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    _movie: Option<&RgbaImage>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let shell = options_layout(viewport_w, viewport_h);
    let dlg = crate::options_dialog::OptionsDialogLayout::new();
    let mut page = RgbaImage::from_raw(
        shell.canvas.w as u32,
        shell.canvas.h as u32,
        vec![0u8; (shell.canvas.w as usize) * (shell.canvas.h as usize) * 4],
    )?;
    // 整页黑底，避免残留主菜单影片/大背景。
    fill_rect(&mut page, shell.canvas, [0, 0, 0, 255]);

    blit_right_panel_top(&mut page, decoded, shell.panel_top, warn_anim_frame);
    if let Some(tile) = find_panel(decoded, "sdbtnbkgd.shp", 0) {
        for i in 0..shell.panel_tile_count {
            let r = RectPx::new(shell.panel_tile.x, shell.panel_tile.y + i * shell.panel_tile.h, shell.panel_tile.w, shell.panel_tile.h);
            blit_stretched(&mut page, &tile.image, r);
        }
    }
    if let Some(bottom) = find_panel(decoded, "sdbtm.shp", 0) {
        blit_stretched(&mut page, &bottom.image, shell.panel_bottom);
    }
    if let Some(lower) = find_panel(decoded, "lwscrnl.shp", 0) {
        blit_stretched(&mut page, &lower.image, shell.lower_strip);
    }

    for (i, entry_id) in OPTIONS_BUTTON_IDS.iter().enumerate() {
        let Some(normal) = find_button_normal(decoded, entry_id)
        else {
            continue;
        };
        let sprite = if pressed_entry_id == Some(entry_id) {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        }
        else if hovered_entry_id == Some(entry_id) {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        }
        else {
            normal
        };
        let cell = shell.buttons[i];
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        if let Some(fnt) = fnt {
            let key = options_csf_label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let pressed = pressed_entry_id == Some(entry_id);
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "options", options_dialog_csf_key("title"));
        blit_caption_in_cell(&mut page, fnt, &title, shell.title.x, shell.title.y, shell.title.w, shell.title.h, MENU_TEXT_SECTION);
    }

    paint_options_dialog_controls(&mut page, &dlg, state, fnt, csf);
    Some(page)
}

/// 合成退出确认：主菜单壳 + 压暗罩 + 居中消息框（确定 / 取消）。
pub fn compose_exit_confirm_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    // 先画完整主菜单壳（右栏六钮仍在），再压暗并叠居中 MessageBox。
    let mut page = compose_shell_menu_page(
        decoded,
        main_menu_layout(viewport_w, viewport_h),
        &MAIN_MENU_BUTTON_IDS,
        None,
        None,
        None,
        fnt,
        csf,
        movie,
        MenuCaptionKind::Main,
        None,
        warn_anim_frame,
    )?;

    let shell = main_menu_layout(viewport_w, viewport_h);
    dim_rect(&mut page, shell.canvas, 160);

    let dlg = exit_confirm_layout(viewport_w, viewport_h);
    if let Some(modal_bg) = find_panel(decoded, "pudlgbgn.shp", 0) {
        blit_rgba(&mut page, &modal_bg.image, dlg.dialog.x, dlg.dialog.y);
    }
    else {
        // 缺底板时不臆造立绘，只留深色框以免完全无反馈。
        fill_rect(&mut page, dlg.dialog, [40, 24, 24, 255]);
    }
    if let Some(fnt) = fnt {
        let prompt = resolve_caption(csf, "exit_confirm", Some(exit_confirm_prompt_csf_key()));
        blit_caption_top_left_clipped(&mut page, fnt, &prompt, dlg.prompt.x, dlg.prompt.y, dlg.prompt.w, dlg.prompt.h, MENU_TEXT_ENABLED);
    }
    for (i, entry_id) in EXIT_CONFIRM_BUTTON_IDS.iter().enumerate() {
        let Some(normal) = find_button_normal(decoded, entry_id)
        else {
            continue;
        };
        let sprite = if pressed_entry_id == Some(entry_id) {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        }
        else if hovered_entry_id == Some(entry_id) {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        }
        else {
            normal
        };
        let cell = dlg.buttons[i];
        // `mnbttn` 自控件 DLU 原点贴齐，不居中缩进。
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        if let Some(fnt) = fnt {
            let key = exit_confirm_csf_label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let pressed = pressed_entry_id == Some(entry_id);
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }
    Some(page)
}

fn stroke_rect(dst: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    fill_rect(dst, RectPx::new(rect.x, rect.y, rect.w, 1), rgba);
    fill_rect(dst, RectPx::new(rect.x, rect.y + rect.h - 1, rect.w, 1), rgba);
    fill_rect(dst, RectPx::new(rect.x, rect.y, 1, rect.h), rgba);
    fill_rect(dst, RectPx::new(rect.x + rect.w - 1, rect.y, 1, rect.h), rgba);
}

fn draw_combo_face(dst: &mut RgbaImage, rect: RectPx, fill: [u8; 4]) {
    fill_rect(dst, rect, [8, 8, 12, 255]);
    stroke_rect(dst, rect, [180, 24, 24, 255]);
    fill_rect(dst, RectPx::new(rect.x + 2, rect.y + 2, (rect.w - 4).max(1), (rect.h - 4).max(1)), fill);
}

fn draw_skirmish_checkbox(dst: &mut RgbaImage, rect: RectPx, checked: bool, chrome: Option<&SkirmishChromeSprites>) {
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

fn draw_skirmish_trackbar(dst: &mut RgbaImage, track: RectPx, pos: i32, max: i32, chrome: Option<&SkirmishChromeSprites>) {
    let caps = chrome.and_then(|c| Some((c.track_cap_l.as_ref()?, c.track_cap_m.as_ref()?, c.track_cap_r.as_ref()?)));
    if let Some((cap_l, cap_m, cap_r)) = caps {
        let h = cap_l.height() as i32;
        let ty = track.y + (track.h - h) / 2;
        let lw = cap_l.width() as i32;
        let rw = cap_r.width() as i32;
        blit_rgba(dst, cap_l, track.x, ty);
        blit_rgba(dst, cap_r, track.x + track.w - rw, ty);
        let mid_w = (track.w - lw - rw).max(1);
        blit_stretched(dst, cap_m, RectPx::new(track.x + lw, ty, mid_w, h));
    }
    else {
        fill_rect(dst, track, [64, 16, 16, 255]);
        let inner = RectPx::new(track.x + 2, track.y + 2, (track.w - 4).max(1), (track.h - 4).max(1));
        fill_rect(dst, inner, [12, 12, 16, 255]);
    }

    let max = max.max(1);
    let thumb_w = chrome.and_then(|c| c.track_thumb.as_ref()).map(|t| t.width() as i32).unwrap_or(10);
    let travel = (track.w - thumb_w).max(1);
    let thumb_x = track.x + (pos.clamp(0, max) * travel) / max;
    if let Some(thumb) = chrome.and_then(|c| c.track_thumb.as_ref()) {
        let ty = track.y + (track.h - thumb.height() as i32) / 2;
        blit_rgba(dst, thumb, thumb_x, ty);
    }
    else {
        let inner = RectPx::new(track.x + 2, track.y + 2, (track.w - 4).max(1), (track.h - 4).max(1));
        fill_rect(dst, RectPx::new(thumb_x, inner.y - 1, thumb_w, inner.h + 2), [220, 40, 40, 255]);
    }
}

fn row_side_name(paint: &SkirmishLobbyPaint<'_>, row: usize) -> &'static str {
    let i = paint.row_side_indices[row.min(paint.row_side_indices.len() - 1)] as usize % LOBBY_SIDES.len();
    LOBBY_SIDES[i]
}

fn row_color_rgb(paint: &SkirmishLobbyPaint<'_>, row: usize) -> [u8; 3] {
    let i = paint.row_color_indices[row.min(paint.row_color_indices.len() - 1)] as usize % LOBBY_COLORS.len();
    LOBBY_COLORS[i]
}

fn row_flag(chrome: Option<&SkirmishChromeSprites>, row: usize) -> Option<&RgbaImage> {
    chrome.and_then(|c| c.row_flags.get(row).and_then(|f| f.as_ref()).or_else(|| if row == 0 { c.flag.as_ref() } else { c.ai_flag.as_ref() }))
}

fn blit_flag(dst: &mut RgbaImage, flag: Option<&RgbaImage>, rect: RectPx) {
    fill_rect(dst, rect, [40, 40, 48, 255]);
    stroke_rect(dst, rect, [180, 24, 24, 255]);
    let Some(img) = flag
    else {
        return;
    };
    let dx = rect.x + (rect.w - img.width() as i32) / 2;
    let dy = rect.y + (rect.h - img.height() as i32) / 2;
    blit_rgba(dst, img, dx, dy);
}

/// 遭遇战 owner-draw 控件精灵（安装内 PCX；缺省时合成回退色块）。
#[derive(Debug, Clone, Default)]
pub struct SkirmishChromeSprites {
    /// 未勾选 `cue_i.pcx`（18×18）。
    pub checkbox_off: Option<RgbaImage>,
    /// 已勾选 `cce_i.pcx`（18×18）。
    pub checkbox_on: Option<RgbaImage>,
    /// 滑条拇指 `trakgrip.pcx`（12×22）。
    pub track_thumb: Option<RgbaImage>,
    /// 滑条左帽 `trofl.pcx`。
    pub track_cap_l: Option<RgbaImage>,
    /// 滑条中段 `trofm.pcx`（按轨宽拉伸）。
    pub track_cap_m: Option<RgbaImage>,
    /// 滑条右帽 `trofr.pcx`。
    pub track_cap_r: Option<RgbaImage>,
    /// 本地玩家旗标。
    pub flag: Option<RgbaImage>,
    /// AI 行旗标（可与本地相同资源）。
    pub ai_flag: Option<RgbaImage>,
    /// 各玩家行旗标（行 0 本地）。
    pub row_flags: [Option<RgbaImage>; ra_layout::ui_layout::SKIRMISH_ROW_COUNT],
}

/// 遭遇战大厅绘制参数（左栏玩家/选项 + 右栏地图名）。
#[derive(Debug, Clone)]
pub struct SkirmishLobbyPaint<'a> {
    /// 当前地图显示名。
    pub map_name: &'a str,
    /// 本地玩家名。
    pub player_name: &'a str,
    /// 本地国家显示名。
    pub country_name: &'a str,
    /// 本地颜色色块。
    pub color_rgb: [u8; 3],
    /// AI 行显示名（难度文案；`ai_rows==0` 时不画）。
    pub ai_name: &'a str,
    /// AI 国家显示名。
    pub ai_country: &'a str,
    /// AI 难度短名（`Easy` / `Normal` / `Hard`）。
    pub ai_difficulty: &'a str,
    /// 可见 AI 行数（0..=7，由地图开局席位推导）。
    pub ai_rows: usize,
    /// 快速游戏。
    pub short_game: bool,
    /// 基地重新部署。
    pub mcv_repacks: bool,
    /// 升级工具箱。
    pub crates: bool,
    /// 超级武器。
    pub superweapons: bool,
    /// 于盟友建造场旁建设。
    pub build_off_ally: bool,
    /// 游戏速度（0..=6）。
    pub game_speed: u8,
    /// 资金。
    pub credits: i32,
    /// 部队数。
    pub unit_count: i32,
    /// 玩家名编辑框是否聚焦。
    pub player_name_editing: bool,
    /// 是否展开国家下拉。
    pub country_combo_open: bool,
    /// 是否展开颜色下拉。
    pub color_combo_open: bool,
    /// 是否展开 AI 难度下拉。
    pub ai_combo_open: bool,
    /// 当前展开下拉所在玩家行。
    pub combo_row: usize,
    /// 各行国家下标（`LOBBY_SIDES`）。
    pub row_side_indices: [u8; ra_layout::ui_layout::SKIRMISH_ROW_COUNT],
    /// 各行色块下标（`LOBBY_COLORS`）。
    pub row_color_indices: [u8; ra_layout::ui_layout::SKIRMISH_ROW_COUNT],
    /// 安装内控件 PCX（可空）。
    pub chrome: Option<&'a SkirmishChromeSprites>,
}

impl Default for SkirmishLobbyPaint<'_> {
    fn default() -> Self {
        Self {
            map_name: "",
            player_name: "Player",
            country_name: "",
            color_rgb: crate::skirmish_setup::LOBBY_COLORS[0],
            ai_name: "",
            ai_country: "",
            ai_difficulty: "Normal",
            ai_rows: 1,
            short_game: true,
            mcv_repacks: true,
            crates: true,
            superweapons: true,
            build_off_ally: false,
            game_speed: 6,
            credits: 10_000,
            unit_count: 10,
            player_name_editing: false,
            country_combo_open: false,
            color_combo_open: false,
            ai_combo_open: false,
            combo_row: 0,
            row_side_indices: [0, 1, 2, 3, 4, 0, 1, 2],
            row_color_indices: [0, 1, 2, 3, 4, 5, 6, 7],
            chrome: None,
        }
    }
}

fn paint_skirmish_lobby_controls(
    page: &mut RgbaImage,
    layout: &SkirmishLobbyLayout,
    paint: &SkirmishLobbyPaint<'_>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
) {
    let label = |kind: &str, fallback: &str| resolve_caption(csf, fallback, skirmish_lobby_static_csf_key(kind));
    let chrome = paint.chrome;

    // 玩家名 / 下拉面 / 色块（本地 + 可选 AI 行）。
    let name_face = if paint.player_name_editing { [40, 40, 56, 255] } else { [16, 16, 20, 255] };
    draw_combo_face(page, layout.player_name, name_face);
    let local_rgb = row_color_rgb(paint, 0);
    draw_combo_face(page, layout.side_faces[0], [16, 16, 20, 255]);
    draw_combo_face(page, layout.color_faces[0], [local_rgb[0], local_rgb[1], local_rgb[2], 255]);
    blit_flag(page, row_flag(chrome, 0), layout.flags[0]);

    let ai_rows = paint.ai_rows.min(layout.ai_faces.len());
    for i in 0..ai_rows {
        draw_combo_face(page, layout.ai_faces[i], [16, 16, 20, 255]);
        let human_row = i + 1;
        if human_row < layout.side_faces.len() {
            draw_combo_face(page, layout.side_faces[human_row], [16, 16, 20, 255]);
        }
        if human_row < layout.color_faces.len() {
            let rgb = row_color_rgb(paint, human_row);
            draw_combo_face(page, layout.color_faces[human_row], [rgb[0], rgb[1], rgb[2], 255]);
        }
        if human_row < layout.flags.len() {
            blit_flag(page, row_flag(chrome, human_row), layout.flags[human_row]);
        }
    }

    let checks = [paint.short_game, paint.mcv_repacks, paint.crates, paint.superweapons, paint.build_off_ally];
    for (i, checked) in checks.iter().enumerate() {
        draw_skirmish_checkbox(page, layout.checkboxes[i], *checked, chrome);
    }

    draw_skirmish_trackbar(page, layout.track_speed, i32::from(paint.game_speed), 6, chrome);
    let credit_pos = (paint.credits / 1000).clamp(0, 10);
    draw_skirmish_trackbar(page, layout.track_credits, credit_pos, 10, chrome);
    draw_skirmish_trackbar(page, layout.track_units, paint.unit_count.clamp(0, 20), 20, chrome);

    if let Some(fnt) = fnt {
        let name_shown = if paint.player_name_editing { format!("{}|", paint.player_name) } else { paint.player_name.to_string() };
        blit_text_colored(
            page,
            fnt,
            &name_shown,
            layout.player_name.x + 4,
            layout.player_name.y + 2,
            if paint.player_name_editing { MENU_TEXT_ACCENT } else { MENU_TEXT_ENABLED },
        );
        let country = row_side_name(paint, 0);
        blit_text_colored(page, fnt, country, layout.side_faces[0].x + 4, layout.side_faces[0].y + 4, MENU_TEXT_ENABLED);

        let ai_label = if paint.ai_name.is_empty() { paint.ai_difficulty.to_string() } else { paint.ai_name.to_string() };
        for i in 0..ai_rows {
            blit_text_colored(page, fnt, &ai_label, layout.ai_faces[i].x + 4, layout.ai_faces[i].y + 4, MENU_TEXT_ENABLED);
            let human_row = i + 1;
            if human_row < layout.side_faces.len() {
                blit_text_colored(
                    page,
                    fnt,
                    row_side_name(paint, human_row),
                    layout.side_faces[human_row].x + 4,
                    layout.side_faces[human_row].y + 4,
                    MENU_TEXT_ENABLED,
                );
            }
        }

        let check_labels = [
            ("short_game", "Short Game"),
            ("mcv_repacks", "MCV Repacks"),
            ("crates", "Crates Appear"),
            ("superweapons", "Super Weapons"),
            ("build_off_ally", "Build Off Ally"),
        ];
        for (i, (key, fb)) in check_labels.iter().enumerate() {
            let r = layout.checkboxes[i];
            blit_text_colored(page, fnt, &label(key, fb), r.x + SKIRMISH_CHECK_W + 8, r.y + 1, MENU_TEXT_ENABLED);
        }

        blit_text_colored(page, fnt, &label("game_speed", "Game Speed"), layout.label_speed.x, layout.label_speed.y, MENU_TEXT_ENABLED);
        blit_text_colored(page, fnt, &label("credits", "Credits"), layout.label_credits.x, layout.label_credits.y, MENU_TEXT_ENABLED);
        blit_text_colored(page, fnt, &label("unit_count", "Unit Count"), layout.label_units.x, layout.label_units.y, MENU_TEXT_ENABLED);
        blit_text_colored(
            page,
            fnt,
            &paint.game_speed.to_string(),
            layout.track_speed.x + layout.track_speed.w - 28,
            layout.track_speed.y + 2,
            MENU_TEXT_ENABLED,
        );
        blit_text_colored(
            page,
            fnt,
            &paint.credits.to_string(),
            layout.track_credits.x + layout.track_credits.w - 48,
            layout.track_credits.y + 2,
            MENU_TEXT_ENABLED,
        );
        blit_text_colored(
            page,
            fnt,
            &paint.unit_count.to_string(),
            layout.track_units.x + layout.track_units.w - 28,
            layout.track_units.y + 2,
            MENU_TEXT_ENABLED,
        );
    }

    if paint.country_combo_open {
        let list = crate::skirmish_setup::SkirmishBootRequest::country_list_rect(layout, paint.combo_row);
        fill_rect(page, list, [12, 12, 18, 255]);
        stroke_rect(page, list, [180, 24, 24, 255]);
        let selected_side = row_side_name(paint, paint.combo_row);
        for (i, side) in LOBBY_SIDES.iter().enumerate() {
            let row = RectPx::new(list.x, list.y + (i as i32) * SKIRMISH_COMBO_FACE_H, list.w, SKIRMISH_COMBO_FACE_H);
            let selected = selected_side.eq_ignore_ascii_case(side);
            if selected {
                fill_rect(page, row, [48, 28, 8, 255]);
            }
            if let Some(fnt) = fnt {
                blit_text_colored(page, fnt, side, row.x + 4, row.y + 4, if selected { MENU_TEXT_ACCENT } else { MENU_TEXT_ENABLED });
            }
        }
    }

    if paint.color_combo_open {
        let list = crate::skirmish_setup::SkirmishBootRequest::color_list_rect(layout, paint.combo_row);
        fill_rect(page, list, [12, 12, 18, 255]);
        stroke_rect(page, list, [180, 24, 24, 255]);
        let selected_rgb = row_color_rgb(paint, paint.combo_row);
        for (i, rgb) in LOBBY_COLORS.iter().enumerate() {
            let row = RectPx::new(list.x, list.y + (i as i32) * SKIRMISH_COMBO_FACE_H, list.w, SKIRMISH_COMBO_FACE_H);
            let swatch = RectPx::new(row.x + 4, row.y + 4, row.w - 8, row.h - 8);
            fill_rect(page, swatch, [rgb[0], rgb[1], rgb[2], 255]);
            if selected_rgb == *rgb {
                stroke_rect(page, swatch, [255, 214, 0, 255]);
            }
        }
    }

    if paint.ai_combo_open {
        let list = crate::skirmish_setup::SkirmishBootRequest::ai_list_rect(layout);
        fill_rect(page, list, [12, 12, 18, 255]);
        stroke_rect(page, list, [180, 24, 24, 255]);
        for (i, diff) in LOBBY_DIFFICULTIES.iter().enumerate() {
            let row = RectPx::new(list.x, list.y + (i as i32) * SKIRMISH_COMBO_FACE_H, list.w, SKIRMISH_COMBO_FACE_H);
            let selected = paint.ai_difficulty.eq_ignore_ascii_case(diff);
            if selected {
                fill_rect(page, row, [48, 28, 8, 255]);
            }
            if let Some(fnt) = fnt {
                let label = resolve_caption(csf, diff, Some(crate::skirmish_setup::SkirmishBootRequest::ai_difficulty_csf_key(diff)));
                blit_text_colored(page, fnt, &label, row.x + 4, row.y + 4, if selected { MENU_TEXT_ACCENT } else { MENU_TEXT_ENABLED });
            }
        }
    }
}

/// 合成遭遇战大厅：右栏预览/地图名 + 左栏玩家与选项（非左侧地图列表）。
pub fn compose_skirmish_lobby_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    map_preview: Option<&RgbaImage>,
    paint: &SkirmishLobbyPaint<'_>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let layout = skirmish_lobby_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
        &SKIRMISH_LOBBY_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        None,
        fnt,
        csf,
        None,
        MenuCaptionKind::SkirmishLobby,
        wave,
        warn_anim_frame,
    )?;

    // 右栏：小地图预览盖住 WARNING 区；标题 / 作战 / 地图名。
    fill_rect(&mut page, layout.map_preview, [8, 10, 16, 255]);
    stroke_rect(&mut page, layout.map_preview, [180, 24, 24, 255]);
    if let Some(preview) = map_preview {
        blit_stretched(&mut page, preview, layout.map_preview);
    }
    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "skirmish", Some(skirmish_title_csf_key()));
        blit_caption_in_cell(&mut page, fnt, &title, layout.title.x, layout.title.y, layout.title.w, layout.title.h, MENU_TEXT_ENABLED);
        let battle = resolve_caption(csf, "battle", skirmish_lobby_static_csf_key("battle"));
        blit_text_colored(&mut page, fnt, &battle, layout.game_type.x, layout.game_type.y, MENU_TEXT_ENABLED);
        if !paint.map_name.is_empty() {
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                paint.map_name,
                layout.map_label.x,
                layout.map_label.y,
                layout.map_label.w,
                layout.map_label.h,
                MENU_TEXT_ENABLED,
            );
        }
    }

    paint_skirmish_lobby_controls(&mut page, &layout, paint, fnt, csf);

    // 底栏状态提示：壳层打字机可见切片。
    if let (Some(fnt), Some(text)) = (fnt, status_text.filter(|s| !s.is_empty())) {
        blit_text_colored(&mut page, fnt, text, layout.status_help.x, layout.status_help.y, MENU_TEXT_ENABLED);
    }

    Some(page)
}

/// 选图页列表行高（像素）。
const CHOOSE_MAP_LIST_ROW_H: i32 = 16;

/// 合成选图页：双列表 + 右栏预览 / 使用地图 / 随机 / 取消。
pub fn compose_choose_map_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    map_preview: Option<&RgbaImage>,
    map_names: &[&str],
    selected_map_index: Option<usize>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let layout = choose_map_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
        &CHOOSE_MAP_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        status_text,
        fnt,
        csf,
        None,
        MenuCaptionKind::ChooseMap,
        wave,
        warn_anim_frame,
    )?;

    fill_rect(&mut page, layout.map_preview, [8, 10, 16, 255]);
    stroke_rect(&mut page, layout.map_preview, [180, 24, 24, 255]);
    if let Some(preview) = map_preview {
        blit_stretched(&mut page, preview, layout.map_preview);
    }

    fill_rect(&mut page, layout.game_type_list, [12, 12, 18, 255]);
    stroke_rect(&mut page, layout.game_type_list, [180, 24, 24, 255]);
    fill_rect(&mut page, layout.map_list, [12, 12, 18, 255]);
    stroke_rect(&mut page, layout.map_list, [180, 24, 24, 255]);

    // 游戏类型：Pre-Alpha 仅「作战」一项。
    let type_row = RectPx::new(layout.game_type_list.x, layout.game_type_list.y, layout.game_type_list.w, CHOOSE_MAP_LIST_ROW_H);
    fill_rect(&mut page, type_row, [48, 28, 8, 255]);

    let visible_rows = (layout.map_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (i, name) in map_names.iter().take(visible_rows).enumerate() {
        let row =
            RectPx::new(layout.map_list.x, layout.map_list.y + (i as i32) * CHOOSE_MAP_LIST_ROW_H, layout.map_list.w, CHOOSE_MAP_LIST_ROW_H);
        if Some(i) == selected_map_index {
            fill_rect(&mut page, row, [48, 28, 8, 255]);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(&mut page, fnt, name, row.x + 4, row.y + 1, row.w - 8, row.h - 2, MENU_TEXT_ENABLED);
        }
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "choose_map", Some(choose_map_title_csf_key()));
        blit_caption_in_cell(&mut page, fnt, &title, layout.title.x, layout.title.y, layout.title.w, layout.title.h, MENU_TEXT_ENABLED);
        let engagement = resolve_caption(csf, "select_engagement", choose_map_static_csf_key("select_engagement"));
        blit_text_colored(&mut page, fnt, &engagement, layout.label_engagement.x, layout.label_engagement.y, MENU_TEXT_ENABLED);
        let game_type = resolve_caption(csf, "game_type", choose_map_static_csf_key("game_type"));
        blit_text_colored(&mut page, fnt, &game_type, layout.label_game_type.x, layout.label_game_type.y, MENU_TEXT_ENABLED);
        let game_map = resolve_caption(csf, "game_map", choose_map_static_csf_key("game_map"));
        blit_text_colored(&mut page, fnt, &game_map, layout.label_game_map.x, layout.label_game_map.y, MENU_TEXT_ENABLED);
        let battle = resolve_caption(csf, "battle", choose_map_static_csf_key("battle"));
        blit_text_colored(&mut page, fnt, &battle, type_row.x + 4, type_row.y + 1, MENU_TEXT_ENABLED);
    }

    Some(page)
}

/// 装载页绘制输入（国家艺术 + 进度 + 失败操作）。
#[derive(Debug, Clone, Copy)]
pub struct LoadScreenPaint<'a> {
    /// 底栏状态（装载中或失败说明；失败时才强调）。
    pub status: &'a str,
    /// 是否允许「重试」（装载线程进行中为 false；失败后为 true）。
    pub allow_retry: bool,
    /// 装载进度 0..=1（驱动 `progbarm` 横向裁剪）。
    pub progress: f32,
}

/// 800×600 基准上的进度条原点（贴国家艺术图预留槽）。
const LOAD_PROG_X_800: i32 = 48;
const LOAD_PROG_Y_800: i32 = 101;

/// 合成遭遇战装载页：国家 `ls*` 全幅 + `progbarm` 裁剪填充；失败时重试/取消。
pub fn compose_load_screen_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
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

    let ratio = paint.progress.clamp(0.0, 1.0);
    if let Some(bar) = find_panel(decoded, "progbarm.shp", 0) {
        let sx = layout.canvas.w as f32 / 800.0;
        let sy = layout.canvas.h as f32 / 600.0;
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

fn blit_rgba_clipped_width(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, clip_w: u32) {
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

fn slots_load_button_hit(entry_id: &str) -> Option<(f32, f32, f32, f32)> {
    match entry_id {
        "retry" => Some((0.30, 0.88, 0.50, 0.96)),
        "cancel" => Some((0.54, 0.88, 0.74, 0.96)),
        _ => None,
    }
}

fn hit_to_rect(canvas: RectPx, hit: (f32, f32, f32, f32)) -> RectPx {
    let x0 = canvas.x + (canvas.w as f32 * hit.0) as i32;
    let y0 = canvas.y + (canvas.h as f32 * hit.1) as i32;
    let x1 = canvas.x + (canvas.w as f32 * hit.2) as i32;
    let y1 = canvas.y + (canvas.h as f32 * hit.3) as i32;
    RectPx::new(x0, y0, (x1 - x0).max(1), (y1 - y0).max(1))
}

/// 对局 HUD 侧栏绘制输入（由宿主从 `HudSnapshot` 投影，组件不依赖 `ra-engine`）。
#[derive(Debug, Clone, Copy)]
pub struct MatchHudPaint<'a> {
    /// 仿真 tick。
    pub tick: u64,
    /// 本地资金。
    pub funds: i32,
    /// 供电。
    pub power_output: i32,
    /// 耗电。
    pub power_drain: i32,
    /// 是否低电。
    pub low_power: bool,
    /// 选中摘要（如 `#3` 或 `#3+2`）。
    pub selected_summary: &'a str,
    /// 生产队列首项文案（可空）。
    pub produce_queue: Option<&'a str>,
    /// 最近命令拒绝原因（可空）。
    pub reject: Option<&'a str>,
    /// 是否暂停。
    pub paused: bool,
    /// 暂停原因。
    pub pause_reason: Option<&'a str>,
    /// 结算文案（可空）。
    pub outcome: Option<&'a str>,
}

/// 合成对局 HUD 叠加层：左透明、右 `RIGHT_PANEL_W` 实心栏。
pub fn compose_match_hud_overlay(viewport_w: u32, viewport_h: u32, fnt: Option<&FntFile>, paint: MatchHudPaint<'_>) -> Option<RgbaImage> {
    let w = viewport_w.max(1);
    let h = viewport_h.max(1);
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;
    let panel_w = RIGHT_PANEL_W.min(w as i32).max(1);
    let panel_x = (w as i32 - panel_w).max(0);
    let panel = RectPx::new(panel_x, 0, panel_w, h as i32);
    fill_rect(&mut page, panel, [28, 32, 40, 230]);
    stroke_rect(&mut page, panel, [180, 40, 40, 255]);

    // 顶部资金 / 电力。
    let econ = RectPx::new(panel.x + 8, 12, panel.w - 16, 48);
    fill_rect(&mut page, econ, [16, 18, 24, 255]);
    stroke_rect(&mut page, econ, [120, 28, 28, 255]);
    let funds_line = format!("$ {}", paint.funds);
    let power_mark = if paint.low_power { "!" } else { "" };
    let power_line = format!("电 {}/{}{power_mark}", paint.power_output, paint.power_drain);
    if let Some(fnt) = fnt {
        blit_text_colored(&mut page, fnt, &funds_line, econ.x + 6, econ.y + 6, MENU_TEXT_ACCENT);
        let power_color = if paint.low_power { [255, 80, 80, 255] } else { MENU_TEXT_ENABLED };
        blit_text_colored(&mut page, fnt, &power_line, econ.x + 6, econ.y + 26, power_color);
    }

    // 选中 / 队列 / 拒绝。
    let mut y = econ.y + econ.h + 12;
    let line_h = 18;
    let text_x = panel.x + 8;
    let text_w = panel.w - 16;
    if let Some(fnt) = fnt {
        blit_caption_top_left_clipped(&mut page, fnt, &format!("选中 {}", paint.selected_summary), text_x, y, text_w, line_h, MENU_TEXT_ENABLED);
        y += line_h + 4;
        let queue = paint.produce_queue.unwrap_or("队列 —");
        blit_caption_top_left_clipped(&mut page, fnt, queue, text_x, y, text_w, line_h, MENU_TEXT_ENABLED);
        y += line_h + 4;
        if let Some(reject) = paint.reject {
            blit_caption_top_left_clipped(&mut page, fnt, reject, text_x, y, text_w, line_h, [255, 120, 80, 255]);
            y += line_h + 8;
        }
        else {
            y += 8;
        }
        blit_caption_top_left_clipped(&mut page, fnt, &format!("t{}", paint.tick), text_x, y, text_w, line_h, MENU_TEXT_SECTION);
        y += line_h + 8;
        if paint.paused {
            let reason = paint.pause_reason.unwrap_or("已暂停");
            blit_caption_top_left_clipped(&mut page, fnt, reason, text_x, y, text_w, line_h, MENU_TEXT_ACCENT);
            y += line_h + 8;
        }
        if let Some(outcome) = paint.outcome {
            blit_caption_top_left_clipped(&mut page, fnt, outcome, text_x, y, text_w, line_h, MENU_TEXT_ACCENT);
            y += line_h + 8;
        }
    }

    // 建造 cameo 占位格（本轮不接线建造命令）。
    let grid_top = y.max(panel.y + 160);
    let cell = 48;
    let gap = 4;
    let cols = ((panel.w - 16) / (cell + gap)).max(1);
    for i in 0..8 {
        let col = i % cols;
        let row = i / cols;
        let cx = panel.x + 8 + col * (cell + gap);
        let cy = grid_top + row * (cell + gap);
        if cy + cell > panel.y + panel.h - 8 {
            break;
        }
        let r = RectPx::new(cx, cy, cell, cell);
        fill_rect(&mut page, r, [20, 22, 28, 255]);
        stroke_rect(&mut page, r, [90, 30, 30, 255]);
    }

    Some(page)
}
