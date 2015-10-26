//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub struct CampaignPaint<'a> {
    /// 已选侧：`allied` / `tutorial` / `soviet`。
    pub selected_side: Option<&'static str>,
    /// 难度档：0 易 / 1 中 / 2 难。
    pub difficulty: u8,
    /// 难度滑条拇指（安装内 `trakgrip.pcx`，可空）。
    pub track_thumb: Option<&'a RgbaImage>,
    /// 侧图悬停/已选箭头动画帧（对侧图 SHP 帧数取模）。
    pub side_anim_frame: usize,
}

impl Default for CampaignPaint<'_> {
    fn default() -> Self {
        Self { selected_side: None, difficulty: 1, track_thumb: None, side_anim_frame: 1 }
    }
}

/// `fsscrn.pal` 下侧图空区近黑 RGB 和阈值（约 (8,8,8)）。
pub(super) const CAMPAIGN_SIDE_NEAR_BLACK_SUM: u16 = 32;

/// 合成战役选边页：三侧图 + 难度 + 右栏返回。
///
/// 几何权威为 `solve_campaign` snapshot；右栏 chrome 经 `shell_rail_layout_from_snap` 供共用合成入口。
pub fn compose_campaign_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    paint: CampaignPaint<'_>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let _ = (viewport_w, viewport_h);
    let snap = ra_layout::solve_campaign();
    let shell = shell_rail_layout_from_snap(&snap, &CAMPAIGN_BUTTON_IDS);
    let allied = rect_px_from_snapshot(&snap, ra_layout::CAMPAIGN_SIDE_IDS[0]);
    let tutorial = rect_px_from_snapshot(&snap, ra_layout::CAMPAIGN_SIDE_IDS[1]);
    let soviet = rect_px_from_snapshot(&snap, ra_layout::CAMPAIGN_SIDE_IDS[2]);
    let difficulty_label = rect_px_from_snapshot(&snap, "difficulty_label");
    let difficulty_value = rect_px_from_snapshot(&snap, "difficulty_value");
    let difficulty_track = rect_px_from_snapshot(&snap, "difficulty");
    let title = rect_px_from_snapshot(&snap, "title");
    let status_help = rect_px_from_snapshot(&snap, "tooltip");

    let mut page = compose_shell_menu_page(
        decoded,
        shell,
        &CAMPAIGN_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        None,
        fnt,
        csf,
        None,
        MenuCaptionKind::Campaign,
        wave,
        warn_anim_frame,
    )?;

    let sides = [
        ("allied", "fsalg.shp", allied),
        ("tutorial", "fsbclg.shp", tutorial),
        ("soviet", "fsslg.shp", soviet),
    ];
    for (id, shp, rect) in sides {
        // `fsbkgdlg` 已烘焙静态徽标。勿整幅不透明拉伸侧图（近黑空区 → 黑块重影）。
        // 悬停/已选：1:1 近黑透叠箭头动画帧。
        let active = paint.selected_side == Some(id) || hovered_entry_id == Some(id);
        if active {
            // 三侧同一套：相对 frame0 差分贴像素，只叠箭头动画，不重画烘焙徽标。
            let base = find_panel(decoded, shp, 0);
            let frame = paint.side_anim_frame.max(1);
            if let Some(sprite) = find_panel(decoded, shp, frame) {
                blit_rgba_diff_from_base(
                    &mut page,
                    &sprite.image,
                    base.map(|b| &b.image),
                    rect.x,
                    rect.y,
                    CAMPAIGN_SIDE_NEAR_BLACK_SUM,
                );
            }
        }
    }

    // 难度轨：底槽 + 档位拇指（优先安装内 `trakgrip.pcx`，控件 `0x50F`）。
    fill_rect(&mut page, difficulty_track, [64, 16, 16, 255]);
    let inner = RectPx::new(
        difficulty_track.x + 2,
        difficulty_track.y + 2,
        (difficulty_track.w - 4).max(1),
        (difficulty_track.h - 4).max(1),
    );
    fill_rect(&mut page, inner, [12, 12, 16, 255]);
    let level = i32::from(paint.difficulty.min(2));
    let thumb_w = paint.track_thumb.map(|t| t.width() as i32).unwrap_or(10);
    let travel = (inner.w - thumb_w).max(1);
    let thumb_x = inner.x + (level * travel) / 2;
    if let Some(thumb) = paint.track_thumb {
        let ty = difficulty_track.y + (difficulty_track.h - thumb.height() as i32) / 2;
        blit_rgba(&mut page, thumb, thumb_x, ty);
    } else {
        fill_rect(
            &mut page,
            RectPx::new(thumb_x, inner.y - 1, thumb_w, inner.h + 2),
            [220, 40, 40, 255],
        );
    }

    if let Some(fnt) = fnt {
        let title_text = resolve_caption(csf, "campaign", Some(campaign_title_csf_key()));
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
        let diff_label = resolve_caption(csf, "difficulty", Some("GUI:Difficulty"));
        blit_text_colored(
            &mut page,
            fnt,
            &diff_label,
            difficulty_label.x,
            difficulty_label.y,
            MENU_TEXT_ENABLED,
        );
        let diff_value = resolve_caption(
            csf,
            "difficulty_value",
            Some(campaign_difficulty_csf_key(paint.difficulty)),
        );
        blit_text_colored(
            &mut page,
            fnt,
            &diff_value,
            difficulty_value.x,
            difficulty_value.y,
            MENU_TEXT_ENABLED,
        );
        if let Some(text) = status_text.filter(|s| !s.is_empty()) {
            blit_text_colored(
                &mut page,
                fnt,
                text,
                status_help.x,
                status_help.y,
                MENU_TEXT_ENABLED,
            );
        }
    }

    Some(page)
}
