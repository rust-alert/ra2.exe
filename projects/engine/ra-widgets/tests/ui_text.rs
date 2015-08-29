//! 集成测试：原 `src/ui_text.rs` 内联测试迁出。

use ra_assets::{CsfFile, FntFile, LABEL_MAGIC, STRING_MAGIC};
use ra_widgets::ui_text::*;
use ra_renderer::RgbaImage;

const CSF_HEADER_MAGIC: u32 = 0x4353_4620; // " FSC"

fn encode_not_utf16(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for u in s.encode_utf16() {
        out.extend_from_slice(&(!u).to_le_bytes());
    }
    out
}

fn csf_with_entries(entries: &[(&str, &str)]) -> CsfFile {
    let mut data = Vec::new();
    data.extend_from_slice(&CSF_HEADER_MAGIC.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    for (label, value) in entries {
        let value_bytes = encode_not_utf16(value);
        data.extend_from_slice(&LABEL_MAGIC.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(label.len() as u32).to_le_bytes());
        data.extend_from_slice(label.as_bytes());
        data.extend_from_slice(&STRING_MAGIC.to_le_bytes());
        data.extend_from_slice(&((value_bytes.len() / 2) as u32).to_le_bytes());
        data.extend_from_slice(&value_bytes);
    }
    CsfFile::parse(&data).unwrap()
}

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
    assert_eq!(single_player_title_csf_key(), "GUI:SinglePlayerMenu");
    assert_eq!(single_player_csf_tooltip("campaign"), Some("STT:SingleButtonNewCampaign"));
    assert_eq!(single_player_csf_tooltip("load"), Some("STT:SingleButtonLoadSavedGame"));
    assert_eq!(single_player_csf_tooltip("skirmish"), Some("STT:SingleButtonSkirmish"));
    assert_eq!(single_player_csf_tooltip("back"), Some("STT:SingleButtonBack"));
}

#[test]
fn skirmish_lobby_csf_tooltips_match_stt_keys() {
    assert_eq!(skirmish_lobby_csf_tooltip("start"), Some("STT:SkirmishButtonStartGame"));
    assert_eq!(skirmish_lobby_csf_tooltip("choose_map"), Some("STT:SkirmishButtonChooseMap"));
    assert_eq!(skirmish_lobby_csf_tooltip("back"), Some("STT:SkirmishButtonBack"));
    assert_eq!(skirmish_lobby_csf_tooltip("short_game"), Some("STT:SkirmishCBoxShortGame"));
    assert_eq!(skirmish_lobby_csf_tooltip("speed"), Some("STT:SkirmishSliderSpeed"));
    assert_eq!(skirmish_lobby_csf_tooltip("country"), Some("STT:SkirmishComboCountry"));
    assert_eq!(skirmish_lobby_csf_tooltip("player_name"), Some("STT:SkirmishEditPlayer"));
}

#[test]
fn campaign_csf_keys_match_shell_labels() {
    assert_eq!(campaign_csf_label("load"), Some("GUI:Load"));
    assert_eq!(campaign_csf_label("back"), Some("GUI:Back"));
    assert_eq!(campaign_title_csf_key(), "GUI:CampaignMenu");
    assert_eq!(campaign_csf_tooltip("allied"), Some("STT:CampaignAnimAllied"));
    assert_eq!(campaign_csf_tooltip("tutorial"), Some("STT:CampaignAnimTutorial"));
    assert_eq!(campaign_csf_tooltip("soviet"), Some("STT:CampaignAnimSoviet"));
    assert_eq!(campaign_csf_tooltip("difficulty"), Some("STT:CampaignSliderDifficulty"));
    assert_eq!(campaign_difficulty_csf_key(0), "TXT_EASY");
    assert_eq!(campaign_difficulty_csf_key(1), "TXT_NORMAL");
    assert_eq!(campaign_difficulty_csf_key(2), "TXT_HARD");
}

#[test]
fn blit_text_writes_colored_pixel() {
    let fnt = tiny_fnt();
    let mut dst = RgbaImage::from_raw(4, 4, vec![0u8; 64]).unwrap();
    blit_text_colored(&mut dst, &fnt, "A", 0, 0, [255, 214, 0, 255]);
    assert_eq!(&dst.as_raw()[0..4], &[255, 214, 0, 255]);
}

#[test]
fn command_button_tip_keys_cover_retail_slots() {
    assert_eq!(command_button_csf_tooltip(0), Some("TIP:TEAM01"));
    assert_eq!(command_button_csf_tooltip(2), Some("TIP:TYPESELECT"));
    assert_eq!(command_button_csf_tooltip(4), Some("TIP:GUARD"));
    assert_eq!(command_button_csf_tooltip(5), Some("TIP:PLANNINGMODE"));
    assert_eq!(command_button_csf_tooltip(12), None);
}

#[test]
fn country_lobby_display_falls_back_to_house_id() {
    assert_eq!(country_lobby_display_name(None, "Africans", "Name:Africans"), "Africans");
    assert_eq!(country_lobby_display_name(None, "Alliance", ""), "Alliance");
}

#[test]
fn country_lobby_display_resolves_uiname_csf() {
    let csf = csf_with_entries(&[
        ("Name:Africans", "利比亚"),
        ("Name:Alliance", "韩国"),
        ("NAME:AMERICANS", "美国"),
    ]);
    assert_eq!(country_lobby_display_name(Some(&csf), "Africans", "Name:Africans"), "利比亚");
    assert_eq!(country_lobby_display_name(Some(&csf), "Alliance", "Name:Alliance"), "韩国");
    // UIName 缺失时回退 `NAME:{ID}`。
    assert_eq!(country_lobby_display_name(Some(&csf), "Americans", ""), "美国");
}
