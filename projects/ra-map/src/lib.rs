//! 地图 / 剧院（占位）。

use ra_types::GameEdition;

#[derive(Debug, Clone)]
pub struct MapInfo {
    pub edition: GameEdition,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

impl MapInfo {
    pub fn empty(edition: GameEdition, name: impl Into<String>) -> Self {
        Self {
            edition,
            name: name.into(),
            width: 0,
            height: 0,
        }
    }
}
