//! 壳层菜单文字：FNT 量宽 + 着色绘制；CSF 标签查找。

use ra_assets::{CsfFile, FntFile};
use ra_renderer::RgbaImage;

/// 主菜单入口 → CSF 标签。
pub fn main_menu_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "single_player" => Some("GUI:SinglePlayer"),
        "ww_online" => Some("GUI:WWOnline"),
        "network" => Some("GUI:Network"),
        "movies" => Some("GUI:MoviesAndCredits"),
        "options" => Some("GUI:Options"),
        "exit" => Some("GUI:ExitGame"),
        _ => None,
    }
}

/// 主菜单悬停提示 → CSF（底栏 `STT:*`）。
pub fn main_menu_csf_tooltip(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "single_player" => Some("STT:MainButtonSinglePlayer"),
        "ww_online" => Some("STT:MainButtonWWOnline"),
        "network" => Some("STT:MainButtonNetwork"),
        "movies" => Some("STT:MainButtonMovies"),
        "options" => Some("STT:MainButtonOptions"),
        "exit" => Some("STT:MainButtonExit"),
        _ => None,
    }
}

/// 单人页入口 → CSF 标签。
pub fn single_player_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "campaign" => Some("GUI:NewCampaign"),
        "load" => Some("GUI:LoadSavedGame"),
        "skirmish" => Some("GUI:Skirmish"),
        "back" => Some("GUI:MainMenu"),
        _ => None,
    }
}

/// 单人页悬停提示 → CSF（底栏 `STT:*`）。
pub fn single_player_csf_tooltip(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "campaign" => Some("STT:SingleButtonNewCampaign"),
        "load" => Some("STT:SingleButtonLoadSavedGame"),
        "skirmish" => Some("STT:SingleButtonSkirmish"),
        "back" => Some("STT:SingleButtonBack"),
        _ => None,
    }
}

/// 单人页顶栏标题 CSF（对话框 `0x100` 控件 `0x694`）。
pub fn single_player_title_csf_key() -> &'static str {
    "GUI:SinglePlayerMenu"
}

/// 战役页入口 → CSF 标签。
pub fn campaign_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "load" => Some("GUI:Load"),
        "back" => Some("GUI:Back"),
        _ => None,
    }
}

/// 战役页悬停提示 → CSF。
pub fn campaign_csf_tooltip(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "allied" => Some("STT:CampaignAnimAllied"),
        "tutorial" => Some("STT:CampaignAnimTutorial"),
        "soviet" => Some("STT:CampaignAnimSoviet"),
        "load" => Some("STT:CampaignButtonLoad"),
        "back" => Some("STT:CampaignButtonBack"),
        "difficulty" => Some("STT:CampaignSliderDifficulty"),
        _ => None,
    }
}

/// 战役页顶栏标题 CSF（对话框 `0x94` 控件 `0x694`）。
pub fn campaign_title_csf_key() -> &'static str {
    "GUI:CampaignMenu"
}

/// 战役难度档位 → CSF。
pub fn campaign_difficulty_csf_key(level: u8) -> &'static str {
    match level {
        0 => "TXT_EASY",
        2 => "TXT_HARD",
        _ => "TXT_NORMAL",
    }
}

/// 遭遇战大厅入口 → CSF 标签。
pub fn skirmish_lobby_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "start" => Some("GUI:StartGame"),
        "choose_map" => Some("GUI:ChooseMap"),
        "back" => Some("GUI:Back"),
        _ => None,
    }
}

/// 遭遇战大厅悬停 → 底栏 `STT:*`。
pub fn skirmish_lobby_csf_tooltip(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "start" => Some("STT:SkirmishButtonStartGame"),
        "choose_map" => Some("STT:SkirmishButtonChooseMap"),
        "back" => Some("STT:SkirmishButtonBack"),
        "short_game" => Some("STT:SkirmishCBoxShortGame"),
        "mcv_repacks" => Some("STT:SkirmishCBoxRedeploys"),
        "crates" => Some("STT:SkirmishCBoxCrates"),
        "superweapons" => Some("STT:SkirmishCBoxSWAllowed"),
        "build_off_ally" => Some("STT:SkirmishCBoxBuildOffAlly"),
        "speed" => Some("STT:SkirmishSliderSpeed"),
        "credits" => Some("STT:SkirmishSliderCredits"),
        "units" => Some("STT:SkirmishSliderUnit"),
        "country" => Some("STT:SkirmishComboCountry"),
        "color" => Some("STT:SkirmishComboColor"),
        "ai" => Some("STT:SkirmishComboAIPlayer"),
        "player_name" => Some("STT:SkirmishEditPlayer"),
        "flag" => Some("STT:SkirmishPictureFlag"),
        "map_preview" => Some("STT:SkirmishMapThumbnail"),
        "game_type" => Some("STT:SkirmishLabelGameType"),
        "map_label" => Some("STT:SkirmishLabelScenario"),
        _ => None,
    }
}

