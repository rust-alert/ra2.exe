//! `[Tags]` / `[Triggers]` / `[Events]` / `[Actions]` / `[CellTags]`。

use std::fmt;

use ra_assets::{CsvField, CsvRow, IniDocument, from_csv_row, from_row, parse_westwood_csv_line};
use ra_types::{HouseName, TagName, TriggerName};
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

use super::{MapActionKind, MapEventKind};

/// `[Tags]` 一行（装载解析中间态；投影进 `ra_types::MapTag` 后由运行契约消费）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTag {
    /// Tag id。
    pub id: String,
    /// 持久性：0 volatile / 1 semi / 2 persistent。
    pub persistence: u8,
    /// 编辑器名。
    pub name: String,
    /// 关联 Trigger id（装载期一次解码为大写 Triggers 键）。
    pub trigger_id: TriggerName,
}

/// `[Triggers]` 一行（装载解析中间态；投影进 `ra_types::MapTrigger` 后由运行契约消费）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTrigger {
    /// Trigger id。
    pub id: String,
    /// 所属 house（装载期一次解码为大写）。
    pub house: HouseName,
    /// 链接的另一 trigger（`<none>` 表示无）。
    pub linked: String,
    /// 编辑器名。
    pub name: String,
    /// `1` = 初始禁用。
    pub disabled: bool,
    /// Easy 难度启用。
    pub easy: bool,
    /// Normal 难度启用。
    pub normal: bool,
    /// Hard 难度启用。
    pub hard: bool,
}

/// 单条事件条件（装载解析中间态；投影进 `ra_types::MapEventCondition`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEventCondition {
    /// 事件类型（未知原版码为 [`MapEventKind::Unknown`]）。
    pub kind: MapEventKind,
    /// 参数（通常 2 个 int；变长事件保留原文参数）。
    pub params: Vec<String>,
}

/// `[Events]` 中与某 trigger 对齐的事件表（装载解析中间态；投影进 `ra_types::MapEvent`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEvent {
    /// Trigger id。
    pub id: String,
    /// 条件列表。
    pub conditions: Vec<MapEventCondition>,
}

/// 单条动作（装载解析中间态；投影进 `ra_types::MapActionCommand`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapActionCommand {
    /// 动作类型（未知原版码为 [`MapActionKind::Unknown`]）。
    pub kind: MapActionKind,
    /// 七个参数槽（含航点字母等）。
    pub params: [String; 7],
}

/// `[Actions]` 中与某 trigger 对齐的动作表（装载解析中间态；投影进 `ra_types::MapAction`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAction {
    /// Trigger id。
    pub id: String,
    /// 动作列表。
    pub commands: Vec<MapActionCommand>,
}

/// `[CellTags]`：格子绑定 Tag（装载解析中间态；投影进 `ra_types::MapCellTag`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapCellTag {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// Tag id（装载期一次解码为大写 Tags 键）。
    pub tag_id: TagName,
}

#[derive(Debug, Deserialize)]
struct TagCsvRow {
    persistence: u8,
    name: String,
    trigger_id: TriggerName,
}

#[derive(Debug, Deserialize)]
struct TriggerCsvRow {
    house: HouseName,
    linked: String,
    name: String,
    #[serde(deserialize_with = "flag_is_one")]
    disabled: bool,
    #[serde(deserialize_with = "flag_not_zero")]
    easy: bool,
    #[serde(deserialize_with = "flag_not_zero")]
    normal: bool,
    #[serde(deserialize_with = "flag_not_zero")]
    hard: bool,
}

#[derive(Debug, Deserialize)]
struct EventConditionCsvRow {
    kind: i32,
    #[serde(default)]
    p1: String,
    #[serde(default)]
    p2: String,
}

#[derive(Debug, Deserialize)]
struct ActionCommandCsvRow {
    kind: i32,
    #[serde(default)]
    p0: String,
    #[serde(default)]
    p1: String,
    #[serde(default)]
    p2: String,
    #[serde(default)]
    p3: String,
    #[serde(default)]
    p4: String,
    #[serde(default)]
    p5: String,
    #[serde(default)]
    p6: String,
}

#[derive(Debug, Deserialize)]
struct ChunkCountCsvRow {
    count: usize,
}

/// 行首块数（Events / Actions 的 `count, ...`）。
fn leading_chunk_count(row: &CsvRow) -> usize {
    from_csv_row::<ChunkCountCsvRow>(&csv_slice(row, 0, 1)).map(|r| r.count).unwrap_or(0)
}

