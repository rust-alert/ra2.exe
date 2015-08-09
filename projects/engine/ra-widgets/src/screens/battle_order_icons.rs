//! 对局命令图标：`mouse.shp` + `mousepal.pal` 的移动 / 攻击 / 部署帧。
//!
//! 帧号与原版光标表一致：Move=31×10、Attack=53×5、Deploy=110×9。

use ra_assets::{Palette, ShpFile};
use ra_renderer::DecodedOrderIcons;
use ra_types::AssetSource;

use crate::{fs_source::GameAssetSource, ui_decode::frame_to_canvas_rgba};

/// Move 光标起始帧。
pub const MOUSE_MOVE_START: usize = 31;
/// Move 帧数。
pub const MOUSE_MOVE_LEN: usize = 10;
/// Attack 光标起始帧。
pub const MOUSE_ATTACK_START: usize = 53;
/// Attack 帧数。
pub const MOUSE_ATTACK_LEN: usize = 5;
/// Deploy 光标起始帧。
pub const MOUSE_DEPLOY_START: usize = 110;
/// Deploy 帧数。
pub const MOUSE_DEPLOY_LEN: usize = 9;

/// 从安装资源加载 `mouse.shp` 命令图标。失败返回 `None`（对局仍可跑，仅无图标）。
pub fn load_battle_order_icons(source: &GameAssetSource) -> Option<DecodedOrderIcons> {
    let shp_bytes = source.read("mouse.shp").ok()?;
    let shp = ShpFile::parse(&shp_bytes).ok()?;
    let pal_bytes = source
        .read("mousepal.pal")
        .or_else(|_| source.read("mouse.pal"))
        .ok()?;
    let pal = Palette::parse(&pal_bytes).ok()?;
    let move_frames = decode_range(&shp, &pal, MOUSE_MOVE_START, MOUSE_MOVE_LEN)?;
    let attack_frames = decode_range(&shp, &pal, MOUSE_ATTACK_START, MOUSE_ATTACK_LEN)?;
    let deploy_frames = decode_range(&shp, &pal, MOUSE_DEPLOY_START, MOUSE_DEPLOY_LEN)?;
    let (canvas_w, canvas_h) = move_frames
        .first()
        .map(|f| (f.width(), f.height()))
        .unwrap_or((u32::from(shp.width), u32::from(shp.height)));
    Some(DecodedOrderIcons {
        canvas_w,
        canvas_h,
        move_frames,
        attack_frames,
        deploy_frames,
    })
}

fn decode_range(
    shp: &ShpFile,
    pal: &Palette,
    start: usize,
    len: usize,
) -> Option<Vec<ra_renderer::RgbaImage>> {
    if start >= shp.frames.len() || len == 0 {
        return None;
    }
    let end = (start + len).min(shp.frames.len());
    let mut out = Vec::with_capacity(end - start);
    for i in start..end {
        let img = frame_to_canvas_rgba(shp, &shp.frames[i], pal)?;
        out.push(img);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
