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
    /// 战役结算用 `GUI:STANDALONESCORE`，否则 `GUI:SKIRMISHSCORE`。
    pub campaign: bool,
}

const HEADER_COLS: [&str; 5] = ["player", "kills", "losses", "built", "score"];
const HEADER_SUFFIXES: [&str; 5] = ["name", "kills", "losses", "built", "score"];

/// 合成遭遇战积分页：壳层右栏 + `0x108` 表叶 + 「继续」。
///
/// 表画在壳层 `mnscrnl` 背景上；**不**叠 `mp*scrnl` 战报图，也**不**另画统计区黑底框。
pub fn compose_skirmish_score_page(
    decoded: &PageDecodeReport,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    paint: SkirmishScorePaint<'_>,
    movie: Option<&RgbaImage>,
    wave: Option<ShellWaveFrames<'_>>,
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
        movie,
        MenuCaptionKind::SkirmishScore,
        wave,
        warn_anim_frame,
    )?;

    let game_label = rect_px_from_snapshot(&snap, "game_label");
    let time_label = rect_px_from_snapshot(&snap, "time_label");
    let title = rect_px_from_snapshot(&snap, "title");

    if let Some(fnt) = fnt {
        let title_text = {
            let key = if paint.campaign { Some("GUI:STANDALONESCORE") } else { skirmish_score_csf_label("title") };
            let from = resolve_caption(csf, "title", key);
            if from == "title" {
                if paint.campaign {
                    "任务积分".to_string()
                } else {
                    skirmish_score_fallback_label("title").to_string()
                }
            } else {
                from
            }
        };
        blit_caption_in_cell(&mut page, fnt, &title_text, title.x, title.y, title.w, title.h, MENU_TEXT_ENABLED);

        let game_text = format!("游戏:{}", paint.game_index.max(1));
        blit_caption_top_left_clipped(
            &mut page,
            fnt,
            &game_text,
            game_label.x,
            game_label.y,
            game_label.w,
            game_label.h,
            MENU_TEXT_ENABLED,
        );
        let time_key = skirmish_score_csf_label("time");
        let time_caption = {
            let from = resolve_caption(csf, "time", time_key);
            if from == "time" {
                skirmish_score_fallback_label("time").to_string()
            } else {
                from
            }
        };
        let time_text = format!("{time_caption}:{}", paint.time_text);
        blit_caption_top_right_clipped(
            &mut page,
            fnt,
            &time_text,
            time_label.x,
            time_label.y,
            time_label.w,
            time_label.h,
            MENU_TEXT_ENABLED,
        );

        for (i, id) in HEADER_COLS.iter().enumerate() {
            let cell = rect_px_from_snapshot(&snap, &format!("header_{}", HEADER_SUFFIXES[i]));
            let label = {
                let from = resolve_caption(csf, id, skirmish_score_csf_label(id));
                if from == *id {
                    skirmish_score_fallback_label(id).to_string()
                } else {
                    from
                }
            };
            if i == 0 {
                blit_caption_top_left_clipped(&mut page, fnt, &label, cell.x, cell.y, cell.w, cell.h, MENU_TEXT_SECTION);
            } else {
                blit_caption_top_right_clipped(&mut page, fnt, &label, cell.x, cell.y, cell.w, cell.h, MENU_TEXT_SECTION);
            }
        }

        for (slot, row) in paint.rows.iter().take(ra_layout::SCORE_ROW_SLOTS).enumerate() {
            let kills = row.kills.to_string();
            let losses = row.losses.to_string();
            let built = row.built.to_string();
            let score = row.score.to_string();
            let cells = [row.name.as_str(), kills.as_str(), losses.as_str(), built.as_str(), score.as_str()];
            for (i, text) in cells.iter().enumerate() {
                let cell = rect_px_from_snapshot(&snap, &format!("row{slot}_{}", HEADER_SUFFIXES[i]));
                if i == 0 {
                    blit_caption_top_left_clipped(&mut page, fnt, text, cell.x, cell.y, cell.w, cell.h, row.color);
                } else {
                    blit_caption_top_right_clipped(&mut page, fnt, text, cell.x, cell.y, cell.w, cell.h, row.color);
                }
            }
        }
    }

    Some(page)
}

/// 窗口像素经 letterbox 映射后的壳层坐标命中「继续」。
pub fn skirmish_score_hit_at(x: i32, y: i32) -> Option<&'static str> {
    let snap = solve_skirmish_score();
    let cell = rect_px_from_snapshot(&snap, "continue");
    if x >= cell.x && y >= cell.y && x < cell.x + cell.w && y < cell.y + cell.h {
        Some("continue")
    } else {
        None
    }
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
