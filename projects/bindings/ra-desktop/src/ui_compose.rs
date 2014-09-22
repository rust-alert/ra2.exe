//! 将已解码壳层精灵合成整页 RGBA（上传 `set_ui_page` 之前）。
//!
//! 合成 ≠ atlas/instance 终态；当前只为验证颜色、原尺寸与粗略位置。

use ra_assets::{CsfFile, FntFile};
use ra_renderer::RgbaImage;

use crate::{
    ui_decode::{DecodedUiSprite, PageDecodeReport},
    ui_layout::{
        CAMPAIGN_BUTTON_IDS, CHOOSE_MAP_BUTTON_IDS, EXIT_CONFIRM_BUTTON_IDS, MAIN_MENU_BUTTON_IDS, MainMenuLayout,
        OPTIONS_BUTTON_IDS, RectPx, SDWRNANM_OFFSET_X, SDWRNANM_OFFSET_Y, SINGLE_PLAYER_BUTTON_IDS,
        SKIRMISH_CHECK_H, SKIRMISH_CHECK_W, SKIRMISH_LOBBY_BUTTON_IDS, SkirmishLobbyLayout,
        campaign_layout, choose_map_layout, exit_confirm_layout, main_menu_layout, options_layout,
        single_player_layout, skirmish_lobby_layout,
    },
    ui_text::{
        MENU_TEXT_ACCENT, MENU_TEXT_DISABLED, MENU_TEXT_ENABLED, MENU_TEXT_SECTION, blit_caption_in_cell,
        blit_caption_top_left_clipped, blit_text_colored, campaign_csf_label, campaign_csf_tooltip,
        campaign_difficulty_csf_key, campaign_title_csf_key, choose_map_csf_label, choose_map_static_csf_key,
        choose_map_title_csf_key, exit_confirm_csf_label, exit_confirm_prompt_csf_key, main_menu_csf_label,
        main_menu_csf_tooltip, options_csf_label, options_dialog_csf_key, resolve_caption, resolve_csf_text,
        single_player_csf_label, single_player_csf_tooltip, single_player_title_csf_key, skirmish_lobby_csf_label,
        skirmish_lobby_csf_tooltip, skirmish_lobby_static_csf_key, skirmish_title_csf_key,
    },
};

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
    fill_rect(
        dst,
        RectPx::new(box_r.x + 2, box_r.y + 2, 12, 12),
        [12, 12, 16, 255],
    );
    if checked {
        fill_rect(
            dst,
            RectPx::new(box_r.x + 4, box_r.y + 4, 8, 8),
            [220, 40, 40, 255],
        );
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
    fill_rect(
        page,
        RectPx::new(layout.content.x + 2, layout.content.y + 2, layout.content.w - 4, layout.content.h - 4),
        [18, 22, 32, 255],
    );

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
        blit_text_colored(
            page,
            fnt,
            &label("scroll", "Scroll Rate"),
            layout.track_scroll.x,
            layout.track_scroll.y - 16,
            MENU_TEXT_ACCENT,
        );
        blit_text_colored(
            page,
            fnt,
            &label("fastest", "Fastest"),
            layout.track_scroll.x + layout.track_scroll.w + 8,
            layout.track_scroll.y + 2,
            MENU_TEXT_ACCENT,
        );

        blit_text_colored(page, fnt, &label("audio", "Audio Options"), layout.sec_audio.x, layout.sec_audio.y, MENU_TEXT_SECTION);
        draw_section_rule(page, layout.sec_audio);
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
        blit_text_colored(page, fnt, &label("tooltips", "Tooltips"), tx, layout.checks[0].y + 4, MENU_TEXT_ACCENT);
        blit_text_colored(page, fnt, &label("scanlines", "Target Lines"), tx, layout.checks[1].y + 4, MENU_TEXT_ACCENT);
        blit_text_colored(page, fnt, &label("damage", "See Hidden Objects"), tx, layout.checks[2].y + 4, MENU_TEXT_ACCENT);
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
        blit_text_colored(
            page,
            fnt,
            state.display_mode.as_str(),
            layout.resolution.x + 8,
            layout.resolution.y + 6,
            MENU_TEXT_ACCENT,
        );
    }
    if state.resolution_open {
        for (i, mode) in ra_types::DisplayMode::ALL.iter().enumerate() {
            let row = layout.resolution_row(i);
            let bg = if *mode == state.display_mode {
                [90, 40, 20, 255]
            } else {
                [28, 28, 34, 255]
            };
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

/// 右栏顶盖：`sdtp` 外壳固定帧 0，再叠 `sdwrnanm` WARNING 屏动画。
fn blit_right_panel_top(
    page: &mut RgbaImage,
    decoded: &PageDecodeReport,
    panel_top: RectPx,
    warn_anim_frame: usize,
) {
    if let Some(top) = find_panel(decoded, "sdtp.shp", 0) {
        blit_stretched(page, &top.image, panel_top);
    }
    if let Some(warn) = find_panel(decoded, "sdwrnanm.shp", warn_anim_frame) {
        blit_rgba(
            page,
            &warn.image,
            panel_top.x + SDWRNANM_OFFSET_X,
            panel_top.y + SDWRNANM_OFFSET_Y,
        );
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
    (
        cell.x + dx,
        cell.y + dy,
        (cell.w - 2 - dx).max(0),
        (cell.h - dy).max(0),
    )
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
    panel_anim_frame: usize,
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

    blit_right_panel_top(&mut page, decoded, layout.panel_top, panel_anim_frame);
    if let Some(tile) = find_panel(decoded, "sdbtnbkgd.shp", 0) {
        for i in 0..layout.panel_tile_count {
            let r = RectPx::new(layout.panel_tile.x, layout.panel_tile.y + i * layout.panel_tile.h, layout.panel_tile.w, layout.panel_tile.h);
            blit_stretched(&mut page, &tile.image, r);
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
        let sprite = if pressed_entry_id == Some(*entry_id) && !disabled {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        } else if hovered_entry_id == Some(*entry_id) && !disabled {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        } else {
            normal
        };
        let cell = layout.buttons[i];
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        // `sdbtnanm` 禁用帧暂与常态同号；压暗格面，避免「载入」等禁用钮仍像高亮可点。
        if disabled {
            dim_rect(&mut page, cell, 110);
        }
        if let Some(fnt) = fnt {
            let key = captions.label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let color = if disabled { MENU_TEXT_DISABLED } else { MENU_TEXT_ENABLED };
            let pressed = pressed_entry_id == Some(*entry_id) && !disabled;
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, color);
        }
    }

    if let Some(fnt) = fnt {
        let title = match captions {
            MenuCaptionKind::Main => Some(resolve_caption(csf, "main_menu", Some("GUI:MainMenu"))),
            MenuCaptionKind::SinglePlayer => {
                Some(resolve_caption(csf, "single_player", Some(single_player_title_csf_key())))
            }
            // 战役 / 遭遇战 / 选图标题由各自 compose 按对话框锚点另画。
            MenuCaptionKind::Campaign | MenuCaptionKind::SkirmishLobby | MenuCaptionKind::ChooseMap => None,
        };
        if let Some(title) = title {
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
        }
        let tooltip_key = match (captions, hovered_entry_id) {
            (MenuCaptionKind::Main, Some(hovered)) => main_menu_csf_tooltip(hovered),
            (MenuCaptionKind::SinglePlayer, Some(hovered)) => single_player_csf_tooltip(hovered),
            // 战役底栏提示用壳层 `tooltip` 锚点，在 `compose_campaign_page` 另画。
            _ => None,
        };
        if let Some(key) = tooltip_key {
            if let Some(text) = resolve_csf_text(csf, key) {
                blit_text_colored(&mut page, fnt, &text, layout.tooltip.x, layout.tooltip.y, MENU_TEXT_ENABLED);
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
    panel_anim_frame: usize,
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
        panel_anim_frame,
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
    panel_anim_frame: usize,
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
        panel_anim_frame,
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
}

impl Default for CampaignPaint<'_> {
    fn default() -> Self {
        Self { selected_side: None, difficulty: 1, track_thumb: None }
    }
}

/// 合成战役选边页：三侧图 + 难度 + 右栏载入/返回。
pub fn compose_campaign_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    paint: CampaignPaint<'_>,
    panel_anim_frame: usize,
) -> Option<RgbaImage> {
    let layout = campaign_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
        &CAMPAIGN_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        None,
        MenuCaptionKind::Campaign,
        panel_anim_frame,
    )?;

    let sides = [
        ("allied", "fsalg.shp", layout.allied),
        ("tutorial", "fsbclg.shp", layout.tutorial),
        ("soviet", "fsslg.shp", layout.soviet),
    ];
    for (id, shp, rect) in sides {
        // 侧图不得与 `sdwrnanm` 共用 `panel_anim_frame`：WARNING 帧数远多于侧图，
        // 取模会抽到错误高亮/箭头帧，观感像调色板错了。悬停动画另计时后再接。
        if let Some(sprite) = find_panel(decoded, shp, 0) {
            blit_stretched(&mut page, &sprite.image, rect);
        }
        let selected = paint.selected_side == Some(id);
        let hovered = hovered_entry_id == Some(id);
        if selected || hovered {
            let color = if selected { [255, 214, 0, 255] } else { [180, 24, 24, 255] };
            stroke_rect(&mut page, rect, color);
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
    } else {
        fill_rect(
            &mut page,
            RectPx::new(thumb_x, inner.y - 1, thumb_w, inner.h + 2),
            [220, 40, 40, 255],
        );
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "campaign", Some(campaign_title_csf_key()));
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
        let diff_label = resolve_caption(csf, "difficulty", Some("GUI:Difficulty"));
        blit_text_colored(
            &mut page,
            fnt,
            &diff_label,
            layout.difficulty_label.x,
            layout.difficulty_label.y,
            MENU_TEXT_ENABLED,
        );
        let diff_value = resolve_caption(csf, "difficulty_value", Some(campaign_difficulty_csf_key(paint.difficulty)));
        blit_text_colored(
            &mut page,
            fnt,
            &diff_value,
            layout.difficulty_value.x,
            layout.difficulty_value.y,
            MENU_TEXT_ENABLED,
        );
        if let Some(hovered) = hovered_entry_id {
            if let Some(key) = campaign_csf_tooltip(hovered) {
                if let Some(text) = resolve_csf_text(csf, key) {
                    blit_text_colored(
                        &mut page,
                        fnt,
                        &text,
                        layout.status_help.x,
                        layout.status_help.y,
                        MENU_TEXT_ENABLED,
                    );
                }
            }
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
    panel_anim_frame: usize,
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

    blit_right_panel_top(&mut page, decoded, shell.panel_top, panel_anim_frame);
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
        let sprite = if pressed_entry_id == Some(*entry_id) {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        } else if hovered_entry_id == Some(*entry_id) {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        } else {
            normal
        };
        let cell = shell.buttons[i];
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        if let Some(fnt) = fnt {
            let key = options_csf_label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let pressed = pressed_entry_id == Some(*entry_id);
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "options", options_dialog_csf_key("title"));
        blit_caption_in_cell(
            &mut page,
            fnt,
            &title,
            shell.title.x,
            shell.title.y,
            shell.title.w,
            shell.title.h,
            MENU_TEXT_SECTION,
        );
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
    panel_anim_frame: usize,
) -> Option<RgbaImage> {
    // 先画完整主菜单壳（右栏六钮仍在），再压暗并叠居中 MessageBox。
    let mut page = compose_shell_menu_page(
        decoded,
        main_menu_layout(viewport_w, viewport_h),
        &MAIN_MENU_BUTTON_IDS,
        None,
        None,
        fnt,
        csf,
        movie,
        MenuCaptionKind::Main,
        panel_anim_frame,
    )?;

    let shell = main_menu_layout(viewport_w, viewport_h);
    dim_rect(&mut page, shell.canvas, 160);

    let dlg = exit_confirm_layout(viewport_w, viewport_h);
    if let Some(modal_bg) = find_panel(decoded, "pudlgbgn.shp", 0) {
        blit_rgba(&mut page, &modal_bg.image, dlg.dialog.x, dlg.dialog.y);
    } else {
        // 缺底板时不臆造立绘，只留深色框以免完全无反馈。
        fill_rect(&mut page, dlg.dialog, [40, 24, 24, 255]);
    }
    if let Some(fnt) = fnt {
        let prompt = resolve_caption(csf, "exit_confirm", Some(exit_confirm_prompt_csf_key()));
        blit_caption_top_left_clipped(
            &mut page,
            fnt,
            &prompt,
            dlg.prompt.x,
            dlg.prompt.y,
            dlg.prompt.w,
            dlg.prompt.h,
            MENU_TEXT_ENABLED,
        );
    }
    for (i, entry_id) in EXIT_CONFIRM_BUTTON_IDS.iter().enumerate() {
        let Some(normal) = find_button_normal(decoded, entry_id)
        else {
            continue;
        };
        let sprite = if pressed_entry_id == Some(*entry_id) {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        } else if hovered_entry_id == Some(*entry_id) {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        } else {
            normal
        };
        let cell = dlg.buttons[i];
        // `mnbttn` 自控件 DLU 原点贴齐，不居中缩进。
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        if let Some(fnt) = fnt {
            let key = exit_confirm_csf_label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let pressed = pressed_entry_id == Some(*entry_id);
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
    fill_rect(
        dst,
        RectPx::new(rect.x + 2, rect.y + 2, (rect.w - 4).max(1), (rect.h - 4).max(1)),
        fill,
    );
}

fn draw_skirmish_checkbox(dst: &mut RgbaImage, rect: RectPx, checked: bool, chrome: Option<&SkirmishChromeSprites>) {
    let box_r = RectPx::new(rect.x, rect.y, SKIRMISH_CHECK_W, SKIRMISH_CHECK_H.min(rect.h.max(SKIRMISH_CHECK_H)));
    if let Some(img) = chrome.and_then(|c| if checked { c.checkbox_on.as_ref() } else { c.checkbox_off.as_ref() }) {
        blit_rgba(dst, img, box_r.x, box_r.y);
        return;
    }
    fill_rect(dst, box_r, [90, 20, 20, 255]);
    stroke_rect(dst, box_r, [200, 40, 40, 255]);
    fill_rect(
        dst,
        RectPx::new(box_r.x + 2, box_r.y + 2, box_r.w - 4, box_r.h - 4),
        [12, 12, 16, 255],
    );
    if checked {
        fill_rect(
            dst,
            RectPx::new(box_r.x + 5, box_r.y + 5, 8, 8),
            [255, 160, 32, 255],
        );
    }
}

fn draw_skirmish_trackbar(
    dst: &mut RgbaImage,
    track: RectPx,
    pos: i32,
    max: i32,
    chrome: Option<&SkirmishChromeSprites>,
) {
    fill_rect(dst, track, [64, 16, 16, 255]);
    let inner = RectPx::new(track.x + 2, track.y + 2, (track.w - 4).max(1), (track.h - 4).max(1));
    fill_rect(dst, inner, [12, 12, 16, 255]);
    let max = max.max(1);
    let thumb_w = chrome.and_then(|c| c.track_thumb.as_ref()).map(|t| t.width() as i32).unwrap_or(10);
    let travel = (inner.w - thumb_w).max(1);
    let thumb_x = inner.x + (pos.clamp(0, max) * travel) / max;
    if let Some(thumb) = chrome.and_then(|c| c.track_thumb.as_ref()) {
        let ty = track.y + (track.h - thumb.height() as i32) / 2;
        blit_rgba(dst, thumb, thumb_x, ty);
    } else {
        fill_rect(dst, RectPx::new(thumb_x, inner.y - 1, thumb_w, inner.h + 2), [220, 40, 40, 255]);
    }
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
    /// 本地玩家旗标。
    pub flag: Option<RgbaImage>,
    /// AI 行旗标（可与本地相同资源）。
    pub ai_flag: Option<RgbaImage>,
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
    /// AI 行显示名（空则不画第二行）。
    pub ai_name: &'a str,
    /// AI 国家显示名。
    pub ai_country: &'a str,
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
    /// 安装内控件 PCX（可空）。
    pub chrome: Option<&'a SkirmishChromeSprites>,
}

impl Default for SkirmishLobbyPaint<'_> {
    fn default() -> Self {
        Self {
            map_name: "",
            player_name: "Player",
            country_name: "",
            color_rgb: [0, 160, 0],
            ai_name: "",
            ai_country: "",
            short_game: true,
            mcv_repacks: true,
            crates: true,
            superweapons: true,
            build_off_ally: false,
            game_speed: 6,
            credits: 10_000,
            unit_count: 10,
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
    draw_combo_face(page, layout.player_name, [16, 16, 20, 255]);
    draw_combo_face(page, layout.side_faces[0], [16, 16, 20, 255]);
    draw_combo_face(page, layout.color_faces[0], [paint.color_rgb[0], paint.color_rgb[1], paint.color_rgb[2], 255]);
    blit_flag(page, chrome.and_then(|c| c.flag.as_ref()), layout.flags[0]);

    if !paint.ai_name.is_empty() {
        draw_combo_face(page, layout.ai_faces[0], [16, 16, 20, 255]);
        draw_combo_face(page, layout.side_faces[1], [16, 16, 20, 255]);
        draw_combo_face(page, layout.color_faces[1], [180, 40, 40, 255]);
        blit_flag(page, chrome.and_then(|c| c.ai_flag.as_ref()), layout.flags[1]);
    }

    let checks = [
        paint.short_game,
        paint.mcv_repacks,
        paint.crates,
        paint.superweapons,
        paint.build_off_ally,
    ];
    for (i, checked) in checks.iter().enumerate() {
        draw_skirmish_checkbox(page, layout.checkboxes[i], *checked, chrome);
    }

    draw_skirmish_trackbar(page, layout.track_speed, i32::from(paint.game_speed), 6, chrome);
    let credit_pos = (paint.credits / 1000).clamp(0, 10);
    draw_skirmish_trackbar(page, layout.track_credits, credit_pos, 10, chrome);
    draw_skirmish_trackbar(page, layout.track_units, paint.unit_count.clamp(0, 20), 20, chrome);

    if let Some(fnt) = fnt {
        blit_text_colored(page, fnt, paint.player_name, layout.player_name.x + 4, layout.player_name.y + 2, MENU_TEXT_ENABLED);
        let country = if paint.country_name.is_empty() {
            label("side", "Side")
        } else {
            paint.country_name.to_string()
        };
        blit_text_colored(page, fnt, &country, layout.side_faces[0].x + 4, layout.side_faces[0].y + 4, MENU_TEXT_ENABLED);

        if !paint.ai_name.is_empty() {
            blit_text_colored(page, fnt, paint.ai_name, layout.ai_faces[0].x + 4, layout.ai_faces[0].y + 4, MENU_TEXT_ENABLED);
            let ai_country = if paint.ai_country.is_empty() {
                label("ai_hard", "Hard")
            } else {
                paint.ai_country.to_string()
            };
            blit_text_colored(page, fnt, &ai_country, layout.side_faces[1].x + 4, layout.side_faces[1].y + 4, MENU_TEXT_ENABLED);
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
            blit_text_colored(
                page,
                fnt,
                &label(key, fb),
                r.x + SKIRMISH_CHECK_W + 8,
                r.y + 1,
                MENU_TEXT_ENABLED,
            );
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
}

/// 合成遭遇战大厅：右栏预览/地图名 + 左栏玩家与选项（非左侧地图列表）。
pub fn compose_skirmish_lobby_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    map_preview: Option<&RgbaImage>,
    paint: &SkirmishLobbyPaint<'_>,
    panel_anim_frame: usize,
) -> Option<RgbaImage> {
    let layout = skirmish_lobby_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
        &SKIRMISH_LOBBY_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        None,
        MenuCaptionKind::SkirmishLobby,
        panel_anim_frame,
    )?;

    // 右栏：小地图预览盖住 WARNING 区；标题 / 作战 / 地图名。
    fill_rect(&mut page, layout.map_preview, [8, 10, 16, 255]);
    stroke_rect(&mut page, layout.map_preview, [180, 24, 24, 255]);
    if let Some(preview) = map_preview {
        blit_stretched(&mut page, preview, layout.map_preview);
    }
    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "skirmish", Some(skirmish_title_csf_key()));
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

    // 底栏 `0x695`：悬停右栏钮或左栏控件时的 `STT:Skirmish*`。
    if let (Some(fnt), Some(hovered)) = (fnt, hovered_entry_id) {
        if let Some(key) = skirmish_lobby_csf_tooltip(hovered) {
            if let Some(text) = resolve_csf_text(csf, key) {
                blit_text_colored(
                    &mut page,
                    fnt,
                    &text,
                    layout.status_help.x,
                    layout.status_help.y,
                    MENU_TEXT_ENABLED,
                );
            }
        }
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
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    map_preview: Option<&RgbaImage>,
    map_names: &[&str],
    selected_map_index: Option<usize>,
    panel_anim_frame: usize,
) -> Option<RgbaImage> {
    let layout = choose_map_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
        &CHOOSE_MAP_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        None,
        MenuCaptionKind::ChooseMap,
        panel_anim_frame,
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
        let row = RectPx::new(
            layout.map_list.x,
            layout.map_list.y + (i as i32) * CHOOSE_MAP_LIST_ROW_H,
            layout.map_list.w,
            CHOOSE_MAP_LIST_ROW_H,
        );
        if Some(i) == selected_map_index {
            fill_rect(&mut page, row, [48, 28, 8, 255]);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                name,
                row.x + 4,
                row.y + 1,
                row.w - 8,
                row.h - 2,
                MENU_TEXT_ENABLED,
            );
        }
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "choose_map", Some(choose_map_title_csf_key()));
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
        let engagement = resolve_caption(csf, "select_engagement", choose_map_static_csf_key("select_engagement"));
        blit_text_colored(
            &mut page,
            fnt,
            &engagement,
            layout.label_engagement.x,
            layout.label_engagement.y,
            MENU_TEXT_ENABLED,
        );
        let game_type = resolve_caption(csf, "game_type", choose_map_static_csf_key("game_type"));
        blit_text_colored(
            &mut page,
            fnt,
            &game_type,
            layout.label_game_type.x,
            layout.label_game_type.y,
            MENU_TEXT_ENABLED,
        );
        let game_map = resolve_caption(csf, "game_map", choose_map_static_csf_key("game_map"));
        blit_text_colored(
            &mut page,
            fnt,
            &game_map,
            layout.label_game_map.x,
            layout.label_game_map.y,
            MENU_TEXT_ENABLED,
        );
        let battle = resolve_caption(csf, "battle", choose_map_static_csf_key("battle"));
        blit_text_colored(
            &mut page,
            fnt,
            &battle,
            type_row.x + 4,
            type_row.y + 1,
            MENU_TEXT_ENABLED,
        );
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
        assert_eq!(&page.as_raw()[di..di + 3], &[220, 40, 40]);
    }
}
