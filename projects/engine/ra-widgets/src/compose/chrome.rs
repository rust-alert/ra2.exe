//! 壳层侧栏 / 波浪 chrome 合成。

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct ShellWaveFrames<'a> {
    /// 与当前页按钮 id 表对齐。
    pub buttons: &'a [u16],
    /// 与 `panel_tile_count` 对齐；仅当 `animate_empty_tiles` 时用于空格。
    pub tiles: &'a [u16],
    /// 空格是否叠钮面波浪（`SlideOut` 为 true，`SlideIn` / 间隙为 false）。
    pub animate_empty_tiles: bool,
}

/// 壳层按钮文案来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuCaptionKind {
    Main,
    SinglePlayer,
    Campaign,
    SkirmishLobby,
    ChooseMap,
    SkirmishScore,
}

impl MenuCaptionKind {
    pub(super) fn label(self, entry_id: &str) -> Option<&'static str> {
        match self {
            Self::Main => main_menu_csf_label(entry_id),
            Self::SinglePlayer => single_player_csf_label(entry_id),
            Self::Campaign => campaign_csf_label(entry_id),
            Self::SkirmishLobby => skirmish_lobby_csf_label(entry_id),
            Self::ChooseMap => choose_map_csf_label(entry_id),
            Self::SkirmishScore => skirmish_score_csf_label(entry_id),
        }
    }
}

pub(super) fn find_panel<'a>(decoded: &'a PageDecodeReport, needle: &str, anim_frame: usize) -> Option<&'a DecodedUiSprite> {
    let needle = needle.to_ascii_lowercase();
    let mut matches: Vec<&DecodedUiSprite> = decoded
        .panels
        .iter()
        .filter(|p| {
            let lab = p.label.to_ascii_lowercase();
            lab == needle || lab.starts_with(&format!("{needle}#"))
        })
        .collect();
    if matches.is_empty() {
        return None;
    }
    matches.sort_by_key(|p| p.frame);
    Some(matches[anim_frame % matches.len()])
}

/// 右栏顶盖：先画 `sdtp` 帧 0 外壳，再把 `sdwrnanm` 当前帧 1:1 贴进窗内（不拉伸、不盖金属边框）。
pub(super) fn blit_right_panel_top(page: &mut RgbaImage, decoded: &PageDecodeReport, panel_top: RectPx, warn_anim_frame: usize) {
    if let Some(top) = find_panel(decoded, "sdtp.shp", 0) {
        blit_stretched(page, &top.image, panel_top);
    }
    if let Some(warn) = find_panel(decoded, "sdwrnanm.shp", warn_anim_frame) {
        blit_rgba(page, &warn.image, panel_top.x + SDWRNANM_OFFSET_X, panel_top.y + SDWRNANM_OFFSET_Y);
    }
}

/// 右栏静态 chrome：顶盖 + `sdbtnbkgd` 瓦片 + 底盖 + 可选底条。
pub(super) fn paint_right_panel_chrome(
    page: &mut RgbaImage,
    decoded: &PageDecodeReport,
    panel_top: RectPx,
    panel_tile: RectPx,
    panel_tile_count: i32,
    panel_bottom: RectPx,
    lower_strip: RectPx,
    warn_anim_frame: usize,
) {
    blit_right_panel_top(page, decoded, panel_top, warn_anim_frame);
    if let Some(tile) = find_panel(decoded, "sdbtnbkgd.shp", 0) {
        for i in 0..panel_tile_count {
            let r = RectPx::new(panel_tile.x, panel_tile.y + i * panel_tile.h, panel_tile.w, panel_tile.h);
            blit_stretched(page, &tile.image, r);
        }
    }
    if let Some(bottom) = find_panel(decoded, "sdbtm.shp", 0) {
        blit_stretched(page, &bottom.image, panel_bottom);
    }
    if lower_strip.w > 0 && lower_strip.h > 0 {
        if let Some(lower) = find_panel(decoded, "lwscrnl.shp", 0) {
            blit_stretched(page, &lower.image, lower_strip);
        }
    }
}

/// 由 snapshot 的 `panel_tile` / `panel_bottom` 推导右栏瓦片格数。
pub(super) fn panel_tile_count_from_snap(snap: &LayoutSnapshot) -> i32 {
    let tile = rect_px_from_snapshot(snap, "panel_tile");
    let bottom = rect_px_from_snapshot(snap, "panel_bottom");
    if tile.h > 0 { ((bottom.y - tile.y) / tile.h).max(0) } else { 0 }
}

/// 遭遇战 / 选图：在壳层 `sdtp` 帧 0 之上叠帧 1 顶栏高亮牌，再贴 `sdmpbtn` 地图名底板。
pub(super) fn blit_skirmish_preview_chrome(page: &mut RgbaImage, decoded: &PageDecodeReport, panel_top: RectPx, map_name_plate: RectPx) {
    if let Some(top1) = find_panel(decoded, "sdtp.shp", 1) {
        blit_stretched(page, &top1.image, panel_top);
    }
    if let Some(plate) = find_panel(decoded, "sdmpbtn.shp", 0) {
        // 与壳层钮面一致：1:1 贴右缘，不拉伸。
        blit_rgba(page, &plate.image, map_name_plate.x, map_name_plate.y);
    }
}

/// 右栏静态标题：格内水平+垂直居中。
pub(super) fn blit_shell_static_title(page: &mut RgbaImage, fnt: &FntFile, text: &str, cell: RectPx) {
    blit_caption_in_cell(page, fnt, text, cell.x, cell.y, cell.w, cell.h, MENU_TEXT_ENABLED);
}

/// 控件格内文字基线 y：按字高垂直居中。
pub(super) fn text_y_centered(fnt: &FntFile, cell: RectPx) -> i32 {
    let th = fnt.bitmap_rows as i32;
    cell.y + ((cell.h - th).max(0) / 2)
}

pub(super) fn find_button_normal<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_normals.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

pub(super) fn find_button_hover<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_hovers.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

pub(super) fn find_button_pressed<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_presseds.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

/// 按按下 / 悬停态挑选钮面；缺图时回落到常态帧。
pub(super) fn resolve_button_sprite<'a>(
    decoded: &'a PageDecodeReport,
    entry_id: &str,
    pressed: bool,
    hovered: bool,
) -> Option<&'a DecodedUiSprite> {
    let normal = find_button_normal(decoded, entry_id)?;
    if pressed {
        Some(find_button_pressed(decoded, entry_id).unwrap_or(normal))
    }
    else if hovered {
        Some(find_button_hover(decoded, entry_id).unwrap_or(normal))
    }
    else {
        Some(normal)
    }
}

/// 主菜单 owner-draw 文案裁切：未按 `+0/+1/-2/-1`，按下 `+2/+5/-4/-5`。
pub(super) fn owner_draw_caption_rect(cell: RectPx, pressed: bool) -> (i32, i32, i32, i32) {
    let (dx, dy) = if pressed { (2, 5) } else { (0, 1) };
    (cell.x + dx, cell.y + dy, (cell.w - 2 - dx).max(0), (cell.h - dy).max(0))
}
