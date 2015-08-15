//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub(super) fn row_side_name<'a>(paint: &'a SkirmishLobbyPaint<'_>, row: usize) -> &'a str {
    if paint.sides.is_empty() {
        return "";
    }
    let i = paint.row_side_indices[row.min(paint.row_side_indices.len() - 1)] as usize % paint.sides.len();
    &paint.sides[i]
}

pub(super) fn row_color_rgb(paint: &SkirmishLobbyPaint<'_>, row: usize) -> [u8; 3] {
    let i = paint.row_color_indices[row.min(paint.row_color_indices.len() - 1)] as usize % LOBBY_COLORS.len();
    LOBBY_COLORS[i]
}

pub(super) fn row_flag(chrome: Option<&SkirmishChromeSprites>, row: usize) -> Option<&RgbaImage> {
    chrome.and_then(|c| c.row_flags.get(row).and_then(|f| f.as_ref()).or_else(|| if row == 0 { c.flag.as_ref() } else { c.ai_flag.as_ref() }))
}

pub(super) fn blit_flag(dst: &mut RgbaImage, flag: Option<&RgbaImage>, rect: RectPx) {
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
    /// 数值底板左帽 `trofl.pcx`（非轨道）。
    pub track_cap_l: Option<RgbaImage>,
    /// 数值底板中段 `trofm.pcx`。
    pub track_cap_m: Option<RgbaImage>,
    /// 数值底板右帽 `trofr.pcx`。
    pub track_cap_r: Option<RgbaImage>,
    /// 下拉箭头常态 `dnarrowr.pcx`。
    pub combo_arrow: Option<RgbaImage>,
    /// 下拉箭头按下 `dnarrowp.pcx`。
    pub combo_arrow_pressed: Option<RgbaImage>,
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
    /// 右栏游戏类型显示名（来自选中 `mpmodes` 的 CSF）。
    pub game_type_name: &'a str,
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
    /// 可选国家短名（与装载请求 `sides` 同步）。
    pub sides: &'a [String],
    /// 各行国家下标（相对 `sides`）。
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
            game_type_name: "",
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
            sides: &[],
            row_side_indices: [0, 1, 2, 3, 4, 0, 1, 2],
            row_color_indices: [0, 1, 2, 3, 4, 5, 6, 7],
            chrome: None,
        }
    }
}

pub(super) fn paint_skirmish_lobby_controls(
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
    draw_edit_face(page, layout.player_name, name_face);
    let local_rgb = row_color_rgb(paint, 0);
    draw_combo_face(page, layout.side_faces[0], [16, 16, 20, 255], chrome, paint.country_combo_open && paint.combo_row == 0);
    draw_color_combo_face(page, layout.color_faces[0], local_rgb, chrome, paint.color_combo_open && paint.combo_row == 0);
    blit_flag(page, row_flag(chrome, 0), layout.flags[0]);

    let ai_rows = paint.ai_rows.min(layout.ai_faces.len());
    for i in 0..ai_rows {
        draw_combo_face(page, layout.ai_faces[i], [16, 16, 20, 255], chrome, paint.ai_combo_open && i == 0);
        let human_row = i + 1;
        if human_row < layout.side_faces.len() {
            draw_combo_face(
                page,
                layout.side_faces[human_row],
                [16, 16, 20, 255],
                chrome,
                paint.country_combo_open && paint.combo_row == human_row,
            );
        }
        if human_row < layout.color_faces.len() {
            let rgb = row_color_rgb(paint, human_row);
            draw_color_combo_face(
                page,
                layout.color_faces[human_row],
                rgb,
                chrome,
                paint.color_combo_open && paint.combo_row == human_row,
            );
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
        // 数值画在右侧底板内；源色 0x00000C05 → RGB(5,12,0)。
        const TRACK_VALUE: [u8; 4] = [5, 12, 0, 255];
        let value_x = |track: RectPx| track.x + track.w - 0x31;
        blit_text_colored(page, fnt, &paint.game_speed.to_string(), value_x(layout.track_speed), layout.track_speed.y + 2, TRACK_VALUE);
        blit_text_colored(page, fnt, &paint.credits.to_string(), value_x(layout.track_credits), layout.track_credits.y + 2, TRACK_VALUE);
        blit_text_colored(page, fnt, &paint.unit_count.to_string(), value_x(layout.track_units), layout.track_units.y + 2, TRACK_VALUE);
    }

    if paint.country_combo_open && !paint.sides.is_empty() {
        let list = crate::skirmish_setup::SkirmishBootRequest::country_list_rect(layout, paint.combo_row, paint.sides.len());
        fill_rect(page, list, [12, 12, 18, 255]);
        stroke_rect(page, list, [180, 24, 24, 255]);
        let selected_side = row_side_name(paint, paint.combo_row);
        for (i, side) in paint.sides.iter().enumerate() {
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

    // 右栏：`sdtp` 帧 1 标题牌 + `sdmpbtn` 地图名底板；预览等比落入 `0x468` 黑窗（无红描边）。
    blit_skirmish_preview_chrome(&mut page, decoded, layout.shell.panel_top, layout.map_name_plate);
    if let Some(preview) = map_preview {
        blit_map_preview_fit(&mut page, preview, layout.map_preview);
    }
    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "skirmish", Some(skirmish_title_csf_key()));
        blit_shell_static_title(&mut page, fnt, &title, layout.title);
        if !paint.game_type_name.is_empty() {
            blit_text_colored(&mut page, fnt, paint.game_type_name, layout.game_type.x, layout.game_type.y, MENU_TEXT_ENABLED);
        }
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
