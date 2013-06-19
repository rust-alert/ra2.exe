//! 地图合成失败时的启动预览回退（单砖 / 单精灵）。

use ra_assets::{Palette, ShpFile, TmpFile};
use ra_types::AssetSource;

use crate::theater::{theater_ini_name, theater_palette, theater_tmp_extension, Theater};
use crate::tileset::parse_tileset_ini;

/// 原始 RGBA 缓冲（壳层再包成 GPU 图像）。
#[derive(Debug, Clone)]
pub struct RawRgbaImage {
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// 从剧院 TMP 抽一块砖作为回退预览。
pub fn load_fallback_theater_tile(
    source: &dyn AssetSource,
    theater: Theater,
) -> Option<RawRgbaImage> {
    let pal_bytes = source.read(theater_palette(theater)).ok()?;
    let pal = Palette::parse(&pal_bytes).ok()?;

    let mut candidates: Vec<String> = Vec::new();
    if let Ok(ini_bytes) = source.read(theater_ini_name(theater)) {
        if let Ok(lookup) = parse_tileset_ini(&ini_bytes, theater_tmp_extension(theater)) {
            if let Some(name) = lookup.filename(0) {
                candidates.push(name.to_string());
            }
            for id in [14i32, 9, 10, 12] {
                if let Some(name) = lookup.filename(id) {
                    candidates.push(name.to_string());
                }
            }
        }
    }
    candidates.push(format!("clear01.{}", theater_tmp_extension(theater)));

    for name in candidates {
        let Ok(data) = source.read(&name) else {
            continue;
        };
        let Ok(tmp) = TmpFile::parse(&data) else {
            continue;
        };
        let Some((index, tile)) = tmp
            .tiles
            .iter()
            .enumerate()
            .find_map(|(i, t)| t.as_ref().map(|tile| (i, tile)))
        else {
            continue;
        };
        let Ok(rgba) = tmp.tile_to_rgba(index, &pal) else {
            continue;
        };
        return Some(RawRgbaImage {
            label: format!("{name}#{index}"),
            width: tile.pixel_width,
            height: tile.pixel_height,
            pixels: rgba,
        });
    }
    None
}

/// 从常见 UI/单位 SHP 抽第一帧作为回退预览。
pub fn load_fallback_unit_sprite(source: &dyn AssetSource) -> Option<RawRgbaImage> {
    let pal_bytes = source.read("unittem.pal").ok()?;
    let pal = Palette::parse(&pal_bytes).ok()?;
    let candidates = [
        "mouse.shp",
        "e1.shp",
        "clock.shp",
        "power.shp",
        "gaairc.shp",
    ];
    for name in candidates {
        let Ok(bytes) = source.read(name) else {
            continue;
        };
        let Ok(shp) = ShpFile::parse(&bytes) else {
            continue;
        };
        let Some(frame) = shp.frames.first() else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        return Some(RawRgbaImage {
            label: name.to_string(),
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            pixels: frame.to_rgba(&pal),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_types::{RaError, RaResult};

    struct EmptySource;
    impl AssetSource for EmptySource {
        fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
            Err(RaError::MissingFile(relative.to_string()))
        }
    }

    #[test]
    fn empty_source_yields_none() {
        assert!(load_fallback_theater_tile(&EmptySource, Theater::Temperate).is_none());
        assert!(load_fallback_unit_sprite(&EmptySource).is_none());
    }
}
