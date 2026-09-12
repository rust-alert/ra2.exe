//! 对局 HUD 叠层合成。

use super::*;
use crate::{
    battle_pause_menu::{BattlePauseChrome, button_rects, dim_rect, resolve_sidebttn},
    skin::text::battle_pause_menu_fallback_label,
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
    /// 命令条按下槽（高亮帧）。
    pub command_pressed: Option<usize>,
    /// 命令条悬停槽（浮动 `TIP:*`）。
    pub command_hovered: Option<usize>,
    /// 悬停提示文案（已解析 CSF；可含换行）。
    pub command_tip: Option<&'a str>,
    /// 修理工具是否激活（侧栏拱钮按下帧）。
    pub repair_active: bool,
    /// 出售工具是否激活（侧栏拱钮按下帧）。
    pub sell_active: bool,
    /// 本机雷达开图（有存活雷达且未低电）。
    pub radar_online: bool,
    /// 当前分类页签。
    pub sidebar_tab: usize,
    /// 四分类页签是否可见（有对应可建造基础才显示）。
    pub sidebar_tabs_visible: [bool; 4],
    /// 当前页 cameo 列表。
    pub cameos: &'a [crate::battle_hud::BattleCameoPaint<'a>],
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
        None => BattleHudChromeMetrics::sidec01(),
    };
    let snap = solve_battle_hud_with_metrics(w, h, metrics);
    let credits = rect_px_from_snapshot(&snap, "credits");
    let sidebar = rect_px_from_snapshot(&snap, "sidebar");
    let radar = rect_px_from_snapshot(&snap, "radar");
    let bottom_strip = rect_px_from_snapshot(&snap, "bottom_strip");

    let used_chrome = chrome.is_some_and(|c| c.has_sidebar_body());
    if let Some(chrome) = chrome.filter(|c| c.has_sidebar_body()) {
        crate::battle_hud::blit_battle_hud_chrome_ex(
            &mut page,
            chrome,
            &snap,
            metrics.power_w,
            paint.command_pressed,
            paint.paused,
            paint.repair_active,
            paint.sell_active,
            paint.radar_online,
            paint.tick,
            paint.sidebar_tabs_visible,
            paint.sidebar_tab,
        );
        if !paint.paused {
            crate::battle_hud::blit_battle_cameos(&mut page, &snap, metrics.power_w, paint.cameos, paint.tick);
        }
    }
    else {
        // 诊断态：snapshot 占位（跳过战术区底边命令条，保持左下透明）。
        crate::RenderPlan::battle_hud_placeholders(w, h)
            .excluding_ids(&["command_bar", "lendcap", "rendcap", "cmd0", "cmd1", "cmd2", "cmd3", "cmd4", "cmd5"])
            .paint_solids_into(&mut page);
    }

    let funds_line = paint.funds.to_string();
    let power_mark = if paint.low_power { "!" } else { "" };
    let power_line = format!("电 {}/{}{power_mark}", paint.power_output, paint.power_drain);
    if let Some(fnt) = fnt {
        // 零售资金条为亮青（非青绿青绿），且不带 `$ ` 前缀。
        let credit_color = [0, 220, 255, 255];
        blit_caption_in_cell(&mut page, fnt, &funds_line, credits.x, credits.y, credits.w, credits.h, credit_color);
        if !used_chrome {
            blit_text_colored(&mut page, fnt, &power_line, sidebar.x + 8, credits.y + credits.h + 8, {
                if paint.low_power { [255, 80, 80, 255] } else { MENU_TEXT_ENABLED }
            });
            let mut y = radar.y + 8;
            let line_h = 18;
            let text_x = sidebar.x + 8;
            let text_w = sidebar.w - 16;
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                &format!("选中 {}", paint.selected_summary),
                text_x,
                y,
                text_w,
                line_h,
                MENU_TEXT_ENABLED,
            );
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
            let _ = (y, bottom_strip);
        }
        else if !paint.paused {
            // 有 chrome 且非暂停：只在底脚条带写少量诊断（避免盖住 cameo / 暂停钮）。
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
            let _ = y;
        }
        else {
            // 暂停菜单打开：资金条仍画，底脚/侧栏诊断文案一律不写，留给暂停钮与 chrome。
        }
    }

    if !paint.paused {
        if let (Some(tip), Some(slot), Some(_chrome), Some(fnt)) = (paint.command_tip, paint.command_hovered, chrome, fnt) {
            let snap = solve_battle_hud_with_metrics(w, h, metrics);
            let cell = rect_px_from_snapshot(&snap, &format!("cmd{slot}"));
            if cell.w > 0 && cell.h > 0 {
                paint_command_tip(&mut page, fnt, tip, cell, w as i32, h as i32);
            }
        }
    }

    Some(page)
}

