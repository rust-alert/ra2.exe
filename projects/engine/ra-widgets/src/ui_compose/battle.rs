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
        else if paint.paused || paint.outcome.is_some() || paint.reject.is_some() || paint.deploy_hint.is_some()
        {
            // 状态文案锚在右栏底脚内侧，不写到战术区。
            let mut x = bottom_strip.x + 8;
            let y = bottom_strip.y + 8;
            if let Some(hint) = paint.deploy_hint {
                blit_text_colored(&mut page, fnt, hint, x, y, MENU_TEXT_ACCENT);
                x += 100;
            }
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

    // 命令条悬停浮动提示（黑底白边，锚在钮上方）。
    if let (Some(tip), Some(slot), Some(_chrome), Some(fnt)) =
        (paint.command_tip, paint.command_hovered, chrome, fnt)
    {
        if !tip.is_empty() {
            if let Some(id) = COMMAND_BAR_BUTTON_IDS.get(slot) {
                let cell = rect_px_from_snapshot(&snap, id);
                if cell.w > 0 {
                    paint_command_tip(&mut page, fnt, tip, cell, w as i32, h as i32);
                }
            }
        }
    }

    Some(page)
}

fn paint_command_tip(
    page: &mut RgbaImage,
    fnt: &FntFile,
    tip: &str,
    anchor: RectPx,
    viewport_w: i32,
    viewport_h: i32,
) {
    let lines: Vec<&str> = tip.lines().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return;
    }
    let pad_x = 6;
    let pad_y = 3;
    let line_gap = 2;
    let line_h = fnt.bitmap_rows as i32;
    let mut text_w = 0i32;
    for line in &lines {
        text_w = text_w.max(fnt.text_width(line) as i32);
    }
    let box_w = text_w + pad_x * 2;
    let box_h = (lines.len() as i32) * line_h
        + (lines.len().saturating_sub(1) as i32) * line_gap
        + pad_y * 2;
    let mut bx = anchor.x + (anchor.w - box_w) / 2;
    let mut by = anchor.y - box_h - 4;
    if bx < 2 {
        bx = 2;
    }
    if bx + box_w > viewport_w - 2 {
        bx = (viewport_w - 2 - box_w).max(2);
    }
    if by < 2 {
        by = (anchor.y + anchor.h + 4).min(viewport_h - box_h - 2).max(2);
    }
    let rect = RectPx::new(bx, by, box_w, box_h);
    fill_rect(page, rect, [0, 0, 0, 255]);
    stroke_rect(page, rect, [220, 220, 220, 255]);
    let mut ty = by + pad_y;
    for line in lines {
        let tw = fnt.text_width(line) as i32;
        let tx = bx + (box_w - tw) / 2;
        blit_text_colored(page, fnt, line, tx, ty, [255, 255, 255, 255]);
        ty += line_h + line_gap;
    }
}

/// 合成对局暂停菜单叠加层：左战术区压暗，右栏六钮（选项 / 载入 / 保存 / 重开 / 放弃 / 回到游戏）。
///
/// `decoded` 若带壳层面板 / `sdbtnanm` 则走共享右栏 chrome；否则用 `RenderPlan` 纯色占位。不改宿主导航。
pub fn compose_battle_pause_menu_overlay(
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    decoded: Option<&PageDecodeReport>,
) -> Option<RgbaImage> {
    let _ = (viewport_w, viewport_h);
    let snap = solve_battle_pause();
    let canvas = RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H);
    let panel_top = rect_px_from_snapshot(&snap, "panel_top");
    let panel_tile = rect_px_from_snapshot(&snap, "panel_tile");
    let panel_bottom = rect_px_from_snapshot(&snap, "panel_bottom");
    let lower_strip = rect_px_from_snapshot(&snap, "lower_strip");
    let panel_tile_count = RightPanelChrome::shell_defaults().tile_count();
    let dim = RectPx::new(0, 0, panel_top.x, SHELL_BASE_H);
    let w = canvas.w.max(1) as u32;
    let h = canvas.h.max(1) as u32;
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;

    if let Some(decoded) = decoded {
        // 左战术区压暗罩（半透明黑）。
        fill_rect(&mut page, dim, [0, 0, 0, 160]);
        paint_right_panel_chrome(
            &mut page,
            decoded,
            panel_top,
            panel_tile,
            panel_tile_count,
            panel_bottom,
            lower_strip,
            0,
        );
    } else {
        // 诊断态：snapshot 占位色块（去掉全幅背景，保留左区压暗）。
        crate::RenderPlan::battle_pause_placeholders()
            .excluding_ids(&["background", "movie"])
            .paint_solids_into(&mut page);
        fill_rect(&mut page, dim, [0, 0, 0, 160]);
    }

    let button_ids = &BATTLE_PAUSE_MENU_BUTTON_IDS[..];
    let btn_plan = crate::RenderPlan::battle_pause_placeholders().button_sprite_plan(button_ids);
    if let Some(decoded) = decoded {
        btn_plan.paint_sprites_into(&mut page, |slot| {
            let pressed = pressed_entry_id == Some(slot);
            let hovered = hovered_entry_id == Some(slot);
            resolve_button_sprite(decoded, slot, pressed, hovered).map(|s| &s.image)
        });
    }

    for entry_id in BATTLE_PAUSE_MENU_BUTTON_IDS.iter() {
        let Some(cell) = btn_plan.rect_px_of(entry_id) else {
            continue;
        };
        let pressed = pressed_entry_id == Some(*entry_id);
        let hovered = hovered_entry_id == Some(*entry_id);
        let has_sprite = decoded
            .and_then(|d| resolve_button_sprite(d, entry_id, pressed, hovered))
            .is_some();
        if !has_sprite {
            if decoded.is_some() {
                // 有 decode 但缺该钮精灵：本地色块兜底。
                let fill = if pressed {
                    [120, 24, 24, 255]
                } else if hovered {
                    [90, 20, 20, 255]
                } else {
                    [64, 12, 12, 255]
                };
                fill_rect(&mut page, cell, fill);
                stroke_rect(&mut page, cell, [200, 40, 40, 255]);
            } else if pressed || hovered {
                // 无 decode：plan solids 已画灰钮；悬停/按下再叠强调色。
                let fill = if pressed {
                    [120, 24, 24, 255]
                } else {
                    [90, 20, 20, 255]
                };
                fill_rect(&mut page, cell, fill);
                stroke_rect(&mut page, cell, [200, 40, 40, 255]);
            }
        }
        if let Some(fnt) = fnt {
            let caption = resolve_caption(csf, entry_id, battle_pause_menu_csf_label(entry_id));
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    Some(page)
}
