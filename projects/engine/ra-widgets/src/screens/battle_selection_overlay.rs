//! 对局选中血条：`pips.shp` + `pipbrd.shp` + `palette.pal`。

use ra_assets::{Palette, ShpFile};
use ra_renderer::{DecodedSelectionOverlay, RgbaImage};
use ra_types::AssetSource;

use crate::{fs_source::GameAssetSource, skin::decode::frame_to_canvas_rgba};

/// 从安装资源加载选中血条图集。失败返回 `None`。
pub fn load_selection_overlay(source: &GameAssetSource) -> Option<DecodedSelectionOverlay> {
    let pal = load_palette(source)?;
    let (pipbrd, pipbrd_w, pipbrd_h) = load_shp_frames(source, "pipbrd.shp", &pal)?;
    let (pips, pips_w, pips_h) = load_shp_frames(source, "pips.shp", &pal)?;
    // RA2：`pipbrd` 2 帧；`pips` ≥18 帧（单位血格 15/16/17）。
    if pipbrd.len() < 2 || pips.len() < 18 {
        return None;
    }
    Some(DecodedSelectionOverlay {
        pipbrd_frames: pipbrd,
        pips_frames: pips,
        pipbrd_canvas_w: pipbrd_w,
        pipbrd_canvas_h: pipbrd_h,
        pips_canvas_w: pips_w,
        pips_canvas_h: pips_h,
    })
}

fn load_palette(source: &GameAssetSource) -> Option<Palette> {
    let bytes = source.read("palette.pal").or_else(|_| source.read("unittem.pal")).ok()?;
    Palette::parse(&bytes).ok()
}

fn load_shp_frames(source: &GameAssetSource, name: &str, pal: &Palette) -> Option<(Vec<RgbaImage>, u32, u32)> {
    let bytes = source.read(name).ok()?;
    let shp = ShpFile::parse(&bytes).ok()?;
    if shp.frames.is_empty() {
        return None;
    }
    let mut frames = Vec::with_capacity(shp.frames.len());
    for frame in &shp.frames {
        let img = frame_to_canvas_rgba(&shp, frame, pal)?;
        frames.push(img);
    }
    Some((frames, u32::from(shp.width).max(1), u32::from(shp.height).max(1)))
}