/// 选图页入口 → CSF 标签。
pub fn choose_map_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "use_map" => Some("GUI:UseMap"),
        "create_random" => Some("GUI:CreateRandomMap"),
        "cancel" => Some("GUI:Cancel"),
        _ => None,
    }
}

/// 选图页标题 CSF。
pub fn choose_map_title_csf_key() -> &'static str {
    "GUI:ChooseMap"
}

/// 选图页静态文案 CSF。
pub fn choose_map_static_csf_key(kind: &str) -> Option<&'static str> {
    match kind {
        "select_engagement" => Some("GUI:SelectEngagement"),
        "game_type" => Some("GUI:GameType"),
        "game_map" => Some("GUI:GameMap"),
        "battle" => Some("GUI:Battle"),
        _ => None,
    }
}

/// 遭遇战右栏标题 CSF。
pub fn skirmish_title_csf_key() -> &'static str {
    "GUI:SkirmishGame"
}

/// 遭遇战左栏 / 右栏静态文案 CSF。
pub fn skirmish_lobby_static_csf_key(kind: &str) -> Option<&'static str> {
    match kind {
        "players" => Some("GUI:Players"),
        "side" => Some("GUI:Side"),
        "color" => Some("GUI:Color"),
        "battle" => Some("GUI:Battle"),
        "short_game" => Some("GUI:ShortGame"),
        "mcv_repacks" => Some("GUI:MCVRepacks"),
        "crates" => Some("GUI:CratesAppear"),
        "superweapons" => Some("GUI:SuperWeaponsAllowed"),
        "build_off_ally" => Some("GUI:BuildOffAlly"),
        "game_speed" => Some("GUI:GameSpeed"),
        "credits" => Some("GUI:Credits"),
        "unit_count" => Some("GUI:UnitCount"),
        "ai_hard" => Some("GUI:AIHard"),
        "none" => Some("GUI:None"),
        _ => None,
    }
}

/// 选项页入口 → CSF 标签。
pub fn options_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "accept" => Some("GUI:Ok"),
        "cancel" => Some("GUI:Cancel"),
        "main_menu" => Some("GUI:MainMenu"),
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

/// 仅当 CSF 有非空文案时返回；缺省不回退。
pub fn resolve_csf_text(csf: Option<&CsfFile>, key: &str) -> Option<String> {
    let text = csf?.get(key)?;
    let cleaned = sanitize_csf_display(text);
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

/// 去掉 CSF 脏字节（模组截断留下的 U+FFFD / NUL）。
pub fn sanitize_csf_display(text: &str) -> String {
    text.chars().filter(|c| *c != '\0' && *c != '\u{FFFD}').collect::<String>().trim().to_string()
}

/// 装载页国家名 CSF：`NAME:{side}`（如 `NAME:AMERICANS`）。
pub fn load_screen_name_csf_key(side: &str) -> String {
    format!("NAME:{}", side.to_ascii_uppercase())
}

/// 装载页国家介绍 CSF：`LOADBRIEF:{suffix}`。
pub fn load_screen_brief_csf_key(side: &str) -> String {
    format!("LOADBRIEF:{}", crate::skirmish_setup::load_screen_brief_suffix(side))
}

/// 装载页特色兵种名 CSF（如美国 `NAME:PARA`＝伞兵）。
pub fn load_screen_special_unit_csf_key(side: &str) -> &'static str {
    match crate::skirmish_setup::load_screen_brief_suffix(side) {
        "USA" => "NAME:PARA",
        "FRENCH" => "NAME:GTGCAN",
        "GERMANS" => "NAME:TNKD",
        "BRITISH" => "NAME:SNIPE",
        "RUSSIA" => "NAME:TTNK",
        "KOREA" => "NAME:BEAGLE",
        "CUBA" => "NAME:TERROR",
        "IRAQ" => "NAME:DESO",
        "LYBIA" => "NAME:DTRUCK",
        _ => "NAME:PARA",
    }
}

/// 装载页「载入中」CSF。
pub fn load_screen_loading_csf_key() -> &'static str {
    "GUI:LOADINGEX"
}

/// 启用按钮常用黄字（近似原版壳层）。
pub const MENU_TEXT_ENABLED: [u8; 4] = [255, 214, 0, 255];
/// 禁用按钮暗红字（原版壳层约 `#9F0000`，非灰字）。
pub const MENU_TEXT_DISABLED: [u8; 4] = [0x9F, 0x00, 0x00, 255];
/// 选项分区标题。
pub const MENU_TEXT_SECTION: [u8; 4] = [255, 214, 0, 255];
/// 选项控件说明（偏红）。
pub const MENU_TEXT_ACCENT: [u8; 4] = [220, 48, 48, 255];
/// 装载页正文（盟约青蓝，对齐原版国家艺术上的字色）。
pub const LOAD_SCREEN_TEXT: [u8; 4] = [96, 200, 255, 255];
/// 装载页标题字（特色名 / 国名）。
pub const LOAD_SCREEN_TEXT_TITLE: [u8; 4] = [120, 220, 255, 255];

