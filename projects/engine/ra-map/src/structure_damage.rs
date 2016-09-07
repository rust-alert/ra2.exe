//! 建筑受损阈值与主体帧 / 燃烧叠画选用。

use ra_assets::{IniDocument, from_row};
use serde::Deserialize;

/// 规则里的建筑受损阈值与火焰类型名。
#[derive(Debug, Clone, PartialEq)]
#[doc(hidden)]
pub struct StructureDamageRules {
    /// `ConditionYellow`（0..=1），默认 0.5。
    pub yellow: f32,
    /// `ConditionRed`（0..=1），默认 0.25。
    pub red: f32,
    /// `DamageFireTypes`（如 `FIRE01,FIRE02,FIRE03`）。
    pub fire_types: Vec<String>,
}

impl Default for StructureDamageRules {
    fn default() -> Self {
        Self { yellow: 0.5, red: 0.25, fire_types: Vec::new() }
    }
}

/// `[AudioVisual]` 受损相关键。
#[derive(Debug, Default, Deserialize)]
struct AudioVisualDamageFields {
    #[serde(rename = "ConditionYellow")]
    condition_yellow: Option<String>,
    #[serde(rename = "ConditionRed")]
    condition_red: Option<String>,
    #[serde(rename = "DamageFireTypes")]
    damage_fire_types: Option<String>,
    #[serde(rename = "DamageFireNames")]
    damage_fire_names: Option<String>,
}

/// `[General]` 火焰类型键。
#[derive(Debug, Default, Deserialize)]
struct GeneralDamageFireFields {
    #[serde(rename = "DamageFireTypes")]
    damage_fire_types: Option<String>,
    #[serde(rename = "DamageFireNames")]
    damage_fire_names: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FireOffsetRow {
    x: i32,
    y: i32,
}

impl StructureDamageRules {
    /// 从 rules 文档读取 `[AudioVisual]` 阈值与火焰类型。
    pub fn from_rules_doc(doc: &IniDocument) -> Self {
        let mut out = Self::default();
        let av = doc
            .section("AudioVisual")
            .and_then(|s| s.deserialize::<AudioVisualDamageFields>().ok())
            .unwrap_or_default();
        if let Some(raw) = av.condition_yellow.as_deref() {
            if let Some(v) = parse_condition_percent(raw) {
                out.yellow = v;
            }
        }
        if let Some(raw) = av.condition_red.as_deref() {
            if let Some(v) = parse_condition_percent(raw) {
                out.red = v;
            }
        }
        let general = doc
            .section("General")
            .and_then(|s| s.deserialize::<GeneralDamageFireFields>().ok())
            .unwrap_or_default();
        // 零售写在 `[General]`；个别模组可能挂在 `[AudioVisual]`。
        let fire_raw = general
            .damage_fire_types
            .or(general.damage_fire_names)
            .or(av.damage_fire_types)
            .or(av.damage_fire_names);
        if let Some(raw) = fire_raw {
            out.fire_types = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_uppercase())
                .collect();
        }
        out
    }

    /// 地图 `health`（0..=256）是否已进入黄血带及以下（起火 / 受损活动层门槛）。
    pub fn is_yellow(&self, health_256: u16) -> bool {
        health_ratio_256(health_256) <= self.yellow
    }

    /// 是否已进入红血（血条与平民房主体受损帧门槛）。
    pub fn is_red(&self, health_256: u16) -> bool {
        health_ratio_256(health_256) <= self.red
    }
}

/// 解析 `50%` / `0.5` / `50` 为 0..=1。
pub fn parse_condition_percent(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if let Some(num) = s.strip_suffix('%') {
        let v: f32 = num.trim().parse().ok()?;
        return Some((v / 100.0).clamp(0.0, 1.0));
    }
    let v: f32 = s.parse().ok()?;
    if v > 1.0 { Some((v / 100.0).clamp(0.0, 1.0)) } else { Some(v.clamp(0.0, 1.0)) }
}

/// 地图放置血量比例（256=满）。
pub fn health_ratio_256(health_256: u16) -> f32 {
    f32::from(health_256.min(256)) / 256.0
}

#[derive(Debug, Default, Deserialize)]
struct TechLevelFields {
    #[serde(rename = "TechLevel")]
    tech_level: Option<i32>,
}

/// 从 rules 类型节读 `TechLevel`；缺省按平民建筑 `-1`。
pub fn structure_tech_level(rules: Option<&IniDocument>, type_id: &str) -> i32 {
    rules
        .and_then(|d| d.section(type_id))
        .and_then(|s| s.deserialize::<TechLevelFields>().ok())
        .and_then(|f| f.tech_level)
        .unwrap_or(-1)
}

/// 主体受损帧（无人占领、非驻军折叠）。
///
/// - 绿：帧 0
/// - 黄：仅 `TechLevel > 0` 的军建用帧 1；平民（`TechLevel <= 0`）仍帧 0
/// - 红：一律帧 1（需至少 2 帧主体）
pub fn damaged_body_frame(health_256: u16, yellow: f32, red: f32, tech_level: i32, body_frames: usize) -> u16 {
    if body_frames < 2 {
        return 0;
    }
    let ratio = health_ratio_256(health_256);
    let red_tier = ratio <= red;
    let yellow_tier = tech_level > 0 && ratio <= yellow;
    if red_tier || yellow_tier { 1 } else { 0 }
}

/// 解析 `DamageFireOffsetN=x,y`。
pub fn parse_damage_fire_offset(raw: &str) -> Option<(i32, i32)> {
    let row: FireOffsetRow = from_row(raw).ok()?;
    Some((row.x, row.y))
}
