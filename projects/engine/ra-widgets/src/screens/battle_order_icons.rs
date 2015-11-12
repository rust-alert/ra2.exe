//! 对局 `mouse.shp` + `mousepal.pal`：命令图标与软件光标。
//!
//! 帧号与原版光标表一致：
//! - Default = 0
//! - Scroll N…NW = 2…9 / blocked = 10…17
//! - Select = 18×13
//! - Move = 31×10 / NoMove = 41
//! - Attack = 53×5
//! - Deploy = 110×9 / NoDeploy = 119

use ra_assets::{Palette, ShpFile};
use ra_renderer::{DecodedOrderIcons, RgbaImage};
use ra_types::AssetSource;

use crate::{fs_source::GameAssetSource, skin::decode::frame_to_canvas_rgba};

/// Default 光标帧。
pub const MOUSE_DEFAULT_START: usize = 0;
/// Select 起始帧。
pub const MOUSE_SELECT_START: usize = 18;
/// Select 帧数。
pub const MOUSE_SELECT_LEN: usize = 13;
/// Move 光标起始帧。
pub const MOUSE_MOVE_START: usize = 31;
/// Move 帧数。
pub const MOUSE_MOVE_LEN: usize = 10;
/// NoMove（不可达）单帧。
pub const MOUSE_NO_MOVE_START: usize = 41;
/// Attack 光标起始帧。
pub const MOUSE_ATTACK_START: usize = 53;
/// Attack 帧数。
pub const MOUSE_ATTACK_LEN: usize = 5;
/// Deploy 光标起始帧。
pub const MOUSE_DEPLOY_START: usize = 110;
/// Deploy 帧数。
pub const MOUSE_DEPLOY_LEN: usize = 9;
/// NoDeploy 单帧。
pub const MOUSE_NO_DEPLOY_START: usize = 119;
/// 可滚边缘光标起始帧（N）。
pub const MOUSE_SCROLL_START: usize = 2;
/// 贴边禁止滚光标起始帧（N）。
pub const MOUSE_SCROLL_BLOCKED_START: usize = 10;
/// 八向边缘光标数量（N NE E SE S SW W NW）。
pub const MOUSE_SCROLL_DIR_COUNT: usize = 8;
/// 动画光标帧间隔（毫秒；原版表 rate=4 × 16ms）。
pub const MOUSE_CURSOR_ANIM_MS: u64 = 64;

/// 光标热点相对画布的锚点（与原版表 Left/Center/Right × Top/Middle/Bottom 一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseCursorHotspot {
    /// 左上。
    TopLeft,
    /// 上中。
    CenterTop,
    /// 右上。
    RightTop,
    /// 右中。
    RightMiddle,
    /// 右下。
    RightBottom,
    /// 下中。
    CenterBottom,
    /// 左下。
    LeftBottom,
    /// 左中。
    LeftMiddle,
    /// 正中。
    CenterMiddle,
}

impl MouseCursorHotspot {
    /// 将锚点换成像素热点（夹在画布内）。
    pub fn to_px(self, width: u32, height: u32) -> (u16, u16) {
        let w = width.max(1);
        let h = height.max(1);
        let (x, y) = match self {
            Self::TopLeft => (0, 0),
            Self::CenterTop => (w / 2, 0),
            Self::RightTop => (w.saturating_sub(1), 0),
            Self::RightMiddle => (w.saturating_sub(1), h / 2),
            Self::RightBottom => (w.saturating_sub(1), h.saturating_sub(1)),
            Self::CenterBottom => (w / 2, h.saturating_sub(1)),
            Self::LeftBottom => (0, h.saturating_sub(1)),
            Self::LeftMiddle => (0, h / 2),
            Self::CenterMiddle => (w / 2, h / 2),
        };
        (x as u16, y as u16)
    }
}

/// 八向顺序：N、NE、E、SE、S、SW、W、NW（与帧 2…9 / 10…17 对齐）。
pub const MOUSE_SCROLL_HOTSPOTS: [MouseCursorHotspot; MOUSE_SCROLL_DIR_COUNT] = [
    MouseCursorHotspot::CenterTop,
    MouseCursorHotspot::RightTop,
    MouseCursorHotspot::RightMiddle,
    MouseCursorHotspot::RightBottom,
    MouseCursorHotspot::CenterBottom,
    MouseCursorHotspot::LeftBottom,
    MouseCursorHotspot::LeftMiddle,
    MouseCursorHotspot::TopLeft,
];

/// 单帧软件光标（RGBA 画布 + 热点）。
#[derive(Debug, Clone)]
pub struct DecodedMouseCursorFrame {
    /// 画布像素。
    pub image: RgbaImage,
    /// 热点 X。
    pub hotspot_x: u16,
    /// 热点 Y。
    pub hotspot_y: u16,
}

/// 对局软件光标：默认 / 选择 / 移动 / 攻击 / 部署 / 边缘滚屏。
#[derive(Debug, Clone)]
pub struct DecodedBattleEdgeCursors {
    /// 默认箭头。
    pub default: DecodedMouseCursorFrame,
    /// 可点选本方单位。
    pub select: Vec<DecodedMouseCursorFrame>,
    /// 可移动。
    pub move_ok: Vec<DecodedMouseCursorFrame>,
    /// 不可移动。
    pub no_move: DecodedMouseCursorFrame,
    /// 可攻击。
    pub attack: Vec<DecodedMouseCursorFrame>,
    /// 可部署。
    pub deploy: Vec<DecodedMouseCursorFrame>,
    /// 不可部署。
    pub no_deploy: DecodedMouseCursorFrame,
    /// 可滚：N…NW。
    pub scroll: [DecodedMouseCursorFrame; MOUSE_SCROLL_DIR_COUNT],
    /// 贴边禁止：N…NW。
    pub blocked: [DecodedMouseCursorFrame; MOUSE_SCROLL_DIR_COUNT],
}

