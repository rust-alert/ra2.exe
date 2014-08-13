//! 集成测试：原 `src/ui_text.rs` 内联测试迁出。

use ra_assets::FntFile;
use ra_desktop::ui_text::*;
use ra_renderer::RgbaImage;

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
fn single_player_csf_keys_match_shell_labels() {
    assert_eq!(single_player_csf_label("campaign"), Some("GUI:NewCampaign"));
    assert_eq!(single_player_csf_label("load"), Some("GUI:LoadSavedGame"));
    assert_eq!(single_player_csf_label("skirmish"), Some("GUI:Skirmish"));
    assert_eq!(single_player_csf_label("back"), Some("GUI:MainMenu"));
    assert_eq!(single_player_title_csf_key(), "GUI:SinglePlayer");
}

#[test]
fn blit_text_writes_colored_pixel() {
    let fnt = tiny_fnt();
    let mut dst = RgbaImage::from_raw(4, 4, vec![0u8; 64]).unwrap();
    blit_text_colored(&mut dst, &fnt, "A", 0, 0, [255, 214, 0, 255]);
    assert_eq!(&dst.as_raw()[0..4], &[255, 214, 0, 255]);
}
