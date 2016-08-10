//! 遭遇战积分页合成。

use super::*;

/// 积分表一行。
#[derive(Debug, Clone)]
pub struct SkirmishScoreRow {
    /// 玩家显示名。
    pub name: String,
    /// 行颜色（RGBA）。
    pub color: [u8; 4],
    /// 摧毁数。
    pub kills: u32,
    /// 损失数。
    pub losses: u32,
    /// 建造数。
    pub built: u32,
    /// 分数。
    pub score: i32,
}

/// 遭遇战积分页绘制参数。
pub struct SkirmishScorePaint<'a> {
    /// 第几局（占位，默认 1）。
    pub game_index: u32,
    /// 时长文案（如 `00:00:15`）。
    pub time_text: &'a str,
    /// 各方行。
    pub rows: &'a [SkirmishScoreRow],
    /// 可选左区背景（缺则深色底）。
    pub backdrop: Option<&'a RgbaImage>,
    /// 战役结算用 `GUI:STANDALONESCORE`，否则 `GUI:SKIRMISHSCORE`。
    pub campaign: bool,
    /// 是否叠原版半透明黑底统计区。
    ///
    /// - `true`：原版黑底 + 金边（盟军 / 苏军战报）。
    /// - `false`：尤里皮——不叠黑底，改画金属描边框（见合成）。
    pub stats_shade: bool,
}

/// 合成遭遇战积分页：壳层右栏 + 左区统计表 + 「继续」。
pub fn compose_skirmish_score_page(
    decoded: &PageDecodeReport,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    paint: SkirmishScorePaint<'_>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let snap = solve_skirmish_score();
    let mut page = compose_shell_menu_page(
        decoded,
        &snap,
        &SKIRMISH_SCORE_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        None,
        fnt,
        csf,
        None,
        MenuCaptionKind::SkirmishScore,
        None,
        warn_anim_frame,
    )?;

    let stats = rect_px_from_snapshot(&snap, "stats_panel");
    let game_label = rect_px_from_snapshot(&snap, "game_label");
    let time_label = rect_px_from_snapshot(&snap, "time_label");
    let table = rect_px_from_snapshot(&snap, "table");
    let title = rect_px_from_snapshot(&snap, "title");
    let background = rect_px_from_snapshot(&snap, "background");

    // 壳层已贴本方战报图；若另有正确调色板解码结果，铺满左区。
    if let Some(bg) = paint.backdrop {
        blit_stretched(&mut page, bg, background);
    }
    // 统计区外框由 Side chrome / adaptor 决定：
    // - 原版：半透明黑底 + 金边（`stats_shade`）
    // - 尤里：不叠黑底，画银灰金属描边（色取自 `mpyscrnl` 外框采样）
    if paint.stats_shade {
        blend_rect(&mut page, stats, [0, 0, 0, 168]);
        stroke_rect(&mut page, stats, [160, 140, 60, 200]);
    }
    else {
        paint_yuri_score_stats_frame(&mut page, stats);
    }
    if let Some(fnt) = fnt {
        let title_text = {
            let key = if paint.campaign { Some("GUI:STANDALONESCORE") } else { skirmish_score_csf_label("title") };
            let from = resolve_caption(csf, "title", key);
            if from == "title" {
                if paint.campaign { "任务积分".to_string() } else { skirmish_score_fallback_label("title").to_string() }
            }
            else {
                from
            }
        };
        blit_caption_in_cell(&mut page, fnt, &title_text, title.x, title.y, title.w, title.h, MENU_TEXT_ENABLED);

        let game_text = format!("游戏:{}", paint.game_index.max(1));
        blit_caption_top_left_clipped(&mut page, fnt, &game_text, game_label.x, game_label.y, game_label.w, game_label.h, MENU_TEXT_ENABLED);
        let time_key = skirmish_score_csf_label("time");
        let time_caption = {
            let from = resolve_caption(csf, "time", time_key);
            if from == "time" { skirmish_score_fallback_label("time").to_string() } else { from }
        };
        let time_text = format!("{time_caption}:{}", paint.time_text);
        blit_caption_top_right_clipped(&mut page, fnt, &time_text, time_label.x, time_label.y, time_label.w, time_label.h, MENU_TEXT_ENABLED);

        // 原版积分表：名列左齐，数值列与表头同右缘；五列均分剩余宽。
        let headers = ["player", "kills", "losses", "built", "score"];
        let name_w = 132i32;
        let rest = (table.w - name_w).max(0);
        let stat_w = rest / 4;
        let col_w = [name_w, stat_w, stat_w, stat_w, rest - stat_w * 3];
        let row_h = 20i32;
        let mut x = table.x;
        for (i, id) in headers.iter().enumerate() {
            let label = {
                let from = resolve_caption(csf, id, skirmish_score_csf_label(id));
                if from == *id { skirmish_score_fallback_label(id).to_string() } else { from }
            };
            if i == 0 {
                blit_caption_top_left_clipped(&mut page, fnt, &label, x, table.y, col_w[i], row_h, MENU_TEXT_SECTION);
            }
            else {
                blit_caption_top_right_clipped(&mut page, fnt, &label, x, table.y, col_w[i], row_h, MENU_TEXT_SECTION);
            }
            x += col_w[i];
        }

        let mut y = table.y + row_h + 8;
        for row in paint.rows.iter().take(12) {
            if y + row_h > table.y + table.h {
                break;
            }
            let mut cx = table.x;
            let cells = [row.name.as_str(), &row.kills.to_string(), &row.losses.to_string(), &row.built.to_string(), &row.score.to_string()];
            for (i, cell) in cells.iter().enumerate() {
                if i == 0 {
                    blit_caption_top_left_clipped(&mut page, fnt, cell, cx, y, col_w[i], row_h, row.color);
                }
                else {
                    blit_caption_top_right_clipped(&mut page, fnt, cell, cx, y, col_w[i], row_h, row.color);
                }
                cx += col_w[i];
            }
            y += row_h + 2;
        }
    }

    Some(page)
}

/// 尤里结算统计区金属框：外暗内亮双描边（银灰取自 `mpyscrnl` 外框），不叠原版黑底。
fn paint_yuri_score_stats_frame(page: &mut RgbaImage, stats: RectPx) {
    // 外框偏暗、内框偏亮，形成浅金属斜面。
    stroke_rect(page, stats, [76, 76, 76, 230]);
    if stats.w > 4 && stats.h > 4 {
        let inset = RectPx::new(stats.x + 1, stats.y + 1, stats.w - 2, stats.h - 2);
        stroke_rect(page, inset, [195, 199, 211, 220]);
    }
}

/// 窗口像素经 letterbox 映射后的壳层坐标命中「继续」。
pub fn skirmish_score_hit_at(x: i32, y: i32) -> Option<&'static str> {
    let snap = solve_skirmish_score();
    let cell = rect_px_from_snapshot(&snap, "continue");
    if x >= cell.x && y >= cell.y && x < cell.x + cell.w && y < cell.y + cell.h { Some("continue") } else { None }
}

/// tick → `HH:MM:SS`（按 15Hz 估算秒）。
pub fn format_score_time(duration_ticks: u64, tick_hz: u32) -> String {
    let hz = tick_hz.max(1) as u64;
    let secs = duration_ticks / hz;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
