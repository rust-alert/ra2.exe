//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub struct BattleHudModel<'a> {
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

/// 合成战斗 HUD 叠加层：右栏 + 底栏。
///
/// 有 [`BattleHudChrome`] 时贴阵营侧栏素材；否则回退占位灰条（资源未挂载时的诊断态）。
/// 仍为像素合成；占位计划见 `RenderPlan::battle_hud_placeholders`，真资源计划待替换。
pub fn compose_battle_hud_overlay(
    viewport_w: u32,
    viewport_h: u32,
    fnt: Option<&FntFile>,
    paint: BattleHudModel<'_>,
    chrome: Option<&BattleHudChrome>,
) -> Option<RgbaImage> {
    let w = viewport_w.max(1);
    let h = viewport_h.max(1);
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;
    let layout = battle_hud_layout(w, h);

    let used_chrome = chrome.is_some_and(|c| c.has_sidebar_body());
    if let Some(chrome) = chrome.filter(|c| c.has_sidebar_body()) {
        blit_battle_hud_chrome(&mut page, chrome, layout);
    }
    else {
        fill_rect(&mut page, layout.sidebar, [28, 32, 40, 230]);
        stroke_rect(&mut page, layout.sidebar, [180, 40, 40, 255]);
        fill_rect(&mut page, layout.bottom_strip, [28, 32, 40, 230]);
        stroke_rect(&mut page, layout.bottom_strip, [180, 40, 40, 255]);
        let cell = 48;
        let gap = 4;
        let cols = ((layout.sidebar.w - 16) / (cell + gap)).max(1);
        let grid_top = layout.cameo_band.y.max(layout.sidebar.y + 160);
        for i in 0..8 {
            let col = i % cols;
            let row = i / cols;
            let cx = layout.sidebar.x + 8 + col * (cell + gap);
            let cy = grid_top + row * (cell + gap);
            if cy + cell > layout.side3.y {
                break;
            }
            let r = RectPx::new(cx, cy, cell, cell);
            fill_rect(&mut page, r, [20, 22, 28, 255]);
            stroke_rect(&mut page, r, [90, 30, 30, 255]);
        }
    }

    let funds_line = format!("$ {}", paint.funds);
    let power_mark = if paint.low_power { "!" } else { "" };
    let power_line = format!("电 {}/{}{power_mark}", paint.power_output, paint.power_drain);
    if let Some(fnt) = fnt {
        let credit_color = [0, 255, 255, 255];
        blit_caption_in_cell(
            &mut page,
            fnt,
            &funds_line,
            layout.credits.x,
            layout.credits.y,
            layout.credits.w,
            layout.credits.h,
            credit_color,
        );
        if !used_chrome {
            blit_text_colored(&mut page, fnt, &power_line, layout.sidebar.x + 8, layout.credits.y + layout.credits.h + 8, {
                if paint.low_power {
                    [255, 80, 80, 255]
                }
                else {
                    MENU_TEXT_ENABLED
                }
            });
            let mut y = layout.radar.y + 8;
            let line_h = 18;
            let text_x = layout.sidebar.x + 8;
            let text_w = layout.sidebar.w - 16;
            blit_caption_top_left_clipped(&mut page, fnt, &format!("选中 {}", paint.selected_summary), text_x, y, text_w, line_h, MENU_TEXT_ENABLED);
            y += line_h + 4;
            let queue = paint.produce_queue.unwrap_or("队列 —");
            blit_caption_top_left_clipped(&mut page, fnt, queue, text_x, y, text_w, line_h, MENU_TEXT_ENABLED);
            y += line_h + 4;
            if let Some(reject) = paint.reject {
                blit_caption_top_left_clipped(&mut page, fnt, reject, text_x, y, text_w, line_h, [255, 120, 80, 255]);
                y += line_h + 8;
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
            }
        }
        else if paint.paused || paint.outcome.is_some() || paint.reject.is_some() {
            let mut x = layout.bottom_strip.x + 72;
            let y = layout.bottom_strip.y + 8;
            if let Some(reject) = paint.reject {
                blit_text_colored(&mut page, fnt, reject, x, y, [255, 120, 80, 255]);
                x += 120;
            }
            if paint.paused {
                let reason = paint.pause_reason.unwrap_or("已暂停");
                blit_text_colored(&mut page, fnt, reason, x, y, MENU_TEXT_ACCENT);
                x += 80;
            }
            if let Some(outcome) = paint.outcome {
                blit_text_colored(&mut page, fnt, outcome, x, y, MENU_TEXT_ACCENT);
            }
        }
    }

    Some(page)
}

/// 合成对局暂停菜单叠加层：左战术区压暗，右栏六钮（选项 / 载入 / 保存 / 重开 / 放弃 / 回到游戏）。
///
/// `decoded` 若带 `sdbtnanm` 则贴壳层钮面；否则用纯色格占位。不改宿主导航。
pub fn compose_battle_pause_menu_overlay(
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    decoded: Option<&PageDecodeReport>,
) -> Option<RgbaImage> {
    let layout = battle_pause_menu_layout(viewport_w, viewport_h);
    let w = layout.canvas.w.max(1) as u32;
    let h = layout.canvas.h.max(1) as u32;
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;

    // 左战术区压暗罩（半透明黑）。
    fill_rect(&mut page, layout.dim, [0, 0, 0, 160]);
    // 右栏实心底，盖住对局 cameo。
    fill_rect(&mut page, layout.sidebar, [28, 16, 16, 240]);
    stroke_rect(&mut page, layout.sidebar, [140, 32, 32, 255]);

    for (i, entry_id) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().enumerate() {
        let cell = layout.buttons[i];
        let pressed = pressed_entry_id == Some(*entry_id);
        let hovered = hovered_entry_id == Some(*entry_id);
        let sprite = decoded.and_then(|d| {
            if pressed {
                find_button_pressed(d, entry_id).or_else(|| find_button_normal(d, entry_id))
            } else if hovered {
                find_button_hover(d, entry_id).or_else(|| find_button_normal(d, entry_id))
            } else {
                find_button_normal(d, entry_id)
            }
        });
        if let Some(sprite) = sprite {
            blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        } else {
            let fill = if pressed {
                [120, 24, 24, 255]
            } else if hovered {
                [90, 20, 20, 255]
            } else {
                [64, 12, 12, 255]
            };
            fill_rect(&mut page, cell, fill);
            stroke_rect(&mut page, cell, [200, 40, 40, 255]);
        }
        if let Some(fnt) = fnt {
            let caption = resolve_caption(csf, entry_id, battle_pause_menu_csf_label(entry_id));
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    Some(page)
}