fn paint_command_tip(page: &mut RgbaImage, fnt: &FntFile, tip: &str, cell: RectPx, page_w: i32, page_h: i32) {
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

/// 合成对局暂停菜单叠加层（窗口像素；暂停时单独作为 UI 层，不叠 HUD）。
///
/// 几何只认 [`ra_layout::solve_battle_pause_at`]：全屏 `dim` + 右缘 `SIDEBTTN` 四钮。
/// 钮面必须走 `sidebttn.shp` / `sidebar.pal`（owner-draw type 2），禁止自制黄框卡片。
pub fn compose_battle_pause_menu_overlay(
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    pause: Option<&BattlePauseChrome>,
) -> Option<RgbaImage> {
    let w = viewport_w.max(1);
    let h = viewport_h.max(1);
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;

    let dim = dim_rect(w, h);
    fill_rect(&mut page, dim, [0, 0, 0, 160]);

    let rects = button_rects(w, h);
    for (entry_id, cell) in BATTLE_PAUSE_MENU_BUTTON_IDS.iter().zip(rects.iter()) {
        let pressed = pressed_entry_id == Some(*entry_id);
        let hovered = hovered_entry_id == Some(*entry_id);
        let sprite = pause.and_then(|p| resolve_sidebttn(p, pressed, hovered));
        if let Some(sprite) = sprite {
            blit_stretched(&mut page, &sprite.image, *cell);
        } else {
            // 缺 SIDEBTTN 时只留深色占位，不画黄框伪 UI。
            fill_rect(&mut page, *cell, [24, 28, 40, 255]);
        }
        if let Some(fnt) = fnt {
            let caption = {
                let from_csf = resolve_caption(csf, entry_id, battle_pause_menu_csf_label(entry_id));
                if from_csf == entry_id.replace('_', " ") {
                    battle_pause_menu_fallback_label(entry_id).to_string()
                } else {
                    from_csf
                }
            };
            let (tx, ty, tw, th) = owner_draw_caption_rect(*cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    Some(page)
}

/// 合成放弃确认叠层（dim + 右缘 `SIDEBTTN` Leave / Cancel）。
pub fn compose_battle_abort_confirm_overlay(
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    pause: Option<&BattlePauseChrome>,
) -> Option<RgbaImage> {
    use crate::{
        battle_abort_confirm::{button_rects as abort_button_rects, dim_rect as abort_dim, prompt_rect},
        skin::text::{battle_abort_confirm_csf_label, battle_abort_confirm_fallback_label, battle_abort_confirm_prompt_csf_key},
    };
    use ra_layout::BATTLE_ABORT_CONFIRM_BUTTON_IDS;

    let w = viewport_w.max(1);
    let h = viewport_h.max(1);
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;

    fill_rect(&mut page, abort_dim(w, h), [0, 0, 0, 160]);

    if let Some(fnt) = fnt {
        let prompt = prompt_rect(w, h);
        let text = {
            let from = resolve_caption(csf, "prompt", Some(battle_abort_confirm_prompt_csf_key()));
            if from == "prompt" {
                "要放弃当前任务吗？".to_string()
            } else {
                from
            }
        };
        blit_caption_wrapped(&mut page, fnt, &text, prompt.x, prompt.y, prompt.w, prompt.h, MENU_TEXT_ENABLED);
    }

    let rects = abort_button_rects(w, h);
    for (entry_id, cell) in BATTLE_ABORT_CONFIRM_BUTTON_IDS.iter().zip(rects.iter()) {
        let pressed = pressed_entry_id == Some(*entry_id);
        let hovered = hovered_entry_id == Some(*entry_id);
        let sprite = pause.and_then(|p| resolve_sidebttn(p, pressed, hovered));
        if let Some(sprite) = sprite {
            blit_stretched(&mut page, &sprite.image, *cell);
        } else {
            fill_rect(&mut page, *cell, [24, 28, 40, 255]);
        }
        if let Some(fnt) = fnt {
            let caption = {
                let from_csf = resolve_caption(csf, entry_id, battle_abort_confirm_csf_label(entry_id));
                if from_csf == entry_id.replace('_', " ") {
                    battle_abort_confirm_fallback_label(entry_id).to_string()
                } else {
                    from_csf
                }
            };
            let (tx, ty, tw, th) = owner_draw_caption_rect(*cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    Some(page)
}

/// 合成局内选项 `0xBBB` 叠层。
pub fn compose_battle_in_game_options_overlay(
    viewport_w: u32,
    viewport_h: u32,
    state: &crate::battle_in_game_options::BattleInGameOptionsState,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    pause: Option<&BattlePauseChrome>,
    stub_notice: Option<&str>,
) -> Option<RgbaImage> {
    use crate::{
        battle_in_game_options::{button_rects as opts_button_rects, options_snapshot},
        skin::text::{
            battle_in_game_options_csf_label, battle_in_game_options_fallback_label, battle_in_game_speed_label_key,
        },
    };
    use ra_layout::{BATTLE_IN_GAME_OPTIONS_BUTTON_IDS, rect_px_from_snapshot};

    let w = viewport_w.max(1);
    let h = viewport_h.max(1);
    let mut page = RgbaImage::from_raw(w, h, vec![0u8; (w as usize) * (h as usize) * 4])?;
    let snap = options_snapshot(w, h);

    fill_rect(&mut page, rect_px_from_snapshot(&snap, "dim"), [0, 0, 0, 160]);

    let label = |id: &str, fallback: &str| -> String {
        let from = resolve_caption(csf, id, battle_in_game_options_csf_label(id));
        if from == id.replace('_', " ") || from == *id {
            battle_in_game_options_fallback_label(id).to_string()
        } else if from.is_empty() {
            fallback.to_string()
        } else {
            from
        }
    };

    if let Some(fnt) = fnt {
        let title = rect_px_from_snapshot(&snap, "title");
        blit_caption_in_cell(
            &mut page,
            fnt,
            &label("title", "Game Options"),
            title.x,
            title.y,
            title.w,
            title.h,
            MENU_TEXT_SECTION,
        );
        for (id, fallback) in [("caption_game_speed", "Game Speed"), ("caption_scroll_rate", "Scroll Rate")] {
            let cell = rect_px_from_snapshot(&snap, id);
            blit_caption_top_right_clipped(&mut page, fnt, &label(id, fallback), cell.x, cell.y, cell.w, cell.h, MENU_TEXT_ACCENT);
        }
        for (id, pos) in [("value_game_speed", state.game_speed), ("value_scroll_rate", state.scroll_rate)] {
            let cell = rect_px_from_snapshot(&snap, id);
            let key = battle_in_game_speed_label_key(pos);
            let text = resolve_csf_text(csf, key).unwrap_or_else(|| format!("{pos}"));
            blit_caption_top_left_clipped(&mut page, fnt, &text, cell.x, cell.y, cell.w, cell.h, MENU_TEXT_ENABLED);
        }
        for (id, checked, fallback) in [
            ("check_target_lines", state.target_lines, "Target Lines"),
            ("check_show_hidden", state.show_hidden, "Show Hidden"),
            ("check_tooltips", state.tooltips, "Tooltips"),
        ] {
            let cell = rect_px_from_snapshot(&snap, id);
            draw_checkbox(&mut page, cell, checked);
            blit_text_colored(&mut page, fnt, &label(id, fallback), cell.x + 22, cell.y, MENU_TEXT_ACCENT);
        }
    }

    draw_trackbar(&mut page, rect_px_from_snapshot(&snap, "track_game_speed"), state.game_speed, 6);
    draw_trackbar(&mut page, rect_px_from_snapshot(&snap, "track_scroll_rate"), state.scroll_rate, 6);

    let rects = opts_button_rects(w, h);
    for (entry_id, cell) in BATTLE_IN_GAME_OPTIONS_BUTTON_IDS.iter().zip(rects.iter()) {
        let pressed = pressed_entry_id == Some(*entry_id);
        let hovered = hovered_entry_id == Some(*entry_id);
        let sprite = pause.and_then(|p| resolve_sidebttn(p, pressed, hovered));
        if let Some(sprite) = sprite {
            blit_stretched(&mut page, &sprite.image, *cell);
        } else {
            fill_rect(&mut page, *cell, [16, 24, 48, 255]);
            stroke_rect(&mut page, *cell, [80, 120, 180, 255]);
        }
        if let Some(fnt) = fnt {
            let caption = label(entry_id, battle_in_game_options_fallback_label(entry_id));
            let (tx, ty, tw, th) = owner_draw_caption_rect(*cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    if let (Some(fnt), Some(notice)) = (fnt, stub_notice.filter(|s| !s.is_empty())) {
        let footer = rect_px_from_snapshot(&snap, "footer");
        blit_caption_top_left_clipped(&mut page, fnt, notice, footer.x, footer.y, footer.w, footer.h, MENU_TEXT_SECTION);
    }

    Some(page)
}
