//! 建筑受损阈值与主体帧 / 燃烧叠画选用。

use ra_assets::IniDocument;

/// 规则里的建筑受损阈值与火焰类型名。
#[derive(Debug, Clone, PartialEq)]
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
        Self {
            yellow: 0.5,
            red: 0.25,
            fire_types: Vec::new(),
        }
    }
}

impl StructureDamageRules {
    /// 从 rules 文档读取 `[AudioVisual]` 阈值与火焰类型。
    pub fn from_rules_doc(doc: &IniDocument) -> Self {
        let mut out = Self::default();
        if let Some(v) = doc.get("AudioVisual", "ConditionYellow").and_then(parse_condition_percent) {
            out.yellow = v;
        }
        if let Some(v) = doc.get("AudioVisual", "ConditionRed").and_then(parse_condition_percent) {
            out.red = v;
        }
        // 零售写在 `[General]`；个别模组可能挂在 `[AudioVisual]`。
        if let Some(raw) = doc
            .get("General", "DamageFireTypes")
            .or_else(|| doc.get("General", "DamageFireNames"))
            .or_else(|| doc.get("AudioVisual", "DamageFireTypes"))
            .or_else(|| doc.get("AudioVisual", "DamageFireNames"))
        {
            out.fire_types = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_uppercase())
                .collect();
        }
        out
    }

    /// 地图 `health`（0..=256）是否已进入黄血（应显示受损/起火）。
    pub fn is_yellow(&self, health_256: u16) -> bool {
        health_ratio_256(health_256) <= self.yellow
    }

    /// 是否已进入红血。
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

/// 主体帧：黄血及以下且至少 2 帧主体时用第 1 帧（受损），否则第 0 帧。
pub fn damaged_body_frame(health_256: u16, yellow: f32, body_frames: usize) -> u16 {
    if body_frames < 2 {
        return 0;
    }
    if health_ratio_256(health_256) <= yellow {
        1
    } else {
        0
    }
}

/// 解析 `DamageFireOffsetN=x,y`。
pub fn parse_damage_fire_offset(raw: &str) -> Option<(i32, i32)> {
    let mut parts = raw.split(',').map(str::trim);
    let x: i32 = parts.next()?.parse().ok()?;
    let y: i32 = parts.next()?.parse().ok()?;
    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::IniDocument;

    #[test]
    fn parses_percent_and_picks_damaged_frame() {
        assert_eq!(parse_condition_percent("50%"), Some(0.5));
        assert_eq!(parse_condition_percent("25%"), Some(0.25));
        assert_eq!(damaged_body_frame(256, 0.5, 2), 0);
        assert_eq!(damaged_body_frame(128, 0.5, 2), 1);
        assert_eq!(damaged_body_frame(64, 0.5, 1), 0);
        assert_eq!(parse_damage_fire_offset("57,-13"), Some((57, -13)));
    }

    #[test]
    fn damage_fire_types_read_from_general() {
        let doc = IniDocument::parse(
            b"[General]\nDamageFireTypes=FIRE01,FIRE02,FIRE03\n\
[AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n",
        )
        .unwrap();
        let rules = StructureDamageRules::from_rules_doc(&doc);
        assert_eq!(rules.fire_types, vec!["FIRE01", "FIRE02", "FIRE03"]);
        assert_eq!(rules.yellow, 0.5);
        assert_eq!(rules.red, 0.25);
    }
}