/// `1` 为真，其余为假。
fn flag_is_one<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct FlagIsOne;

    impl<'de> Visitor<'de> for FlagIsOne {
        type Value = bool;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("trigger disabled flag (1 = true)")
        }

        fn visit_bool<E: de::Error>(self, v: bool) -> Result<bool, E> {
            Ok(v)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E> {
            Ok(v == 1)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<bool, E> {
            Ok(v == 1)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<bool, E> {
            Ok(v.trim() == "1")
        }
    }

    deserializer.deserialize_any(FlagIsOne)
}

/// 非 `0` 为真（缺列由上层行长校验兜住）。
fn flag_not_zero<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    struct FlagNotZero;

    impl<'de> Visitor<'de> for FlagNotZero {
        type Value = bool;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("trigger difficulty switch (non-zero = true)")
        }

        fn visit_bool<E: de::Error>(self, v: bool) -> Result<bool, E> {
            Ok(v)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E> {
            Ok(v != 0)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<bool, E> {
            Ok(v != 0)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<bool, E> {
            let t = v.trim();
            if t.is_empty() {
                return Err(E::custom("缺难度开关列"));
            }
            Ok(t != "0")
        }
    }

    deserializer.deserialize_any(FlagNotZero)
}

/// 解析 `[Tags]`。
pub fn parse_tags(doc: &IniDocument) -> Vec<MapTag> {
    let Some(sec) = doc.section("Tags")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let Ok(row) = from_row::<TagCsvRow>(value)
        else {
            continue;
        };
        out.push(MapTag {
            id: id.to_string(),
            persistence: row.persistence,
            name: row.name,
            trigger_id: row.trigger_id,
        });
    }
    out
}

/// 解析 `[Triggers]`。
pub fn parse_triggers(doc: &IniDocument) -> Vec<MapTrigger> {
    let Some(sec) = doc.section("Triggers")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let Ok(row) = from_row::<TriggerCsvRow>(value)
        else {
            continue;
        };
        out.push(MapTrigger {
            id: id.to_string(),
            house: row.house,
            linked: row.linked,
            name: row.name,
            disabled: row.disabled,
            easy: row.easy,
            normal: row.normal,
            hard: row.hard,
        });
    }
    out
}

/// 解析 `[Events]`（每条件默认 3 字段：kind,p1,p2）。
pub fn parse_events(doc: &IniDocument) -> Vec<MapEvent> {
    let Some(sec) = doc.section("Events")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let row = parse_westwood_csv_line(value);
        if row.is_empty() {
            continue;
        }
        let count = leading_chunk_count(&row);
        let mut conditions = Vec::new();
        let mut idx = 1usize;
        for _ in 0..count {
            if idx >= row.len() {
                break;
            }
            let chunk = csv_slice(&row, idx, 3);
            idx += chunk.len();
            let Ok(cond) = from_csv_row::<EventConditionCsvRow>(&chunk)
            else {
                continue;
            };
            conditions.push(MapEventCondition {
                kind: MapEventKind::from_code(cond.kind),
                params: vec![cond.p1, cond.p2],
            });
        }
        out.push(MapEvent { id: id.to_string(), conditions });
    }
    out
}

/// 解析 `[Actions]`（每动作 8 字段：kind + 7 params）。
pub fn parse_actions(doc: &IniDocument) -> Vec<MapAction> {
    let Some(sec) = doc.section("Actions")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let row = parse_westwood_csv_line(value);
        if row.is_empty() {
            continue;
        }
        let count = leading_chunk_count(&row);
        let mut commands = Vec::new();
        let mut idx = 1usize;
        for _ in 0..count {
            if idx >= row.len() {
                break;
            }
            let chunk = csv_slice(&row, idx, 8);
            idx += chunk.len();
            let Ok(cmd) = from_csv_row::<ActionCommandCsvRow>(&chunk)
            else {
                continue;
            };
            commands.push(MapActionCommand {
                kind: MapActionKind::from_code(cmd.kind),
                params: [cmd.p0, cmd.p1, cmd.p2, cmd.p3, cmd.p4, cmd.p5, cmd.p6],
            });
        }
        out.push(MapAction { id: id.to_string(), commands });
    }
    out
}

fn csv_slice(row: &CsvRow, start: usize, max_len: usize) -> CsvRow {
    let end = (start + max_len).min(row.len());
    CsvRow {
        fields: row.fields[start..end]
            .iter()
            .map(|f| CsvField {
                value: f.value.clone(),
            })
            .collect(),
    }
}

/// 解析 `[CellTags]`（键 = `y * 1000 + x`）。
pub fn parse_cell_tags(doc: &IniDocument) -> Vec<MapCellTag> {
    let Some(sec) = doc.section("CellTags")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (packed, tag_id) in sec.pairs() {
        let Some((x, y)) = crate::packed_cell::parse_packed_cell(packed)
        else {
            continue;
        };
        out.push(MapCellTag { x, y, tag_id: TagName::parse(tag_id) });
    }
    out
}
