//! 地图剧本相关节：Houses / Tags / Triggers / Events / Actions / CellTags / Teams。

mod houses;
mod script_teams;
mod triggers;

pub use houses::{MapHouse, parse_map_houses};
pub use script_teams::{
    MapScriptStep, MapScriptType, MapTaskForce, MapTaskForceEntry, MapTeamType, parse_script_types, parse_task_forces,
    parse_team_types,
};
pub use triggers::{
    MapAction, MapActionCommand, MapCellTag, MapEvent, MapEventCondition, MapTag, MapTrigger, parse_actions, parse_cell_tags,
    parse_events, parse_tags, parse_triggers,
};

use ra_assets::IniDocument;

/// 地图剧本数据（触发链与脚本队）。缺节则为空。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapScripting {
    /// `[Houses]` 各方。
    pub houses: Vec<MapHouse>,
    /// `[Tags]`。
    pub tags: Vec<MapTag>,
    /// `[Triggers]`。
    pub triggers: Vec<MapTrigger>,
    /// `[Events]`（与 trigger id 对齐）。
    pub events: Vec<MapEvent>,
    /// `[Actions]`（与 trigger id 对齐）。
    pub actions: Vec<MapAction>,
    /// `[CellTags]`。
    pub cell_tags: Vec<MapCellTag>,
    /// `[TaskForces]`。
    pub task_forces: Vec<MapTaskForce>,
    /// `[ScriptTypes]`。
    pub script_types: Vec<MapScriptType>,
    /// `[TeamTypes]`。
    pub team_types: Vec<MapTeamType>,
    /// 识别到但本解析器未建模的节名（供能力缺口报告）。
    pub unknown_sections: Vec<String>,
}

/// 已知会解析或明确忽略（几何等）的节，不进 unknown。
const KNOWN_SECTIONS: &[&str] = &[
    "Map",
    "Basic",
    "Lighting",
    "Preview",
    "PreviewPack",
    "IsoMapPack5",
    "OverlayPack",
    "OverlayDataPack",
    "Terrain",
    "Smudge",
    "Structures",
    "Units",
    "Infantry",
    "Aircraft",
    "Waypoints",
    "Houses",
    "Tags",
    "Triggers",
    "Events",
    "Actions",
    "CellTags",
    "TaskForces",
    "ScriptTypes",
    "TeamTypes",
    "AITriggerTypes",
    "SpecialFlags",
    "VariableNames",
    "Digest",
];

/// 从场景 INI 解析剧本相关节。
pub fn parse_map_scripting(doc: &IniDocument) -> MapScripting {
    let mut scripting = MapScripting {
        houses: parse_map_houses(doc),
        tags: parse_tags(doc),
        triggers: parse_triggers(doc),
        events: parse_events(doc),
        actions: parse_actions(doc),
        cell_tags: parse_cell_tags(doc),
        task_forces: parse_task_forces(doc),
        script_types: parse_script_types(doc),
        team_types: parse_team_types(doc),
        unknown_sections: Vec::new(),
    };
    scripting.unknown_sections = collect_unknown_sections(doc);
    scripting
}

fn collect_unknown_sections(doc: &IniDocument) -> Vec<String> {
    let mut out = Vec::new();
    for sec in &doc.sections {
        let name = sec.name_raw.as_str();
        if name.is_empty() {
            continue;
        }
        let known = KNOWN_SECTIONS.iter().any(|k| k.eq_ignore_ascii_case(name));
        if known {
            continue;
        }
        if scripting_house_section(doc, name) {
            continue;
        }
        if scripting_named_object_section(doc, name) {
            continue;
        }
        out.push(name.to_string());
    }
    out.sort();
    out.dedup();
    out
}

fn scripting_house_section(doc: &IniDocument, name: &str) -> bool {
    let Some(sec) = doc.section("Houses")
    else {
        return false;
    };
    sec.pairs().any(|(_, v)| v.eq_ignore_ascii_case(name))
}

fn scripting_named_object_section(doc: &IniDocument, name: &str) -> bool {
    for list in ["TaskForces", "ScriptTypes", "TeamTypes"] {
        let Some(sec) = doc.section(list)
        else {
            continue;
        };
        if sec.pairs().any(|(_, v)| v.eq_ignore_ascii_case(name)) {
            return true;
        }
    }
    false
}
