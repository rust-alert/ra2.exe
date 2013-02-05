//! 地图 / 剧院。

mod theater;

use ra_assets::IniDocument;
use ra_types::{GameEdition, RaError, RaResult};

pub use theater::{Theater, theater_mix_names, theater_palette};

/// 地图基本信息（先读 Size / Theater，地形包后续再解）。
#[derive(Debug, Clone)]
pub struct MapInfo {
    pub edition: GameEdition,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub theater: Theater,
}

impl MapInfo {
    pub fn empty(edition: GameEdition, name: impl Into<String>) -> Self {
        Self {
            edition,
            name: name.into(),
            width: 0,
            height: 0,
            theater: Theater::Temperate,
        }
    }

    /// 从场景 INI（`.map` / `.mpr`）解析尺寸与剧院。
    pub fn parse_ini(edition: GameEdition, name: impl Into<String>, bytes: &[u8]) -> RaResult<Self> {
        let doc = IniDocument::parse(bytes)?;
        let size = doc
            .get("Map", "Size")
            .ok_or_else(|| RaError::Parse("地图缺少 [Map] Size".into()))?;
        let (width, height) = parse_size(size)?;
        let theater_raw = doc.get("Map", "Theater").unwrap_or("TEMPERATE");
        let theater = Theater::parse(theater_raw)?;
        Ok(Self {
            edition,
            name: name.into(),
            width,
            height,
            theater,
        })
    }
}

/// `Size=x,y,width,height`
fn parse_size(raw: &str) -> RaResult<(u32, u32)> {
    let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
    if parts.len() < 4 {
        return Err(RaError::Parse(format!("无效 Size: {raw}")));
    }
    let width: u32 = parts[2]
        .parse()
        .map_err(|_| RaError::Parse(format!("Size 宽无效: {}", parts[2])))?;
    let height: u32 = parts[3]
        .parse()
        .map_err(|_| RaError::Parse(format!("Size 高无效: {}", parts[3])))?;
    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_map_ini() {
        let text = b"[Map]\nSize=0,0,50,40\nTheater=SNOW\n";
        let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
        assert_eq!(info.width, 50);
        assert_eq!(info.height, 40);
        assert_eq!(info.theater, Theater::Snow);
    }
}
