//! 地图放置段：`[Structures]` / `[Units]` / `[Infantry]` / `[Aircraft]`。
//!
//! 行值是 Westwood CSV，经 [`ra_assets::from_row`] 按列序反序列化。

use std::fmt;

use ra_assets::{IniDocument, from_row};
use ra_types::{HouseName, MissionName, TagName, TechnoName};
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

/// 放置类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapEntityKind {
    /// 建筑。
    Structure,
    /// 载具。
    Unit,
    /// 步兵。
    Infantry,
    /// 飞行器。
    Aircraft,
}

/// 场景里预放的一个实体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntity {
    /// 放置类别。
    pub kind: MapEntityKind,
    /// 所属方名称（装载期一次解码为大写）。
    pub owner: HouseName,
    /// 类型 id（装载期一次解码为大写）。
    pub type_id: TechnoName,
    /// 0..=256，零售常用 256 表示满血。
    pub health: u16,
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 朝向。
    pub facing: u8,
    /// 仅步兵：子格 0..=4；其它为 0。
    pub sub_cell: u8,
    /// 初始任务（装载期一次解码为大写）；空表示未指定。
    pub mission: MissionName,
    /// 绑定的 Tag id（装载期一次解码为大写 Tags 键；空表示无）。
    pub tag: TagName,
}

impl MapEntity {
    /// 测试 / 工具用：无 mission / tag 的放置。
    pub fn plain(
        kind: MapEntityKind,
        owner: impl Into<HouseName>,
        type_id: impl Into<TechnoName>,
        health: u16,
        x: u16,
        y: u16,
        facing: u8,
        sub_cell: u8,
    ) -> Self {
        Self {
            kind,
            owner: owner.into(),
            type_id: type_id.into(),
            health,
            x,
            y,
            facing,
            sub_cell,
            mission: MissionName::default(),
            tag: TagName::default(),
        }
    }
}

/// 解析四类放置段（缺省节则跳过）。
pub fn parse_map_entities(doc: &IniDocument) -> Vec<MapEntity> {
    let mut out = Vec::new();
    parse_section(doc, "Units", MapEntityKind::Unit, &mut out);
    parse_section(doc, "Aircraft", MapEntityKind::Aircraft, &mut out);
    parse_section(doc, "Infantry", MapEntityKind::Infantry, &mut out);
    parse_section(doc, "Structures", MapEntityKind::Structure, &mut out);
    out
}

fn parse_section(doc: &IniDocument, section: &str, kind: MapEntityKind, out: &mut Vec<MapEntity>) {
    let Some(sec) = doc.section(section)
    else {
        return;
    };
    for (_key, value) in sec.pairs() {
        if let Some(entity) = parse_line(kind, value) {
            out.push(entity);
        }
    }
}

fn parse_line(kind: MapEntityKind, value: &str) -> Option<MapEntity> {
    match kind {
        MapEntityKind::Infantry => {
            // HOUSE,ID,HEALTH,X,Y,SUBCELL,MISSION,FACING[,TAG,…]
            let row: InfantryRow = from_row(value).ok()?;
            Some(MapEntity {
                kind,
                owner: row.owner,
                type_id: row.type_id,
                health: row.health,
                x: row.x,
                y: row.y,
                sub_cell: row.sub_cell,
                mission: row.mission,
                facing: row.facing,
                tag: row.tag,
            })
        }
        MapEntityKind::Unit | MapEntityKind::Aircraft => {
            // HOUSE,ID,HEALTH,X,Y,FACING[,MISSION[,TAG,…]]
            let row: MobileRow = from_row(value).ok()?;
            Some(MapEntity {
                kind,
                owner: row.owner,
                type_id: row.type_id,
                health: row.health,
                x: row.x,
                y: row.y,
                facing: row.facing,
                sub_cell: 0,
                mission: row.mission,
                tag: row.tag,
            })
        }
        MapEntityKind::Structure => {
            // HOUSE,ID,HEALTH,X,Y,FACING[,TAG,…]
            let row: StructureRow = from_row(value).ok()?;
            Some(MapEntity {
                kind,
                owner: row.owner,
                type_id: row.type_id,
                health: row.health,
                x: row.x,
                y: row.y,
                facing: row.facing,
                sub_cell: 0,
                mission: MissionName::default(),
                tag: row.tag,
            })
        }
    }
}

#[derive(Debug, Deserialize)]
struct InfantryRow {
    owner: HouseName,
    type_id: TechnoName,
    #[serde(deserialize_with = "placement_health")]
    health: u16,
    x: u16,
    y: u16,
    #[serde(deserialize_with = "placement_sub_cell")]
    sub_cell: u8,
    #[serde(default)]
    mission: MissionName,
    #[serde(deserialize_with = "placement_facing")]
    facing: u8,
    #[serde(default)]
    tag: TagName,
}

#[derive(Debug, Deserialize)]
struct MobileRow {
    owner: HouseName,
    type_id: TechnoName,
    #[serde(deserialize_with = "placement_health")]
    health: u16,
    x: u16,
    y: u16,
    #[serde(deserialize_with = "placement_facing")]
    facing: u8,
    #[serde(default)]
    mission: MissionName,
    #[serde(default)]
    tag: TagName,
}

#[derive(Debug, Deserialize)]
struct StructureRow {
    owner: HouseName,
    type_id: TechnoName,
    #[serde(deserialize_with = "placement_health")]
    health: u16,
    x: u16,
    y: u16,
    #[serde(deserialize_with = "placement_facing")]
    facing: u8,
    #[serde(default)]
    tag: TagName,
}

/// 放置血量：非法回落 256 并钳到 `0..=256`；空列失败（行不够长）。
fn placement_health<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    struct HealthVisitor;

    impl<'de> Visitor<'de> for HealthVisitor {
        type Value = u16;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("placement health 0..=256")
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u16, E> {
            Ok(v.min(256) as u16)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<u16, E> {
            if v < 0 {
                return Ok(256);
            }
            self.visit_u64(v as u64)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<u16, E> {
            let t = v.trim();
            if t.is_empty() {
                return Err(E::custom("缺 health 列"));
            }
            Ok(t.parse().unwrap_or(256).min(256))
        }
    }

    deserializer.deserialize_any(HealthVisitor)
}

/// 朝向：非法回落 0，钳到 `u8`；空列失败。
fn placement_facing<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    struct FacingVisitor;

    impl<'de> Visitor<'de> for FacingVisitor {
        type Value = u8;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("placement facing 0..=255")
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u8, E> {
            Ok(v.min(255) as u8)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<u8, E> {
            if v < 0 {
                return Ok(0);
            }
            self.visit_u64(v as u64)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<u8, E> {
            let t = v.trim();
            if t.is_empty() {
                return Err(E::custom("缺 facing 列"));
            }
            Ok(t.parse::<u16>().unwrap_or(0).min(255) as u8)
        }
    }

    deserializer.deserialize_any(FacingVisitor)
}

/// 步兵子格：非法回落 0，钳到 `0..=4`；空列失败。
fn placement_sub_cell<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    struct SubCellVisitor;

    impl<'de> Visitor<'de> for SubCellVisitor {
        type Value = u8;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("infantry sub_cell 0..=4")
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u8, E> {
            Ok(v.min(4) as u8)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<u8, E> {
            if v < 0 {
                return Ok(0);
            }
            self.visit_u64(v as u64)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<u8, E> {
            let t = v.trim();
            if t.is_empty() {
                return Err(E::custom("缺 sub_cell 列"));
            }
            Ok(t.parse().unwrap_or(0).min(4))
        }
    }

    deserializer.deserialize_any(SubCellVisitor)
}
