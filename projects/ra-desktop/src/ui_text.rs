//! 壳层菜单文字：FNT 量宽 + 着色绘制；CSF 标签查找。

use ra_assets::{CsfFile, FntFile};
use ra_renderer::RgbaImage;

/// 主菜单入口 → CSF 标签。
pub fn main_menu_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "single_player" => Some("GUI:SinglePlayer"),
        "network" => Some("GUI:NetworkOnline"),
        "options" => Some("GUI:Options"),
        "exit" => Some("GUI:ExitGame"),
        _ => None,
    }
}

/// 单人页入口 → CSF 标签。
pub fn single_player_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "campaign" => Some("GUI:NewCampaign"),
        "skirmish" => Some("GUI:Skirmish"),
        "training" => Some("GUI:LoadScenario"),
        "back" => Some("GUI:Back"),
        _ => None,
    }
}

/// 遭遇战大厅入口 → CSF 标签。
pub fn skirmish_lobby_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "side" => Some("GUI:PlayerSide"),
        "difficulty" => Some("GUI:Difficulty"),
        "start" => Some("GUI:Battle"),
        "back" => Some("GUI:Back"),
        _ => None,
    }
}

/// 解析文案：CSF 命中优先，否则回退 `entry_id`。
pub fn resolve_caption<'a>(csf: Option<&'a CsfFile>, entry_id: &str, csf_key: Option<&str>) -> String {
    if let (Some(csf), Some(key)) = (csf, csf_key) {
        if let Some(text) = csf.get(key) {
            if !text.is_empty() {
                return text.to_string();
            }
        }
    }
    entry_id.replace('_', " ")
}

/// 启用按钮常用黄字（近似原版壳层）。
pub const MENU_TEXT_ENABLED: [u8; 4] = [255, 214, 0, 255];
/// 禁用按钮灰字。
pub const MENU_TEXT_DISABLED: [u8; 4] = [128, 128, 128, 255];

/// 将白字字形着色后画到目标（字距 1px）。
pub fn blit_text_colored(dst: &mut RgbaImage, fnt: &FntFile, text: &str, x: i32, y: i32, rgba: [u8; 4]) {
    let mut pen_x = x;
    let mut first = true;
    for ch in text.chars() {
        let cp = ch as u32;
        if cp > u32::from(u16::MAX) {
            continue;
        }
        let Some(glyph) = fnt.glyph(cp as u16)
        else {
            continue;
        };
        if !first {
            pen_x += 1;
        }
        first = false;
        let tinted = tint_glyph(&glyph.rgba, glyph.width, fnt.bitmap_rows, rgba);
        if let Some(img) = RgbaImage::from_raw(glyph.width, fnt.bitmap_rows, tinted) {
            blit_glyph(dst, &img, pen_x, y);
        }
        pen_x += glyph.width as i32;
    }
}

fn blit_glyph(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    for row in 0..src.height() {
        let dy = y + row as i32;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..src.width() {
            let dx = x + col as i32;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let si = ((row * src.width() + col) * 4) as usize;
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            let sa = src.as_raw()[si + 3];
            if sa == 0 {
                continue;
            }
            dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
        }
    }
}

/// 在按钮格内水平居中绘制一行（垂直偏上贴近原版）。
pub fn blit_caption_in_cell(
    dst: &mut RgbaImage,
    fnt: &FntFile,
    text: &str,
    cell_x: i32,
    cell_y: i32,
    cell_w: i32,
    cell_h: i32,
    rgba: [u8; 4],
) {
    let tw = fnt.text_width(text) as i32;
    let th = fnt.bitmap_rows as i32;
    let x = cell_x + ((cell_w - tw).max(0) / 2);
    let y = cell_y + ((cell_h - th).max(0) / 2);
    blit_text_colored(dst, fnt, text, x, y, rgba);
}

fn tint_glyph(src: &[u8], width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut out = vec![0u8; (width * height * 4) as usize];
    for i in 0..(width * height) as usize {
        let a = src[i * 4 + 3];
        if a == 0 {
            continue;
        }
        out[i * 4] = rgba[0];
        out[i * 4 + 1] = rgba[1];
        out[i * 4 + 2] = rgba[2];
        out[i * 4 + 3] = ((u32::from(a) * u32::from(rgba[3])) / 255) as u8;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::FntFile;

    fn tiny_fnt() -> FntFile {
        // 复用 fnt 模块测试构造思路的最小子集：直接 parse 内联字节。
        const FONT_MAGIC: u32 = 0x546E_6F66;
        const LOOKUP: usize = 65536 * 2;
        let mut data = Vec::new();
        data.extend_from_slice(&FONT_MAGIC.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        let mut lut = vec![0u8; LOOKUP];
        let off = (b'A' as usize) * 2;
        lut[off] = 1;
        data.extend_from_slice(&lut);
        data.push(1);
        data.push(0b1000_0000);
        FntFile::parse(&data).unwrap()
    }

    #[test]
    fn resolve_caption_falls_back_to_entry_id() {
        assert_eq!(resolve_caption(None, "single_player", Some("GUI:SinglePlayer")), "single player");
    }

    #[test]
    fn blit_text_writes_colored_pixel() {
        let fnt = tiny_fnt();
        let mut dst = RgbaImage::from_raw(4, 4, vec![0u8; 64]).unwrap();
        blit_text_colored(&mut dst, &fnt, "A", 0, 0, [255, 214, 0, 255]);
        assert_eq!(&dst.as_raw()[0..4], &[255, 214, 0, 255]);
    }
}
