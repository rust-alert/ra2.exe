//! 从 rules 扫描动画产矿地形节，产出冻结表。

use serde::Deserialize;

use ra_types::{TerrainName, TerrainSpawnerDefinition, TerrainSpawnerDefinitions};

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView};

const PROBABILITY_DENOMINATOR: f32 = 1_000_000.0;

#[derive(Debug, Deserialize)]
struct TerrainSpawnerSectionFields {
    #[serde(rename = "SpawnsTiberium")]
    spawns_tiberium: Option<bool>,
    #[serde(rename = "IsAnimated")]
    is_animated: Option<bool>,
    #[serde(rename = "AnimationProbability")]
    animation_probability: Option<f32>,
    #[serde(rename = "AnimationRate")]
    animation_rate: Option<u16>,
}

/// 扫描全部节，收集 `SpawnsTiberium` + `IsAnimated` 的产矿地形。
pub fn terrain_spawners_from_rules(rules: &IniDocument) -> TerrainSpawnerDefinitions {
    let policy = IniMergePolicy::last_wins();
    let docs = std::slice::from_ref(rules);
    terrain_spawners_from_layered(LayeredIniView::new(docs, &policy))
}

/// 从层叠 rules 视图扫描产矿地形节。
pub fn terrain_spawners_from_layered(view: LayeredIniView<'_>) -> TerrainSpawnerDefinitions {
    let mut out = TerrainSpawnerDefinitions::default();
    for name_key in view.section_keys() {
        let Some(section) = view.section(name_key)
        else {
            continue;
        };
        let Ok(fields) = section.deserialize::<TerrainSpawnerSectionFields>()
        else {
            continue;
        };
        if fields.spawns_tiberium != Some(true) || fields.is_animated != Some(true) {
            continue;
        }
        let probability = fields
            .animation_probability
            .map(|v| (v.clamp(0.0, 1.0) * PROBABILITY_DENOMINATOR).round() as u32)
            .unwrap_or(0);
        let rate = fields.animation_rate.unwrap_or(1).max(1);
        let type_key = TerrainName::parse(section.name_raw());
        if type_key.is_empty() {
            continue;
        }
        out.insert(TerrainSpawnerDefinition {
            type_key,
            animation_probability_micros: probability,
            animation_rate_ticks: rate,
        });
    }
    out
}
