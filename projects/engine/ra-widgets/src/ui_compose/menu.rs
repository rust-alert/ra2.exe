//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub(super) fn compose_shell_menu_page(
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

    paint_right_panel_chrome(
        &mut page,
        decoded,
        layout.panel_top,
        layout.panel_tile,
        layout.panel_tile_count,
        layout.panel_bottom,
        layout.lower_strip,
        warn_anim_frame,
    );
    let btn_n = button_ids.len();
    let tile_occupied = |tile_y: i32| {
        (0..btn_n).any(|i| {
            let b = layout.buttons[i];
            b.w > 0 && b.h > 0 && b.y == tile_y
        })
    };
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
                let cell_x = layout
                    .buttons
                    .iter()
                    .find(|b| b.w > 0 && b.h > 0)
                    .map(|b| b.x)
                    .unwrap_or(layout.panel_tile.x + (RIGHT_PANEL_W - BUTTON_CELL_W));
                blit_rgba(&mut page, &sprite.image, cell_x, tile_y);
            }
        }
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
