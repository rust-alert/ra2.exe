//! 选中供电建筑时的「电力 / 负载」浮动条（对齐零售 `TXT_POWER_DRAIN2`）。
//!
//! 数值为本方（建筑所属 house）**全局**有效供电与总负载，不是单座建筑的 `Power=`。
//! 文字与边框用阵营色。

use ra_assets::{CsfFile, FntFile};
use ra_layout::RectPx;
use ra_renderer::RgbaImage;

use crate::{
    compose::{fill_rect, stroke_rect},
    skin::text::{blit_text_colored, resolve_csf_text},
};

/// 单行模板（零售主用）：`电力 = %d 负载 = %d`。
pub const TXT_POWER_DRAIN2: &str = "TXT_POWER_DRAIN2";
/// 双行模板（回退）：`电力=%d \n 负载=%d`。
pub const TXT_POWER_DRAIN: &str = "TXT_POWER_DRAIN";

/// 浮动条底色（黑底）。
pub const POWER_TIP_BG: [u8; 4] = [0, 0, 0, 220];
/// 缺阵营色时的回退文字色。
pub const POWER_TIP_TEXT_FALLBACK: [u8; 4] = [220, 220, 220, 255];

/// 将 CSF 模板中的 `%d` 按序替换为整型参数。
pub fn format_csf_percent_d(template: &str, values: &[i32]) -> String {
    let mut out = String::with_capacity(template.len() + 8);
    let mut rest = template;
    let mut vi = 0usize;
    while let Some(pos) = rest.find("%d") {
        out.push_str(&rest[..pos]);
        if vi < values.len() {
            out.push_str(&values[vi].to_string());
            vi += 1;
        }
        else {
            out.push_str("%d");
        }
        rest = &rest[pos + 2..];
    }
    out.push_str(rest);
    out
}

/// 解析 `TXT_POWER_DRAIN2`（缺省回退 `TXT_POWER_DRAIN` / 简体占位）并填入供电与负载。
pub fn selection_power_drain_caption(csf: Option<&CsfFile>, power: i32, load: i32) -> String {
    let raw = resolve_csf_text(csf, TXT_POWER_DRAIN2)
        .or_else(|| resolve_csf_text(csf, TXT_POWER_DRAIN))
        .unwrap_or_else(|| "电力 = %d 负载 = %d".into());
    // 文本表导出把换行写成 `\n`；CSF 本体也可能带真换行。
    let template = raw.replace("\\n", "\n");
    format_csf_percent_d(&template, &[power, load])
}

/// 把阵营主色调亮一点，便于黑底可读。
pub fn power_tip_rgba_from_primary(r: u8, g: u8, b: u8) -> [u8; 4] {
    let lift = |c: u8| -> u8 { c.saturating_add((255u16.saturating_sub(u16::from(c)) / 3) as u8) };
    [lift(r), lift(g), lift(b), 255]
}

/// 在窗口像素 `(center_x, center_y)` 居中绘制黑底阵营色电力提示。
pub fn paint_selection_power_tip(
    page: &mut RgbaImage,
    fnt: &FntFile,
    caption: &str,
    center_x: i32,
    center_y: i32,
    page_w: i32,
    page_h: i32,
    faction_rgba: [u8; 4],
) {
    let lines: Vec<&str> = caption.lines().filter(|l| !l.trim().is_empty()).collect();
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
    let mut bx = center_x - box_w / 2;
    let mut by = center_y - box_h / 2;
    if bx < 0 {
        bx = 0;
    }
    if by < 0 {
        by = 0;
    }
    if bx + box_w > page_w {
        bx = (page_w - box_w).max(0);
    }
    if by + box_h > page_h {
        by = (page_h - box_h).max(0);
    }
    fill_rect(page, RectPx::new(bx, by, box_w, box_h), POWER_TIP_BG);
    stroke_rect(page, RectPx::new(bx, by, box_w, box_h), faction_rgba);
    let mut ty = by + pad_y;
    for line in lines {
        let tw = fnt.text_width(line) as i32;
        let tx = bx + (box_w - tw) / 2;
        blit_text_colored(page, fnt, line, tx, ty, faction_rgba);
        ty += line_h + line_gap;
    }
}

/// 建筑占地在预览图像素中的几何中心（与选中括号同源）。
pub fn structure_selection_center_preview(screen_x: i32, screen_y: i32, foundation_w: u16, foundation_h: u16, art_height: u16) -> (f32, f32) {
    let fw = foundation_w.max(1) as f32;
    let fh = foundation_h.max(1) as f32;
    let cx = screen_x as f32 + (fw - fh) * 15.0;
    // 抬到建筑中部，避免贴在占地底边。
    let cy = screen_y as f32 + (fw + fh) * 7.5 - 15.0 - art_height.max(1) as f32 * 15.0 * 0.45;
    (cx, cy)
}
