//! 地图放置段：`[Structures]` / `[Units]` / `[Infantry]` / `[Aircraft]`。

use ra_assets::IniDocument;

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
    /// 所属方名称。
    pub owner: String,
    /// 类型 id（通常已大写）。
    pub type_id: String,
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
    /// 初始任务（如 `Guard`）；空表示未指定。
    pub mission: String,
    /// 绑定的 Tag id；空表示无。
    pub tag: String,
}

impl MapEntity {
    /// 测试 / 工具用：无 mission / tag 的放置。
    pub fn plain(
        kind: MapEntityKind,
        owner: impl Into<String>,
        type_id: impl Into<String>,
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
            mission: String::new(),
            tag: String::new(),
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
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    match kind {
        MapEntityKind::Infantry => {
            // HOUSE,ID,HEALTH,X,Y,SUBCELL,MISSION,FACING[,TAG,…]
            if fields.len() < 8 {
                return None;
            }
            Some(MapEntity {
                kind,
                owner: fields[0].to_string(),
                type_id: fields[1].to_ascii_uppercase(),
                health: fields[2].parse().unwrap_or(256).min(256),
                x: fields[3].parse().ok()?,
                y: fields[4].parse().ok()?,
                sub_cell: fields[5].parse().unwrap_or(0).min(4),
                mission: fields[6].to_string(),
                facing: fields[7].parse::<u16>().unwrap_or(0).min(255) as u8,
                tag: fields.get(8).unwrap_or(&"").to_string(),
            })
        }
        MapEntityKind::Unit | MapEntityKind::Aircraft => {
            // HOUSE,ID,HEALTH,X,Y,FACING[,MISSION[,TAG,…]]
            if fields.len() < 6 {
                return None;
            }
            Some(MapEntity {
                kind,
                owner: fields[0].to_string(),
                type_id: fields[1].to_ascii_uppercase(),
                health: fields[2].parse().unwrap_or(256).min(256),
                x: fields[3].parse().ok()?,
                y: fields[4].parse().ok()?,
                facing: fields[5].parse::<u16>().unwrap_or(0).min(255) as u8,
                sub_cell: 0,
                mission: fields.get(6).unwrap_or(&"").to_string(),
                tag: fields.get(7).unwrap_or(&"").to_string(),
            })
        }
        MapEntityKind::Structure => {
            // HOUSE,ID,HEALTH,X,Y,FACING[,TAG,…]
            if fields.len() < 6 {
                return None;
            }
            Some(MapEntity {
                kind,
                owner: fields[0].to_string(),
                type_id: fields[1].to_ascii_uppercase(),
                health: fields[2].parse().unwrap_or(256).min(256),
                x: fields[3].parse().ok()?,
                y: fields[4].parse().ok()?,
                facing: fields[5].parse::<u16>().unwrap_or(0).min(255) as u8,
                sub_cell: 0,
                mission: String::new(),
                tag: fields.get(6).unwrap_or(&"").to_string(),
            })
        }
    }
}
