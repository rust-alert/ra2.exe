//! 遭遇战大厅配置：对话框 `0x102` 选项 + 装载请求。
//!
//! 控件几何一律来自 `solve_skirmish_lobby` snapshot；本模块只持状态与命中。

/// 大厅可选难度标签（写入装载请求；引擎按 Easy/Normal/Hard 调节 AI 节奏）。
use super::chrome::UiFactionChrome;

/// 装载图回退调色板：共享 `mpls.pal`（非国家猜测）。
pub const LOAD_SCREEN_FALLBACK_PAL: &str = "mpls.pal";

/// 进度条 SHP（帧 0；按进度横向裁剪填充）。
pub const LOAD_SCREEN_PROGRESS_SHP: &str = "progbarm.shp";

/// 装载背景：仅使用显式文件名；空则无背景可画（不猜国名后缀）。
///
/// 若显式名为 `ls800…` 且视口较窄，尝试把前缀换成 `ls640`（同文件族缩放，非国名启发式）。
pub fn load_screen_background_shp_resolved(viewport_w: u32, rules_shp: Option<&str>) -> Option<String> {
    let name = rules_shp.map(str::trim).filter(|s| !s.is_empty())?;
    if viewport_w < 800 {
        if let Some(rest) = name.strip_prefix("ls800") {
            return Some(format!("ls640{rest}"));
        }
    }
    Some(name.to_string())
}

/// 装载调色板：仅显式名；不可读则共享 [`LOAD_SCREEN_FALLBACK_PAL`]；皆无则 `None`。
pub fn load_screen_palette_resolved(rules_pal: Option<&str>, pal_readable: impl Fn(&str) -> bool) -> Option<String> {
    if let Some(p) = rules_pal.map(str::trim).filter(|s| !s.is_empty()) {
        if pal_readable(p) {
            return Some(p.to_string());
        }
    }
    if pal_readable(LOAD_SCREEN_FALLBACK_PAL) {
        return Some(LOAD_SCREEN_FALLBACK_PAL.to_string());
    }
    None
}

/// 装载介绍 CSF 键：显式 `LoadScreenText.Brief` 优先；否则 `LOADBRIEF:{country_id}`（不映射后缀表）。
pub fn load_screen_brief_csf_key(country_id: &str, rules_brief: Option<&str>) -> String {
    if let Some(b) = rules_brief.map(str::trim).filter(|s| !s.is_empty()) {
        if b.contains(':') {
            return b.to_string();
        }
        return format!("LOADBRIEF:{b}");
    }
    format!("LOADBRIEF:{country_id}")
}

/// 结算战报图候选（消费已解析 chrome）。
pub fn score_screen_background_candidates(chrome: &UiFactionChrome) -> Vec<String> {
    chrome.score_background_candidates()
}

/// 结算调色板候选（消费已解析 chrome）。
pub fn score_screen_palette_candidates(chrome: &UiFactionChrome) -> Vec<String> {
    chrome.score_palette_candidates()
}

/// 结算战报图首选。
pub fn score_screen_background_shp(chrome: &UiFactionChrome) -> String {
    chrome.score_background_shp()
}

/// 结算调色板首选。
pub fn score_screen_palette(chrome: &UiFactionChrome) -> String {
    chrome.score_palette()
}
