//! 从 rules 读取装载期全局键（`[General]` / 对话设置 / 语音间隔 / `[AI]` / `[IQ]` 等）。

use serde::Deserialize;

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView, deserialize_opt_f64, deserialize_opt_i32};
use ra_types::TechnoName;

/// 装载期全局字段（缺省由 adaptor 填产品默认，不在此冒充「未写」）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RulesGlobals {
    /// `[MultiplayerDialogSettings] TechLevel`。
    pub multiplayer_tech_level: Option<i32>,
    /// `[General] RepairPercent`。
    pub repair_percent: Option<i32>,
    /// `[General] RefundPercent`：出售时相对造价的百分比（缺省 50）。
    pub refund_percent: Option<i32>,
    /// `[General] RepairStep`。
    pub repair_step: Option<i32>,
    /// `[General] RepairRate`（分钟）。
    pub repair_rate_minutes: Option<f64>,
    /// `[AudioVisual] SpeakDelay`，否则 `[General] SpeakDelay`（分钟）。
    pub speak_delay_minutes: Option<f64>,
    /// `[AudioVisual] SavourDelay`（分钟）：胜负已定后继续仿真的收束窗。
    pub savour_delay_minutes: Option<f64>,
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
    /// `[General] BaseUnit`：短局下可替代建筑保活的 MCV 类载具。
    pub base_unit: Vec<TechnoName>,
    /// `[AI] AIBaseSpacing`：AI 建筑之间最少空隙格数。
    pub ai_base_spacing: Option<i32>,
    /// `[General] AINavalYardAdjacency`：AI 船厂相对建造场最大距离（格）。
    pub ai_naval_yard_adjacency: Option<i32>,
    /// `[AI] BuildConst`。
    pub ai_build_const: Vec<TechnoName>,
    /// `[AI] BuildPower`。
    pub ai_build_power: Vec<TechnoName>,
    /// `[AI] PowerSurplus`。
    pub ai_power_surplus: Option<i32>,
    /// `[AI] BuildRefinery`。
    pub ai_build_refinery: Vec<TechnoName>,
    /// `[AI] RefineryRatio`。
    pub ai_refinery_ratio: Option<f64>,
    /// `[AI] RefineryLimit`。
    pub ai_refinery_limit: Option<i32>,
    /// `[AI] BuildBarracks`。
    pub ai_build_barracks: Vec<TechnoName>,
    /// `[AI] BarracksRatio`。
    pub ai_barracks_ratio: Option<f64>,
    /// `[AI] BarracksLimit`。
    pub ai_barracks_limit: Option<i32>,
    /// `[AI] BuildWeapons`。
    pub ai_build_weapons: Vec<TechnoName>,
    /// `[AI] WarRatio`。
    pub ai_war_ratio: Option<f64>,
    /// `[AI] WarLimit`。
    pub ai_war_limit: Option<i32>,
    /// `[AI] BuildRadar`。
    pub ai_build_radar: Vec<TechnoName>,
    /// `[AI] BuildTech`。
    pub ai_build_tech: Vec<TechnoName>,
    /// `[AI] BuildNavalYard`。
    pub ai_build_naval_yard: Vec<TechnoName>,
    /// `[AI] BuildHelipad`。
    pub ai_build_helipad: Vec<TechnoName>,
    /// `[AI] HelipadRatio`。
    pub ai_helipad_ratio: Option<f64>,
    /// `[AI] HelipadLimit`。
    pub ai_helipad_limit: Option<i32>,
    /// `[AI] BuildDefense`。
    pub ai_build_defense: Vec<TechnoName>,
    /// `[AI] DefenseRatio`。
    pub ai_defense_ratio: Option<f64>,
    /// `[AI] DefenseLimit`。
    pub ai_defense_limit: Option<i32>,
    /// `[AI] BuildAA`。
    pub ai_build_aa: Vec<TechnoName>,
    /// `[AI] AARatio`。
    pub ai_aa_ratio: Option<f64>,
    /// `[AI] AALimit`。
    pub ai_aa_limit: Option<i32>,
    /// `[AI] BuildDummy`。
    pub ai_build_dummy: Vec<TechnoName>,
    /// `[AI] BaseSizeAdd`。
    pub ai_base_size_add: Option<i32>,
    /// `[IQ] MaxIQLevels`。
    pub iq_max_levels: Option<i32>,
    /// `[IQ] Production`。
    pub iq_production: Option<i32>,
    /// `[IQ] SuperWeapons`。
    pub iq_super_weapons: Option<i32>,
    /// `[IQ] GuardArea`。
    pub iq_guard_area: Option<i32>,
    /// `[IQ] RepairSell`。
    pub iq_repair_sell: Option<i32>,
    /// `[IQ] AutoCrush`。
    pub iq_auto_crush: Option<i32>,
    /// `[IQ] Scatter`。
    pub iq_scatter: Option<i32>,
    /// `[IQ] ContentScan`。
    pub iq_content_scan: Option<i32>,
    /// `[IQ] Aircraft`。
    pub iq_aircraft: Option<i32>,
    /// `[IQ] Harvester`。
    pub iq_harvester: Option<i32>,
    /// `[IQ] SellBack`。
    pub iq_sell_back: Option<i32>,
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
        let general = view.section("General").and_then(|s| s.deserialize::<GeneralSectionFields>().ok()).unwrap_or_default();
        let dialog = view.section("MultiplayerDialogSettings").and_then(|s| s.deserialize::<DialogSectionFields>().ok()).unwrap_or_default();
        let audio = view.section("AudioVisual").and_then(|s| s.deserialize::<AudioVisualSectionFields>().ok()).unwrap_or_default();
        let ai = view.section("AI").and_then(|s| s.deserialize::<AiSectionFields>().ok()).unwrap_or_default();
        let iq = view.section("IQ").and_then(|s| s.deserialize::<IqSectionFields>().ok()).unwrap_or_default();
        let speak_delay_minutes = audio.speak_delay.or(general.speak_delay);
        Self {
            multiplayer_tech_level: dialog.tech_level,
            repair_percent: general.repair_percent,
            refund_percent: general.refund_percent,
            repair_step: general.repair_step,
            repair_rate_minutes: general.repair_rate,
            speak_delay_minutes,
            savour_delay_minutes: audio.savour_delay,
            prerequisite_power: filter_techno_names(general.prerequisite_power),
            prerequisite_factory: filter_techno_names(general.prerequisite_factory),
            prerequisite_barracks: filter_techno_names(general.prerequisite_barracks),
            prerequisite_radar: filter_techno_names(general.prerequisite_radar),
            prerequisite_tech: filter_techno_names(general.prerequisite_tech),
            prerequisite_proc: filter_techno_names(general.prerequisite_proc),
            prerequisite_proc_alternate: filter_techno_names(general.prerequisite_proc_alternate),
            base_unit: filter_techno_names(general.base_unit),
            ai_base_spacing: ai.ai_base_spacing,
            ai_naval_yard_adjacency: general.ai_naval_yard_adjacency,
            ai_build_const: filter_techno_names(ai.build_const),
            ai_build_power: filter_techno_names(ai.build_power),
            ai_power_surplus: ai.power_surplus,
            ai_build_refinery: filter_techno_names(ai.build_refinery),
            ai_refinery_ratio: ai.refinery_ratio,
            ai_refinery_limit: ai.refinery_limit,
            ai_build_barracks: filter_techno_names(ai.build_barracks),
            ai_barracks_ratio: ai.barracks_ratio,
            ai_barracks_limit: ai.barracks_limit,
            ai_build_weapons: filter_techno_names(ai.build_weapons),
            ai_war_ratio: ai.war_ratio,
            ai_war_limit: ai.war_limit,
            ai_build_radar: filter_techno_names(ai.build_radar),
            ai_build_tech: filter_techno_names(ai.build_tech),
            ai_build_naval_yard: filter_techno_names(ai.build_naval_yard),
            ai_build_helipad: filter_techno_names(ai.build_helipad),
            ai_helipad_ratio: ai.helipad_ratio,
            ai_helipad_limit: ai.helipad_limit,
            ai_build_defense: filter_techno_names(ai.build_defense),
            ai_defense_ratio: ai.defense_ratio,
            ai_defense_limit: ai.defense_limit,
            ai_build_aa: filter_techno_names(ai.build_aa),
            ai_aa_ratio: ai.aa_ratio,
            ai_aa_limit: ai.aa_limit,
            ai_build_dummy: filter_techno_names(ai.build_dummy),
            ai_base_size_add: ai.base_size_add,
            iq_max_levels: iq.max_iq_levels,
            iq_production: iq.production,
            iq_super_weapons: iq.super_weapons,
            iq_guard_area: iq.guard_area,
            iq_repair_sell: iq.repair_sell,
            iq_auto_crush: iq.auto_crush,
            iq_scatter: iq.scatter,
            iq_content_scan: iq.content_scan,
            iq_aircraft: iq.aircraft,
            iq_harvester: iq.harvester,
            iq_sell_back: iq.sell_back,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct GeneralSectionFields {
    /// 非法文本回落 `None`，不拖垮整节其它键。
    #[serde(rename = "RepairPercent", default, deserialize_with = "deserialize_opt_i32")]
    repair_percent: Option<i32>,
    #[serde(rename = "RefundPercent", default, deserialize_with = "deserialize_opt_i32")]
    refund_percent: Option<i32>,
    #[serde(rename = "RepairStep", default, deserialize_with = "deserialize_opt_i32")]
    repair_step: Option<i32>,
    #[serde(rename = "RepairRate", default, deserialize_with = "deserialize_opt_f64")]
    repair_rate: Option<f64>,
    #[serde(rename = "SpeakDelay", default, deserialize_with = "deserialize_opt_f64")]
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
    #[serde(rename = "BaseUnit", default)]
    base_unit: Vec<TechnoName>,
    #[serde(rename = "AINavalYardAdjacency", default, deserialize_with = "deserialize_opt_i32")]
    ai_naval_yard_adjacency: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct DialogSectionFields {
    #[serde(rename = "TechLevel", default, deserialize_with = "deserialize_opt_i32")]
    tech_level: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct AudioVisualSectionFields {
    #[serde(rename = "SpeakDelay", default, deserialize_with = "deserialize_opt_f64")]
    speak_delay: Option<f64>,
    #[serde(rename = "SavourDelay", default, deserialize_with = "deserialize_opt_f64")]
    savour_delay: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct AiSectionFields {
    #[serde(rename = "AIBaseSpacing", default, deserialize_with = "deserialize_opt_i32")]
    ai_base_spacing: Option<i32>,
    #[serde(rename = "BuildConst", default)]
    build_const: Vec<TechnoName>,
    #[serde(rename = "BuildPower", default)]
    build_power: Vec<TechnoName>,
    #[serde(rename = "PowerSurplus", default, deserialize_with = "deserialize_opt_i32")]
    power_surplus: Option<i32>,
    #[serde(rename = "BuildRefinery", default)]
    build_refinery: Vec<TechnoName>,
    #[serde(rename = "RefineryRatio", default, deserialize_with = "deserialize_opt_f64")]
    refinery_ratio: Option<f64>,
    #[serde(rename = "RefineryLimit", default, deserialize_with = "deserialize_opt_i32")]
    refinery_limit: Option<i32>,
    #[serde(rename = "BuildBarracks", default)]
    build_barracks: Vec<TechnoName>,
    #[serde(rename = "BarracksRatio", default, deserialize_with = "deserialize_opt_f64")]
    barracks_ratio: Option<f64>,
    #[serde(rename = "BarracksLimit", default, deserialize_with = "deserialize_opt_i32")]
    barracks_limit: Option<i32>,
    #[serde(rename = "BuildWeapons", default)]
    build_weapons: Vec<TechnoName>,
    #[serde(rename = "WarRatio", default, deserialize_with = "deserialize_opt_f64")]
    war_ratio: Option<f64>,
    #[serde(rename = "WarLimit", default, deserialize_with = "deserialize_opt_i32")]
    war_limit: Option<i32>,
    #[serde(rename = "BuildRadar", default)]
    build_radar: Vec<TechnoName>,
    #[serde(rename = "BuildTech", default)]
    build_tech: Vec<TechnoName>,
    #[serde(rename = "BuildNavalYard", default)]
    build_naval_yard: Vec<TechnoName>,
    #[serde(rename = "BuildHelipad", default)]
    build_helipad: Vec<TechnoName>,
    #[serde(rename = "HelipadRatio", default, deserialize_with = "deserialize_opt_f64")]
    helipad_ratio: Option<f64>,
    #[serde(rename = "HelipadLimit", default, deserialize_with = "deserialize_opt_i32")]
    helipad_limit: Option<i32>,
    #[serde(rename = "BuildDefense", default)]
    build_defense: Vec<TechnoName>,
    #[serde(rename = "DefenseRatio", default, deserialize_with = "deserialize_opt_f64")]
    defense_ratio: Option<f64>,
    #[serde(rename = "DefenseLimit", default, deserialize_with = "deserialize_opt_i32")]
    defense_limit: Option<i32>,
    #[serde(rename = "BuildAA", default)]
    build_aa: Vec<TechnoName>,
    #[serde(rename = "AARatio", default, deserialize_with = "deserialize_opt_f64")]
    aa_ratio: Option<f64>,
    #[serde(rename = "AALimit", default, deserialize_with = "deserialize_opt_i32")]
    aa_limit: Option<i32>,
    #[serde(rename = "BuildDummy", default)]
    build_dummy: Vec<TechnoName>,
    #[serde(rename = "BaseSizeAdd", default, deserialize_with = "deserialize_opt_i32")]
    base_size_add: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct IqSectionFields {
    #[serde(rename = "MaxIQLevels", default, deserialize_with = "deserialize_opt_i32")]
    max_iq_levels: Option<i32>,
    #[serde(rename = "SuperWeapons", default, deserialize_with = "deserialize_opt_i32")]
    super_weapons: Option<i32>,
    #[serde(rename = "Production", default, deserialize_with = "deserialize_opt_i32")]
    production: Option<i32>,
    #[serde(rename = "GuardArea", default, deserialize_with = "deserialize_opt_i32")]
    guard_area: Option<i32>,
    #[serde(rename = "RepairSell", default, deserialize_with = "deserialize_opt_i32")]
    repair_sell: Option<i32>,
    #[serde(rename = "AutoCrush", default, deserialize_with = "deserialize_opt_i32")]
    auto_crush: Option<i32>,
    #[serde(rename = "Scatter", default, deserialize_with = "deserialize_opt_i32")]
    scatter: Option<i32>,
    #[serde(rename = "ContentScan", default, deserialize_with = "deserialize_opt_i32")]
    content_scan: Option<i32>,
    #[serde(rename = "Aircraft", default, deserialize_with = "deserialize_opt_i32")]
    aircraft: Option<i32>,
    #[serde(rename = "Harvester", default, deserialize_with = "deserialize_opt_i32")]
    harvester: Option<i32>,
    #[serde(rename = "SellBack", default, deserialize_with = "deserialize_opt_i32")]
    sell_back: Option<i32>,
}

fn filter_techno_names(items: Vec<TechnoName>) -> Vec<TechnoName> {
    items.into_iter().filter(|n| !n.is_empty()).collect()
}
