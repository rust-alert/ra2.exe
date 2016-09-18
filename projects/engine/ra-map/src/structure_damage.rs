//! 建筑受损阈值与主体帧 / 燃烧叠画选用。

use std::fmt;

use ra_assets::{IniDocument, from_row};
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

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
    #[serde(rename = "ConditionYellow", default, deserialize_with = "deserialize_optional_condition_percent")]
    condition_yellow: Option<f32>,
    #[serde(rename = "ConditionRed", default, deserialize_with = "deserialize_optional_condition_percent")]
    condition_red: Option<f32>,
    #[serde(rename = "DamageFireTypes", default, deserialize_with = "deserialize_optional_fire_type_list")]
    damage_fire_types: Option<Vec<String>>,
    #[serde(rename = "DamageFireNames", default, deserialize_with = "deserialize_optional_fire_type_list")]
    damage_fire_names: Option<Vec<String>>,
}

/// `[General]` 火焰类型键。
#[derive(Debug, Default, Deserialize)]
struct GeneralDamageFireFields {
    #[serde(rename = "DamageFireTypes", default, deserialize_with = "deserialize_optional_fire_type_list")]
    damage_fire_types: Option<Vec<String>>,
    #[serde(rename = "DamageFireNames", default, deserialize_with = "deserialize_optional_fire_type_list")]
    damage_fire_names: Option<Vec<String>>,
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
        if let Some(v) = av.condition_yellow {
            out.yellow = v;
        }
        if let Some(v) = av.condition_red {
            out.red = v;
        }
        let general = doc
            .section("General")
            .and_then(|s| s.deserialize::<GeneralDamageFireFields>().ok())
            .unwrap_or_default();
        // 零售写在 `[General]`；个别模组可能挂在 `[AudioVisual]`。
        if let Some(types) = general
            .damage_fire_types
            .or(general.damage_fire_names)
            .or(av.damage_fire_types)
            .or(av.damage_fire_names)
        {
            out.fire_types = types;
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

fn deserialize_optional_condition_percent<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(raw) = Option::<String>::deserialize(deserializer)?
    else {
        return Ok(None);
    };
    Ok(parse_condition_percent(&raw))
}

fn deserialize_optional_fire_type_list<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct FireListVisitor;

    impl<'de> Visitor<'de> for FireListVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("comma-separated fire type names")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(parse_fire_type_list(v)))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(parse_fire_type_list(&v)))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(part) = seq.next_element::<String>()? {
                let name = part.trim();
                if !name.is_empty() {
                    out.push(name.to_ascii_uppercase());
                }
            }
            Ok(Some(out))
        }
    }

    deserializer.deserialize_any(FireListVisitor)
}

fn parse_fire_type_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_uppercase())
        .collect()
}

/// 解析 `50%` / `0.5` / `50` 为 0..=1。
pub fn parse_condition_percent(raw: &str) -> Option<f32> {
    let s = raw.trim();
    if let Some(num) = s.strip_suffix('%') {
        let v: f32 = num.trim().parse().ok()?;
        return Some((v / 100.0).clamp(0.0, 1.0));
    }
    let v: f32 = s.parse().ok()?;
    if v > 1.0 {
        Some((v / 100.0).clamp(0.0, 1.0))
    } else {
        Some(v.clamp(0.0, 1.0))
    }
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
