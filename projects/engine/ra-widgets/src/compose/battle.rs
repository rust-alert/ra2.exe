//! 对局 HUD 叠层合成。

use super::*;
use crate::battle_pause_menu::{
    button_rects, dim_rect, logo_dest_rect, resolve_sidebttn, BattlePauseChrome,
};

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
    /// 选中摘要（如 `#3 AMCV` 或 `#3+2`）。
    pub selected_summary: &'a str,
    /// 可部署提示（如 `X→GACNST`），无可部署时为空。
    pub deploy_hint: Option<&'a str>,
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
    /// 命令条按下槽（高亮帧）。
    pub command_pressed: Option<usize>,
    /// 命令条悬停槽（浮动 `TIP:*`）。
    pub command_hovered: Option<usize>,
    /// 悬停提示文案（已解析 CSF；可含换行）。
    pub command_tip: Option<&'a str>,
}

/// 合成战斗 HUD 叠加层：右栏 + 底边命令条。
///
/// 有 [`BattleHudChrome`] 时贴阵营侧栏与命令条素材；否则回退 `RenderPlan` 占位（资源未挂载时的诊断态）。
/// 仍为像素合成；真资源计划待替换。
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
    let metrics = match chrome {
        Some(c) => BattleHudChromeMetrics::for_mix(&c.mix),
        None => BattleHudChromeMetrics::allied(),
    };
    let snap = solve_battle_hud_with_metrics(w, h, metrics);
    let credits = rect_px_from_snapshot(&snap, "credits");
    let sidebar = rect_px_from_snapshot(&snap, "sidebar");
    let radar = rect_px_from_snapshot(&snap, "radar");
    let bottom_strip = rect_px_from_snapshot(&snap, "bottom_strip");

    let used_chrome = chrome.is_some_and(|c| c.has_sidebar_body());
    if let Some(chrome) = chrome.filter(|c| c.has_sidebar_body()) {
        crate::battle_hud::blit_battle_hud_chrome_with_state(
            &mut page,
            chrome,
            &snap,
            metrics.power_w,
            paint.command_pressed,
        );
    } else {
        // 诊断态：snapshot 占位（跳过战术区底边命令条，保持左下透明）。
        crate::RenderPlan::battle_hud_placeholders(w, h)
            .excluding_ids(&[
                "command_bar",
                "lendcap",
                "rendcap",
                "cmd0",
                "cmd1",
                "cmd2",
                "cmd3",
                "cmd4",
                "cmd5",
            ])
            .paint_solids_into(&mut page);
    }

    let funds_line = paint.funds.to_string();
    let power_mark = if paint.low_power { "!" } else { "" };
    let power_line = format!("电 {}/{}{power_mark}", paint.power_output, paint.power_drain);
    if let Some(fnt) = fnt {
        // 零售资金条为亮青（非青绿青绿），且不带 `$ ` 前缀。
        let credit_color = [0, 220, 255, 255];
        blit_caption_in_cell(
            &mut page,
            fnt,
            &funds_line,
            credits.x,
            credits.y,
            credits.w,
            credits.h,
            credit_color,
        );
        if !used_chrome {
            blit_text_colored(&mut page, fnt, &power_line, sidebar.x + 8, credits.y + credits.h + 8, {
                if paint.low_power {
                    [255, 80, 80, 255]
                }
                else {
                    MENU_TEXT_ENABLED
                }
            });
            let mut y = radar.y + 8;
            let line_h = 18;
            let text_x = sidebar.x + 8;
            let text_w = sidebar.w - 16;
            blit_caption_top_left_clipped(&mut page, fnt, &format!("选中 {}", paint.selected_summary), text_x, y, text_w, line_h, MENU_TEXT_ENABLED);
            y += line_h + 4;
            if let Some(hint) = paint.deploy_hint {
                blit_caption_top_left_clipped(&mut page, fnt, hint, text_x, y, text_w, line_h, MENU_TEXT_ACCENT);
                y += line_h + 4;
            }
            if let Some(queue) = paint.produce_queue {
                blit_caption_top_left_clipped(&mut page, fnt, queue, text_x, y, text_w, line_h, MENU_TEXT_ENABLED);
                y += line_h + 4;
            }
            if let Some(reject) = paint.reject {
                blit_caption_top_left_clipped(&mut page, fnt, reject, text_x, y, text_w, line_h, [255, 120, 80, 255]);
                y += line_h + 4;
            }
            blit_caption_top_left_clipped(&mut page, fnt, &format!("t{}", paint.tick), text_x, y, text_w, line_h, MENU_TEXT_SECTION);
            y += line_h + 4;
            if paint.paused {
                let reason = paint.pause_reason.unwrap_or("已暂停");
                blit_caption_top_left_clipped(&mut page, fnt, reason, text_x, y, text_w, line_h, MENU_TEXT_ACCENT);
                y += line_h + 4;
            }
            if let Some(outcome) = paint.outcome {
                blit_caption_top_left_clipped(&mut page, fnt, outcome, text_x, y, text_w, line_h, MENU_TEXT_ACCENT);
            }
            let _ = (y, bottom_strip);
        } else {
            // 有 chrome 时只在底脚条带写少量诊断（避免盖住 cameo）。
            let x = sidebar.x + 8;
            let mut y = bottom_strip.y + 4;
            if let Some(hint) = paint.deploy_hint {
                blit_text_colored(&mut page, fnt, hint, x, y, MENU_TEXT_ACCENT);
                y += 14;
            }
            if let Some(reject) = paint.reject {
                blit_text_colored(&mut page, fnt, reject, x, y, [255, 120, 80, 255]);
                y += 14;
            }
            if paint.paused {
                let reason = paint.pause_reason.unwrap_or("已暂停");
                blit_text_colored(&mut page, fnt, reason, x, y, MENU_TEXT_ACCENT);
                y += 14;
            }
            if let Some(outcome) = paint.outcome {
                blit_text_colored(&mut page, fnt, outcome, x, y, MENU_TEXT_ACCENT);
            }
        }
    }

    if let (Some(tip), Some(slot), Some(_chrome), Some(fnt)) =
        (paint.command_tip, paint.command_hovered, chrome, fnt)
    {
        let snap = solve_battle_hud_with_metrics(w, h, metrics);
        let cell = rect_px_from_snapshot(&snap, &format!("cmd{slot}"));
        if cell.w > 0 && cell.h > 0 {
            paint_command_tip(&mut page, fnt, tip, cell, w as i32, h as i32);
        }
    }

    Some(page)
}

