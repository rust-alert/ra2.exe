//! 地图 `[Terrain]`：树 / 岩石等静态物件占位。

use ra_assets::IniDocument;

/// 一处地形物件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainObject {
    pub x: u16,
    pub y: u16,
    pub name: String,
}

/// 解析 `[Terrain]`：键为 `y * 1000 + x`，值为物件类型名。
pub fn parse_terrain_objects(doc: &IniDocument) -> Vec<TerrainObject> {
    let Some(section) = doc.sections.get("Terrain") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in &section.order {
        let Ok(pos) = key.parse::<u32>() else {
            continue;
        };
        let name = value.trim();
        if name.is_empty() {
            continue;
        }
        let y = (pos / 1000) as u16;
        let x = (pos % 1000) as u16;
        out.push(TerrainObject {
            x,
            y,
            name: name.to_ascii_uppercase(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_trees() {
        let doc = IniDocument::parse(b"[Terrain]\n3002=INTREE01\n4005=CACTUS01\n").unwrap();
        let objs = parse_terrain_objects(&doc);
        assert_eq!(objs.len(), 2);
        assert_eq!(objs[0].y, 3);
        assert_eq!(objs[0].x, 2);
        assert_eq!(objs[0].name, "INTREE01");
        assert_eq!(objs[1].y, 4);
        assert_eq!(objs[1].x, 5);
        assert_eq!(objs[1].name, "CACTUS01");
    }
}
