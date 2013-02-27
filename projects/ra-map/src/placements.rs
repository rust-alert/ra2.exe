//! 地图放置段：`[Structures]` / `[Units]` / `[Infantry]` / `[Aircraft]`。

use ra_assets::IniDocument;

/// 放置类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapEntityKind {
    Structure,
    Unit,
    Infantry,
    Aircraft,
}

/// 场景里预放的一个实体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEntity {
    pub kind: MapEntityKind,
    pub owner: String,
    pub type_id: String,
    /// 0..=256，零售常用 256 表示满血。
    pub health: u16,
    pub x: u16,
    pub y: u16,
    pub facing: u8,
    /// 仅步兵：子格 0..=4；其它为 0。
    pub sub_cell: u8,
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

fn parse_section(
    doc: &IniDocument,
    section: &str,
    kind: MapEntityKind,
    out: &mut Vec<MapEntity>,
) {
    let Some(sec) = doc.sections.get(section) else {
        return;
    };
    for (_key, value) in &sec.order {
        if let Some(entity) = parse_line(kind, value) {
            out.push(entity);
        }
    }
}

fn parse_line(kind: MapEntityKind, value: &str) -> Option<MapEntity> {
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    match kind {
        MapEntityKind::Infantry => {
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
                facing: fields[7].parse::<u16>().unwrap_or(0).min(255) as u8,
            })
        }
        MapEntityKind::Structure | MapEntityKind::Unit | MapEntityKind::Aircraft => {
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
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_structure_and_infantry() {
        let text = b"\
[Structures]\n\
1=Neutral,GACNST,256,10,20,0,None\n\
[Infantry]\n\
2=Americans,E1,256,11,21,2,Guard,32\n\
";
        let doc = IniDocument::parse(text).unwrap();
        let ents = parse_map_entities(&doc);
        assert_eq!(ents.len(), 2);
        let structure = ents
            .iter()
            .find(|e| e.kind == MapEntityKind::Structure)
            .unwrap();
        let infantry = ents
            .iter()
            .find(|e| e.kind == MapEntityKind::Infantry)
            .unwrap();
        assert_eq!(structure.type_id, "GACNST");
        assert_eq!(structure.x, 10);
        assert_eq!(infantry.sub_cell, 2);
        assert_eq!(infantry.facing, 32);
    }
}