fn paint_command_tip(
    page: &mut RgbaImage,
    fnt: &FntFile,
    tip: &str,
    cell: RectPx,
    page_w: i32,
    page_h: i32,
) {
    let lines: Vec<&str> = tip.lines().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return;
    }
    let pad_x = 6;
    let pad_y = 4;
    let line_gap = 2;
    let line_h = fnt.bitmap_rows as i32;
    let mut text_w = 0i32;
    for line in &lines {
        text_w = text_w.max(fnt.text_width(line) as i32);
    }
    let box_w = text_w + pad_x * 2;
    let box_h = (line_h + line_gap) * lines.len() as i32 - line_gap + pad_y * 2;
    let mut bx = cell.x + cell.w / 2 - box_w / 2;
    let mut by = cell.y - box_h - 6;
    if bx < 0 {
        bx = 0;
    }
    if by < 0 {
        by = cell.y + cell.h + 4;
    }
    if bx + box_w > page_w {
        bx = (page_w - box_w).max(0);
    }
    if by + box_h > page_h {
        by = (page_h - box_h).max(0);
    }
    fill_rect(page, RectPx::new(bx, by, box_w, box_h), [0, 0, 0, 200]);
    stroke_rect(page, RectPx::new(bx, by, box_w, box_h), [200, 200, 80, 255]);
    let mut ty = by + pad_y;
    for line in lines {
        let tw = fnt.text_width(line) as i32;
        let tx = bx + (box_w - tw) / 2;
        blit_text_colored(page, fnt, line, tx, ty, [255, 255, 255, 255]);
        ty += line_h + line_gap;
    }
}

/// 合成对局暂停菜单叠加层（窗口像素，叠在已画好的对局 HUD 之上）。
///
/// - 战术区半透明压暗
/// - 居中贴本地阵营 `radar.shp` 首帧（盟军鹰 / 苏军镰锤）
/// - cameo 带上贴 `sidebttn` 六钮 + CSF 文案
///
/// **禁止**再画主菜单 `sdtp` / `sdbtnanm`。
pub fn compose_battle_pause_menu_overlay(
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    pause: Option<&BattlePauseChrome>,
    hud_metrics: BattleHudChromeMetrics,
) -> Option<RgbaImage> {
    let w = viewport_w.max(1);
    let h = viewport_h.max(1);
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;

    let dim = dim_rect(w, h, hud_metrics);
    fill_rect(&mut page, dim, [0, 0, 0, 160]);

    if let Some(pause) = pause {
        if let Some(logo) = pause.logo.as_ref() {
            let dest = logo_dest_rect(w, h, hud_metrics, logo.image.width(), logo.image.height());
            blit_stretched(&mut page, &logo.image, dest);
        }

        let snap = solve_battle_hud_with_metrics(w, h, hud_metrics);
        let cameo = rect_px_from_snapshot(&snap, "cameo_band");
        // 盖住 cameo 生产线，避免暂停钮与建造图标叠在一起。
        fill_rect(&mut page, cameo, [0, 0, 0, 220]);

        let rects = button_rects(w, h, hud_metrics);
        for (entry_id, cell) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().zip(rects.iter()) {
            let pressed = pressed_entry_id == Some(*entry_id);
            let hovered = hovered_entry_id == Some(*entry_id);
            if let Some(sprite) = resolve_sidebttn(pause, pressed, hovered) {
                blit_stretched(&mut page, &sprite.image, *cell);
            } else {
                let fill = if pressed {
                    [40, 40, 80, 255]
                } else if hovered {
                    [30, 30, 60, 255]
                } else {
                    [20, 20, 40, 255]
                };
                fill_rect(&mut page, *cell, fill);
                stroke_rect(&mut page, *cell, [180, 180, 200, 255]);
            }
            if let Some(fnt) = fnt {
                let caption = resolve_caption(csf, entry_id, battle_pause_menu_csf_label(entry_id));
                let (tx, ty, tw, th) = owner_draw_caption_rect(*cell, pressed);
                blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
            }
        }
    } else {
        // 无阵营素材时仍画压暗 + 占位钮，避免只剩黑屏。
        let rects = button_rects(w, h, hud_metrics);
        for (entry_id, cell) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().zip(rects.iter()) {
            let pressed = pressed_entry_id == Some(*entry_id);
            let hovered = hovered_entry_id == Some(*entry_id);
            let fill = if pressed {
                [40, 40, 80, 255]
            } else if hovered {
                [30, 30, 60, 255]
            } else {
                [20, 20, 40, 255]
            };
            fill_rect(&mut page, *cell, fill);
            stroke_rect(&mut page, *cell, [180, 180, 200, 255]);
            if let Some(fnt) = fnt {
                let caption = resolve_caption(csf, entry_id, battle_pause_menu_csf_label(entry_id));
                let (tx, ty, tw, th) = owner_draw_caption_rect(*cell, pressed);
                blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
            }
        }
    }

    Some(page)
}
