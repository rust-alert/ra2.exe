//! 战役关卡装载外观（`mission.ini` / `missionmd.ini`）。
//!
//! 节名即 scenario 文件名（如 `ALL01T.MAP`）；字段驱动装载页背景与简报文案，
//! 与遭遇战国家 `ls*` / `LOADBRIEF` 路径分离。

use ra_types::{MapFileName, RaResult, UiName};
use serde::Deserialize;

use crate::IniDocument;

/// 一关战役装载外观（`mission.ini` 单节）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionPresentation {
    /// 节名 / scenario 文件名（保留盘上大小写；查找时大小写不敏感）。
    pub scenario: MapFileName,
    /// 任务简报 CSF（`Briefing=`，如 `BRIEF:ALL01`）；可空。
    pub briefing_csf: UiName,
    /// 关卡显示名 CSF（`UIName=`，如 `NAME:ALL01`）；可空。
    pub ui_name_csf: UiName,
    /// 装载底栏状态 CSF（`LSLoadMessage=`，如 `LOADMSG:ALL01`）；可空。
    pub load_message_csf: UiName,
    /// 装载简报 CSF（`LSLoadBriefing=`，如 `LOADBRIEF:ALL01`）；可空。
    pub load_briefing_csf: UiName,
    /// 640 宽简报原点 X（`LS640BriefLocX`）。
    pub brief_loc_x_640: i32,
    /// 640 宽简报原点 Y（`LS640BriefLocY`）。
    pub brief_loc_y_640: i32,
    /// 800 宽简报原点 X（`LS800BriefLocX`）。
    pub brief_loc_x_800: i32,
    /// 800 宽简报原点 Y（`LS800BriefLocY`）。
    pub brief_loc_y_800: i32,
    /// 640 宽装载背景 SHP（`LS640BkgdName`，如 `LS640A01.SHP`）；可空。
    pub background_shp_640: String,
    /// 800 宽装载背景 SHP（`LS800BkgdName`，如 `LS800A01.SHP`）；可空。
    pub background_shp_800: String,
    /// 800 宽装载背景调色板（`LS800BkgdPal`，如 `LS800A01.PAL`）；RA2 零售常空，由上层回退 `ldscrna`/`ldscrns`。
    pub background_pal_800: String,
}

#[derive(Debug, Default, Deserialize)]
struct MissionSectionFields {
    #[serde(rename = "Briefing", default)]
    briefing: UiName,
    #[serde(rename = "UIName", default)]
    ui_name: UiName,
    #[serde(rename = "LSLoadMessage", default)]
    ls_load_message: UiName,
    #[serde(rename = "LSLoadBriefing", default)]
    ls_load_briefing: UiName,
    #[serde(rename = "LS640BriefLocX")]
    ls640_brief_loc_x: Option<i32>,
    #[serde(rename = "LS640BriefLocY")]
    ls640_brief_loc_y: Option<i32>,
    #[serde(rename = "LS800BriefLocX")]
    ls800_brief_loc_x: Option<i32>,
    #[serde(rename = "LS800BriefLocY")]
    ls800_brief_loc_y: Option<i32>,
    #[serde(rename = "LS640BkgdName", default)]
    ls640_bkgd_name: String,
    #[serde(rename = "LS800BkgdName", default)]
    ls800_bkgd_name: String,
    #[serde(rename = "LS800BkgdPal", default)]
    ls800_bkgd_pal: String,
}

/// 从 `mission.ini` 字节解析全部关卡装载外观。
///
/// 收录带节名的条目；无背景与无文案的空节仍保留（查找时可见，由上层决定回退）。
pub fn parse_mission_presentations(bytes: &[u8]) -> RaResult<Vec<MissionPresentation>> {
    let doc = IniDocument::parse(bytes)?;
    let mut out = Vec::new();
    for section in &doc.sections {
        let name = section.name_raw.trim();
        if name.is_empty() {
            continue;
        }
        // 跳过非 scenario 形节名（无扩展名的全局节极少见；宁漏勿误收）。
        if !name.contains('.') {
            continue;
        }
        let fields = section.deserialize::<MissionSectionFields>().unwrap_or_default();
        out.push(MissionPresentation {
            scenario: MapFileName::parse(name),
            briefing_csf: fields.briefing,
            ui_name_csf: fields.ui_name,
            load_message_csf: fields.ls_load_message,
            load_briefing_csf: fields.ls_load_briefing,
            brief_loc_x_640: fields.ls640_brief_loc_x.unwrap_or(0),
            brief_loc_y_640: fields.ls640_brief_loc_y.unwrap_or(0),
            brief_loc_x_800: fields.ls800_brief_loc_x.unwrap_or(0),
            brief_loc_y_800: fields.ls800_brief_loc_y.unwrap_or(0),
            background_shp_640: fields.ls640_bkgd_name.trim().to_string(),
            background_shp_800: fields.ls800_bkgd_name.trim().to_string(),
            background_pal_800: fields.ls800_bkgd_pal.trim().to_string(),
        });
    }
    Ok(out)
}

/// 按 scenario 文件名（大小写不敏感）查找装载外观。
pub fn find_mission_presentation<'a>(missions: &'a [MissionPresentation], scenario: &str) -> Option<&'a MissionPresentation> {
    let needle = scenario.trim();
    if needle.is_empty() {
        return None;
    }
    missions.iter().find(|m| m.scenario.as_str().eq_ignore_ascii_case(needle))
}

impl MissionPresentation {
    /// 按视口宽选择装载背景 SHP；空串表示未配置。
    pub fn background_shp_for_viewport(&self, viewport_w: u32) -> Option<&str> {
        let name = if viewport_w < 800 {
            self.background_shp_640.as_str()
        } else {
            self.background_shp_800.as_str()
        };
        let trimmed = name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    /// 装载背景调色板：零售仅见 `LS800BkgdPal`（640 视口仍用同一盘）；空串表示未配置。
    pub fn background_pal_for_viewport(&self, _viewport_w: u32) -> Option<&str> {
        let trimmed = self.background_pal_800.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    /// 按视口宽选择简报原点。
    pub fn brief_loc_for_viewport(&self, viewport_w: u32) -> (i32, i32) {
        if viewport_w < 800 {
            (self.brief_loc_x_640, self.brief_loc_y_640)
        } else {
            (self.brief_loc_x_800, self.brief_loc_y_800)
        }
    }
}
