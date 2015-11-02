//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub(super) fn compose_shell_menu_page(
    decoded: &PageDecodeReport,
    snap: &LayoutSnapshot,
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
    let canvas = RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H);
    let background = rect_px_from_snapshot(snap, "background");
    let movie_rect = rect_px_from_snapshot(snap, "movie");
    let panel_top = rect_px_from_snapshot(snap, "panel_top");
    let panel_tile = rect_px_from_snapshot(snap, "panel_tile");
    let panel_tile_count = RightPanelChrome::shell_defaults().tile_count();
    let panel_bottom = rect_px_from_snapshot(snap, "panel_bottom");
    let lower_strip = rect_px_from_snapshot(snap, "lower_strip");
    let title = rect_px_from_snapshot(snap, "title");
    let tooltip = rect_px_from_snapshot(snap, "tooltip");

    let bg = decoded.background.as_ref()?;
    let mut page = RgbaImage::from_raw(
        canvas.w as u32,
        canvas.h as u32,
        vec![0u8; (canvas.w as usize) * (canvas.h as usize) * 4],
    )?;

    blit_rgba(&mut page, &bg.image, background.x, background.y);
    if let Some(frame) = movie {
        blit_stretched(&mut page, frame, movie_rect);
    }

    paint_right_panel_chrome(
        &mut page,
        decoded,
        panel_top,
        panel_tile,
        panel_tile_count,
        panel_bottom,
        lower_strip,
        warn_anim_frame,
    );

    let btn_plan = shell_button_sprite_plan(captions, button_ids);
    let tile_occupied = |tile_y: i32| {
        button_ids.iter().any(|id| {
            btn_plan
                .rect_px_of(id)
                .is_some_and(|b| b.w > 0 && b.h > 0 && b.y == tile_y)
        })
    };
    // 波浪出去：无字平铺格叠 `SDBTNANM`；进来不叠，避免满钮收束后瞬间消失。
    if let Some(wave) = wave {
        if wave.animate_empty_tiles {
            for ti in 0..panel_tile_count {
                let tile_y = panel_tile.y + ti * panel_tile.h;
                if tile_occupied(tile_y) {
                    continue;
                }
                let Some(&frame) = wave.tiles.get(ti as usize) else {
                    continue;
                };
                let Some(sprite) = decoded.sdbtnanm_frame(frame) else {
                    continue;
                };
                let cell_x = button_ids
                    .iter()
                    .find_map(|id| btn_plan.rect_px_of(id))
                    .map(|b| b.x)
                    .unwrap_or(panel_tile.x + (RIGHT_PANEL_W - BUTTON_CELL_W));
                blit_rgba(&mut page, &sprite.image, cell_x, tile_y);
            }
        }
    }

    for (i, entry_id) in button_ids.iter().enumerate() {
        let Some(cell) = btn_plan.rect_px_of(entry_id) else {
            continue;
        };
        let normal = find_button_normal(decoded, entry_id)?;
        // 禁用态跟入口 id：主菜单占位项 + 各页「载入」未实现；单人「新战役」已可进。
        let disabled = matches!(
            *entry_id,
            "ww_online" | "network" | "movies" | "load" | "create_random"
        );
        let wave_frame = wave.and_then(|w| w.buttons.get(i).copied());
        let sprite = if let Some(frame) = wave_frame {
            decoded.sdbtnanm_frame(frame).unwrap_or(normal)
        } else if disabled {
            normal
        } else {
            resolve_button_sprite(
                decoded,
                entry_id,
                pressed_entry_id == Some(entry_id),
                hovered_entry_id == Some(entry_id),
            )
            .unwrap_or(normal)
        };
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        // 切页流程：字先消 → 钮进出 → 停稳后再出字。`wave` 有值时只画钮面。
        if wave_frame.is_some() {
            continue;
        }
        // 壳层禁用：同常态 `SDBTNANM` 帧 + 暗红字，不压暗钮面（原版无整格压暗投影）。
        if let Some(fnt) = fnt {
            let key = captions.label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let color = if disabled {
                MENU_TEXT_DISABLED
            } else {
                MENU_TEXT_ENABLED
            };
            let pressed = pressed_entry_id == Some(entry_id) && !disabled;
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, color);
        }
    }

    if let Some(fnt) = fnt {
        let title_text = match captions {
            MenuCaptionKind::Main => Some(resolve_caption(csf, "main_menu", Some("GUI:MainMenu"))),
            MenuCaptionKind::SinglePlayer => {
                Some(resolve_caption(csf, "single_player", Some(single_player_title_csf_key())))
            }
            // 战役 / 遭遇战 / 选图标题由各自 compose 按对话框锚点另画。
            MenuCaptionKind::Campaign | MenuCaptionKind::SkirmishLobby | MenuCaptionKind::ChooseMap => {
                None
            }
        };
        if let Some(title_text) = title_text {
            blit_caption_in_cell(
                &mut page,
                fnt,
                &title_text,
                title.x,
                title.y,
                title.w,
                title.h,
                MENU_TEXT_ENABLED,
            );
        }
        // 主菜单 / 单人页底栏：由壳层传入打字机可见切片。
        if matches!(
            captions,
            MenuCaptionKind::Main | MenuCaptionKind::SinglePlayer
        ) {
            if let Some(text) = status_text.filter(|s| !s.is_empty()) {
                blit_text_colored(
                    &mut page,
                    fnt,
                    text,
                    tooltip.x,
                    tooltip.y,
                    MENU_TEXT_ENABLED,
                );
            }
        }
    }

    Some(page)
}

/// 当前壳层页右栏按钮的精灵计划（几何来自对应 `solve_*` snapshot）。
fn shell_button_sprite_plan(captions: MenuCaptionKind, button_ids: &[&str]) -> crate::RenderPlan {
    let base = match captions {
        MenuCaptionKind::Main => crate::RenderPlan::shell_page_placeholders(
            "main_menu",
            &MAIN_MENU_BUTTON_IDS[..5],
            Some(MAIN_MENU_BUTTON_IDS[5]),
        ),
        MenuCaptionKind::SinglePlayer => crate::RenderPlan::shell_page_placeholders(
            "single_player",
            &SINGLE_PLAYER_BUTTON_IDS[..3],
            Some(SINGLE_PLAYER_BUTTON_IDS[3]),
        ),
        MenuCaptionKind::Campaign => crate::RenderPlan::campaign_placeholders(),
        MenuCaptionKind::SkirmishLobby => crate::RenderPlan::skirmish_lobby_placeholders(),
        MenuCaptionKind::ChooseMap => crate::RenderPlan::choose_map_placeholders(),
    };
    base.button_sprite_plan(button_ids)
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
    let _ = (viewport_w, viewport_h);
    let snap = ra_layout::solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    compose_shell_menu_page(
        decoded,
        &snap,
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
    let _ = (viewport_w, viewport_h);
    let snap = ra_layout::solve_shell_page(
        "single_player",
        &SINGLE_PLAYER_BUTTON_IDS[..3],
        Some(SINGLE_PLAYER_BUTTON_IDS[3]),
    );
    compose_shell_menu_page(
        decoded,
        &snap,
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
