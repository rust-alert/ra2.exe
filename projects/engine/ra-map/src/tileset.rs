//! 剧院瓷砖表：`tile_num` → TMP 文件名。

use ra_assets::IniDocument;
use ra_types::{RaError, RaResult};

/// 累计索引表：`IsoCell.tile_num` 查 TMP 名。
#[derive(Debug, Clone, Default)]
pub struct TilesetLookup {
    /// 与全局 tile_id 对齐；空槽为 `None`（blank）。
    entries: Vec<Option<String>>,
}

impl TilesetLookup {
    /// 槽位总数（含 blank）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否无任何槽位。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按全局 `tile_num` 取 TMP 文件名。
    pub fn filename(&self, tile_num: i32) -> Option<&str> {
        if tile_num < 0 {
            return None;
        }
        self.entries.get(tile_num as usize).and_then(|e| e.as_deref())
    }
}

/// 解析剧院 INI 的 `[TileSetNNNN]` 序列。
///
/// 文件名规则：`{FileName}{NN:02}.{extension}`，NN 从 1 起。
/// 空 `FileName` 仍占用编号槽位。
pub fn parse_tileset_ini(ini_data: &[u8], extension: &str) -> RaResult<TilesetLookup> {
    let doc = IniDocument::parse(ini_data)?;
    let mut entries = Vec::new();
    let mut idx = 0u32;
    loop {
        let section = format!("TileSet{idx:04}");
        let Some(tiles_raw) = doc.get(&section, "TilesInSet")
        else {
            break;
        };
        let tiles_in_set: i32 = tiles_raw.parse().unwrap_or(-1);
        if tiles_in_set < 0 {
            break;
        }
        let filename = doc.get(&section, "FileName").unwrap_or("");
        let count = tiles_in_set as usize;
        if filename.is_empty() {
            for _ in 0..count {
                entries.push(None);
            }
        }
        else {
            for i in 1..=count {
                entries.push(Some(format!("{filename}{i:02}.{extension}")));
            }
        }
        idx = idx.checked_add(1).ok_or_else(|| RaError::Parse("TileSet 序号溢出".into()))?;
        if idx > 10_000 {
            return Err(RaError::Parse("TileSet 过多".into()));
        }
    }
    Ok(TilesetLookup { entries })
}
