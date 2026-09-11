//! 从 rules 读取装载期全局键（`[General]` / 对话设置 / 语音间隔等）。

use serde::Deserialize;

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView};
use ra_types::TechnoName;

/// 装载期全局字段（缺省由 adaptor 填产品默认，不在此冒充「未写」）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RulesGlobals {
    /// `[MultiplayerDialogSettings] TechLevel`。
    pub multiplayer_tech_level: Option<i32>,
    /// `[General] RepairPercent`。
    pub repair_percent: Option<i32>,
    /// `[General] RepairStep`。
    pub repair_step: Option<i32>,
    /// `[General] RepairRate`（分钟）。
    pub repair_rate_minutes: Option<f64>,
    /// `[AudioVisual] SpeakDelay`，否则 `[General] SpeakDelay`（分钟）。
    pub speak_delay_minutes: Option<f64>,
    /// `[General] PrerequisitePower`（装载期一次解码为大写类型名）。
    pub prerequisite_power: Vec<TechnoName>,
    /// `[General] PrerequisiteFactory`。
    pub prerequisite_factory: Vec<TechnoName>,
    /// `[General] PrerequisiteBarracks`。
    pub prerequisite_barracks: Vec<TechnoName>,
    /// `[General] PrerequisiteRadar`。
    pub prerequisite_radar: Vec<TechnoName>,
    /// `[General] PrerequisiteTech`。
    pub prerequisite_tech: Vec<TechnoName>,
    /// `[General] PrerequisiteProc`。
    pub prerequisite_proc: Vec<TechnoName>,
    /// `[General] PrerequisiteProcAlternate`。
    pub prerequisite_proc_alternate: Vec<TechnoName>,
}

impl RulesGlobals {
    /// 从单份 rules 文档解码（内部走层叠视图，仅一层）。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(rules);
        Self::from_layered(LayeredIniView::new(docs, &policy))
    }

    /// 从层叠 rules 视图一次解码全局节字段。
    pub fn from_layered(view: LayeredIniView<'_>) -> Self {
        let general = view
            .section("General")
            .and_then(|s| s.deserialize::<GeneralSectionFields>().ok())
            .unwrap_or_default();
        let dialog = view
            .section("MultiplayerDialogSettings")
            .and_then(|s| s.deserialize::<DialogSectionFields>().ok())
            .unwrap_or_default();
        let audio = view
            .section("AudioVisual")
            .and_then(|s| s.deserialize::<AudioVisualSectionFields>().ok())
            .unwrap_or_default();
        let speak_delay_minutes = audio.speak_delay.or(general.speak_delay);
        Self {
            multiplayer_tech_level: dialog.tech_level,
            repair_percent: general.repair_percent,
            repair_step: general.repair_step,
            repair_rate_minutes: general.repair_rate,
            speak_delay_minutes,
            prerequisite_power: filter_techno_names(general.prerequisite_power),
            prerequisite_factory: filter_techno_names(general.prerequisite_factory),
            prerequisite_barracks: filter_techno_names(general.prerequisite_barracks),
            prerequisite_radar: filter_techno_names(general.prerequisite_radar),
            prerequisite_tech: filter_techno_names(general.prerequisite_tech),
            prerequisite_proc: filter_techno_names(general.prerequisite_proc),
            prerequisite_proc_alternate: filter_techno_names(general.prerequisite_proc_alternate),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct GeneralSectionFields {
    #[serde(rename = "RepairPercent")]
    repair_percent: Option<i32>,
    #[serde(rename = "RepairStep")]
    repair_step: Option<i32>,
    #[serde(rename = "RepairRate")]
    repair_rate: Option<f64>,
    #[serde(rename = "SpeakDelay")]
    speak_delay: Option<f64>,
    #[serde(rename = "PrerequisitePower", default)]
    prerequisite_power: Vec<TechnoName>,
    #[serde(rename = "PrerequisiteFactory", default)]
    prerequisite_factory: Vec<TechnoName>,
    #[serde(rename = "PrerequisiteBarracks", default)]
    prerequisite_barracks: Vec<TechnoName>,
    #[serde(rename = "PrerequisiteRadar", default)]
    prerequisite_radar: Vec<TechnoName>,
    #[serde(rename = "PrerequisiteTech", default)]
    prerequisite_tech: Vec<TechnoName>,
    #[serde(rename = "PrerequisiteProc", default)]
    prerequisite_proc: Vec<TechnoName>,
    #[serde(rename = "PrerequisiteProcAlternate", default)]
    prerequisite_proc_alternate: Vec<TechnoName>,
}

#[derive(Debug, Default, Deserialize)]
struct DialogSectionFields {
    #[serde(rename = "TechLevel")]
    tech_level: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct AudioVisualSectionFields {
    #[serde(rename = "SpeakDelay")]
    speak_delay: Option<f64>,
}

fn filter_techno_names(items: Vec<TechnoName>) -> Vec<TechnoName> {
    items.into_iter().filter(|n| !n.is_empty()).collect()
}