/// 选项左栏文案键。
pub fn options_dialog_csf_key(kind: &str) -> Option<&'static str> {
    match kind {
        "title" => Some("GUI:OptionsMenu"),
        "display" => Some("GUI:DisplayOptions"),
        "game" => Some("GUI:GameOptions"),
        "ui" => Some("GUI:UIOptions"),
        "audio" => Some("GUI:AudioOptions"),
        "detail" => Some("GUI:VisualDetails"),
        "resolution" => Some("GUI:SetResolution"),
        "difficulty" => Some("GUI:Difficulty"),
        "tooltips" => Some("GUI:Tooltips"),
        "target_lines" | "scanlines" => Some("GUI:TargetLines"),
        "show_hidden" | "damage" => Some("GUI:ShowHidden"),
        "scroll" => Some("GUI:ScrollRate"),
        "music" => Some("GUI:MusicVolume"),
        "sound" => Some("GUI:SoundVolume"),
        "voice" => Some("GUI:VoiceVolume"),
        // 质感区无原版 CSF 键，绘制侧用英文 fallback。
        "present" | "present_16bit" => None,
        "high" => Some("TXT_HIGH"),
        "hard" => Some("TXT_HARD"),
        "fastest" => Some("TXT_FASTEST"),
        _ => None,
    }
}

/// 退出确认钮 → CSF。
pub fn exit_confirm_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "ok" => Some("GUI:Ok"),
        "cancel" => Some("GUI:Cancel"),
        _ => None,
    }
}

/// 对局暂停菜单钮 → CSF。
pub fn pause_menu_csf_label(entry_id: &str) -> Option<&'static str> {
    match entry_id {
        "options" => Some("GUI:Options"),
        "load" => Some("GUI:LoadMission"),
        "save" => Some("GUI:SaveMission"),
        "restart" => Some("GUI:AbortRestart"),
        "abort" => Some("GUI:AbortMission"),
        "resume" => Some("GUI:ResumeMission"),
        _ => None,
    }
}

/// 退出确认提示 CSF 键。
pub fn exit_confirm_prompt_csf_key() -> &'static str {
    "GUI:ExitAreYouSure"
}

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

/// 在矩形内左上锚点绘制一行（超出宽度则截断绘制，不居中）。
///
/// 对齐 MessageBox 正文静态 `0x5B0`：左/上锚点，在控件矩形内裁剪。
pub fn blit_caption_top_left_clipped(
    dst: &mut RgbaImage,
    fnt: &FntFile,
    text: &str,
    cell_x: i32,
    cell_y: i32,
    cell_w: i32,
    cell_h: i32,
    rgba: [u8; 4],
) {
    if cell_w <= 0 || cell_h <= 0 {
        return;
    }
    let th = fnt.bitmap_rows as i32;
    if th > cell_h {
        return;
    }
    // 逐字绘制并在右边界截断。
    let mut pen_x = cell_x;
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
        let advance = if first { 0 } else { 1 } + glyph.width as i32;
        if pen_x + advance > cell_x + cell_w {
            break;
        }
        if !first {
            pen_x += 1;
        }
        first = false;
        blit_text_colored(dst, fnt, &ch.to_string(), pen_x, cell_y, rgba);
        pen_x += glyph.width as i32;
    }
}

/// 在矩形内左上锚点逐行绘制（按字宽折行；显式 `\n` 换行）。
pub fn blit_caption_wrapped(
    dst: &mut RgbaImage,
    fnt: &FntFile,
    text: &str,
    cell_x: i32,
    cell_y: i32,
    cell_w: i32,
    cell_h: i32,
    rgba: [u8; 4],
) {
    if cell_w <= 0 || cell_h <= 0 || text.is_empty() {
        return;
    }
    let line_h = (fnt.bitmap_rows as i32).max(1) + 2;
    let mut y = cell_y;
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for ch in paragraph.chars() {
            let candidate = {
                let mut t = line.clone();
                t.push(ch);
                t
            };
            if fnt.text_width(&candidate) as i32 <= cell_w || line.is_empty() {
                line.push(ch);
                continue;
            }
            if y + line_h > cell_y + cell_h {
                return;
            }
            blit_caption_top_left_clipped(dst, fnt, &line, cell_x, y, cell_w, line_h, rgba);
            y += line_h;
            line.clear();
            line.push(ch);
        }
        if !line.is_empty() {
            if y + line_h > cell_y + cell_h {
                return;
            }
            blit_caption_top_left_clipped(dst, fnt, &line, cell_x, y, cell_w, line_h, rgba);
            y += line_h;
        }
        else {
            // 空段仍推进一行，保留段落间距。
            y += line_h;
        }
        if y >= cell_y + cell_h {
            return;
        }
    }
}

/// 在按钮格内水平居中绘制一行（垂直居中；按下态由调用方先 inset 格）。
pub fn blit_caption_in_cell(dst: &mut RgbaImage, fnt: &FntFile, text: &str, cell_x: i32, cell_y: i32, cell_w: i32, cell_h: i32, rgba: [u8; 4]) {
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
