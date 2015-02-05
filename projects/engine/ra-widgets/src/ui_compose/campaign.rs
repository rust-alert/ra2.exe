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

/// 合成战役选边页：三侧图 + 难度 + 右栏载入/返回。
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
    let layout = campaign_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
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

    let sides = [("allied", "fsalg.shp", layout.allied), ("tutorial", "fsbclg.shp", layout.tutorial), ("soviet", "fsslg.shp", layout.soviet)];
    for (id, shp, rect) in sides {
        // `fsbkgdlg` 已烘焙静态徽标。勿整幅不透明拉伸侧图（近黑空区 → 黑块重影）。
        // 悬停/已选：1:1 近黑透叠箭头动画帧。
        let active = paint.selected_side == Some(id) || hovered_entry_id == Some(id);
        if active {
            // 三侧同一套：相对 frame0 差分贴像素，只叠箭头动画，不重画烘焙徽标。
            let base = find_panel(decoded, shp, 0);
            let frame = paint.side_anim_frame.max(1);
            if let Some(sprite) = find_panel(decoded, shp, frame) {
                blit_rgba_diff_from_base(&mut page, &sprite.image, base.map(|b| &b.image), rect.x, rect.y, CAMPAIGN_SIDE_NEAR_BLACK_SUM);
            }
        }
    }

    // 难度轨：底槽 + 档位拇指（优先安装内 `trakgrip.pcx`，控件 `0x50F`）。
    fill_rect(&mut page, layout.difficulty_track, [64, 16, 16, 255]);
    let inner = RectPx::new(
        layout.difficulty_track.x + 2,
        layout.difficulty_track.y + 2,
        (layout.difficulty_track.w - 4).max(1),
        (layout.difficulty_track.h - 4).max(1),
    );
    fill_rect(&mut page, inner, [12, 12, 16, 255]);
    let level = i32::from(paint.difficulty.min(2));
    let thumb_w = paint.track_thumb.map(|t| t.width() as i32).unwrap_or(10);
    let travel = (inner.w - thumb_w).max(1);
    let thumb_x = inner.x + (level * travel) / 2;
    if let Some(thumb) = paint.track_thumb {
        let ty = layout.difficulty_track.y + (layout.difficulty_track.h - thumb.height() as i32) / 2;
        blit_rgba(&mut page, thumb, thumb_x, ty);
    }
    else {
        fill_rect(&mut page, RectPx::new(thumb_x, inner.y - 1, thumb_w, inner.h + 2), [220, 40, 40, 255]);
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "campaign", Some(campaign_title_csf_key()));
        blit_caption_in_cell(&mut page, fnt, &title, layout.title.x, layout.title.y, layout.title.w, layout.title.h, MENU_TEXT_ENABLED);
        let diff_label = resolve_caption(csf, "difficulty", Some("GUI:Difficulty"));
        blit_text_colored(&mut page, fnt, &diff_label, layout.difficulty_label.x, layout.difficulty_label.y, MENU_TEXT_ENABLED);
        let diff_value = resolve_caption(csf, "difficulty_value", Some(campaign_difficulty_csf_key(paint.difficulty)));
        blit_text_colored(&mut page, fnt, &diff_value, layout.difficulty_value.x, layout.difficulty_value.y, MENU_TEXT_ENABLED);
        if let Some(text) = status_text.filter(|s| !s.is_empty()) {
            blit_text_colored(&mut page, fnt, text, layout.status_help.x, layout.status_help.y, MENU_TEXT_ENABLED);
        }
    }

    Some(page)
}