/// 从安装资源加载 `mouse.shp` 命令图标。失败返回 `None`（对局仍可跑，仅无图标）。
pub fn load_battle_order_icons(source: &GameAssetSource) -> Option<DecodedOrderIcons> {
    let (shp, pal) = open_mouse_shp(source)?;
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

/// 从同一份 `mouse.shp` 加载对局软件光标。失败返回 `None`。
pub fn load_battle_edge_cursors(source: &GameAssetSource) -> Option<DecodedBattleEdgeCursors> {
    let (shp, pal) = open_mouse_shp(source)?;
    let default = decode_one(&shp, &pal, MOUSE_DEFAULT_START, MouseCursorHotspot::TopLeft)?;
    let select = decode_seq(&shp, &pal, MOUSE_SELECT_START, MOUSE_SELECT_LEN, MouseCursorHotspot::CenterMiddle)?;
    let move_ok = decode_seq(&shp, &pal, MOUSE_MOVE_START, MOUSE_MOVE_LEN, MouseCursorHotspot::CenterMiddle)?;
    let no_move = decode_one(&shp, &pal, MOUSE_NO_MOVE_START, MouseCursorHotspot::CenterMiddle)?;
    let attack = decode_seq(&shp, &pal, MOUSE_ATTACK_START, MOUSE_ATTACK_LEN, MouseCursorHotspot::CenterMiddle)?;
    let deploy = decode_seq(&shp, &pal, MOUSE_DEPLOY_START, MOUSE_DEPLOY_LEN, MouseCursorHotspot::CenterMiddle)?;
    let no_deploy = decode_one(&shp, &pal, MOUSE_NO_DEPLOY_START, MouseCursorHotspot::CenterMiddle)?;
    let scroll = decode_dir_ring(&shp, &pal, MOUSE_SCROLL_START)?;
    let blocked = decode_dir_ring(&shp, &pal, MOUSE_SCROLL_BLOCKED_START)?;
    Some(DecodedBattleEdgeCursors {
        default,
        select,
        move_ok,
        no_move,
        attack,
        deploy,
        no_deploy,
        scroll,
        blocked,
    })
}

fn open_mouse_shp(source: &GameAssetSource) -> Option<(ShpFile, Palette)> {
    let shp_bytes = source.read("mouse.shp").ok()?;
    let shp = ShpFile::parse(&shp_bytes).ok()?;
    let pal_bytes = source
        .read("mousepal.pal")
        .or_else(|_| source.read("mouse.pal"))
        .ok()?;
    let pal = Palette::parse(&pal_bytes).ok()?;
    Some((shp, pal))
}

fn decode_one(
    shp: &ShpFile,
    pal: &Palette,
    index: usize,
    hotspot: MouseCursorHotspot,
) -> Option<DecodedMouseCursorFrame> {
    let img = frame_to_canvas_rgba(shp, shp.frames.get(index)?, pal)?;
    let (hx, hy) = hotspot.to_px(img.width(), img.height());
    Some(DecodedMouseCursorFrame {
        image: img,
        hotspot_x: hx,
        hotspot_y: hy,
    })
}

fn decode_seq(
    shp: &ShpFile,
    pal: &Palette,
    start: usize,
    len: usize,
    hotspot: MouseCursorHotspot,
) -> Option<Vec<DecodedMouseCursorFrame>> {
    if len == 0 || start >= shp.frames.len() {
        return None;
    }
    let end = (start + len).min(shp.frames.len());
    let mut out = Vec::with_capacity(end - start);
    for i in start..end {
        let img = frame_to_canvas_rgba(shp, &shp.frames[i], pal)?;
        let (hx, hy) = hotspot.to_px(img.width(), img.height());
        out.push(DecodedMouseCursorFrame {
            image: img,
            hotspot_x: hx,
            hotspot_y: hy,
        });
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn decode_dir_ring(
    shp: &ShpFile,
    pal: &Palette,
    start: usize,
) -> Option<[DecodedMouseCursorFrame; MOUSE_SCROLL_DIR_COUNT]> {
    let mut frames: Vec<DecodedMouseCursorFrame> = Vec::with_capacity(MOUSE_SCROLL_DIR_COUNT);
    for i in 0..MOUSE_SCROLL_DIR_COUNT {
        frames.push(decode_one(shp, pal, start + i, MOUSE_SCROLL_HOTSPOTS[i])?);
    }
    frames.try_into().ok()
}

fn decode_range(
    shp: &ShpFile,
    pal: &Palette,
    start: usize,
    len: usize,
) -> Option<Vec<RgbaImage>> {
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

#[cfg(test)]
mod hotspot_tests {
    use super::*;

    #[test]
    fn scroll_hotspot_center_top_on_even_canvas() {
        let (x, y) = MouseCursorHotspot::CenterTop.to_px(64, 48);
        assert_eq!((x, y), (32, 0));
    }

    #[test]
    fn scroll_ring_frame_offsets_match_table() {
        assert_eq!(MOUSE_SCROLL_START + 7, 9);
        assert_eq!(MOUSE_SCROLL_BLOCKED_START + 7, 17);
    }

    #[test]
    fn core_gameplay_frame_table() {
        assert_eq!(MOUSE_SELECT_START + MOUSE_SELECT_LEN - 1, 30);
        assert_eq!(MOUSE_MOVE_START + MOUSE_MOVE_LEN - 1, 40);
        assert_eq!(MOUSE_NO_MOVE_START, 41);
        assert_eq!(MOUSE_ATTACK_START + MOUSE_ATTACK_LEN - 1, 57);
        assert_eq!(MOUSE_NO_DEPLOY_START, 119);
    }
}
