//! 对局指针手势：点选 / 框选状态机（左键不再驱动相机平移）。

/// 左键位移小于该像素阈值时视为点选，否则进入框选。
pub const CLICK_SLOP_PX: f32 = 6.0;

/// 框选命中用的实体屏幕半宽/半高（与标记环量级一致）。
pub const MARQUEE_HIT_HALF_PX: f32 = 12.0;

/// 战术区边缘滚屏触发带宽（窗口像素）。
pub const EDGE_SCROLL_MARGIN_PX: f32 = 16.0;

/// 边缘滚屏速度（屏幕像素 / 秒）。
pub const EDGE_SCROLL_SPEED_PX_PER_SEC: f32 = 640.0;

/// 边缘滚屏方向（相对整窗；含对角）。上下左右光标不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeScrollDir {
    /// 不在边缘。
    None,
    /// 上。
    North,
    /// 下。
    South,
    /// 东（右）。
    East,
    /// 西（左）。
    West,
    /// 右上。
    NorthEast,
    /// 左上。
    NorthWest,
    /// 右下。
    SouthEast,
    /// 左下。
    SouthWest,
}

/// 边缘滚屏光标态：可滚或已贴地图边界。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeScrollCursor {
    /// 默认箭头。
    Default,
    /// 对应方向可滚。
    Scroll(EdgeScrollDir),
    /// 对应方向已顶到地图边界（禁止滚）。
    Blocked(EdgeScrollDir),
}

impl EdgeScrollDir {
    /// 由轴向意图合成方向（可对角）。
    pub fn from_axes(west: bool, east: bool, north: bool, south: bool) -> Self {
        match (west, east, north, south) {
            (true, false, true, false) => Self::NorthWest,
            (false, true, true, false) => Self::NorthEast,
            (true, false, false, true) => Self::SouthWest,
            (false, true, false, true) => Self::SouthEast,
            (true, false, false, false) => Self::West,
            (false, true, false, false) => Self::East,
            (false, false, true, false) => Self::North,
            (false, false, false, true) => Self::South,
            _ => Self::None,
        }
    }

    /// 该方向是否包含水平分量。
    pub fn has_horizontal(self) -> bool {
        matches!(
            self,
            Self::East | Self::West | Self::NorthEast | Self::NorthWest | Self::SouthEast | Self::SouthWest
        )
    }

    /// 该方向是否包含垂直分量。
    pub fn has_vertical(self) -> bool {
        matches!(
            self,
            Self::North | Self::South | Self::NorthEast | Self::NorthWest | Self::SouthEast | Self::SouthWest
        )
    }
}

/// 整窗边缘意图：右栏 / 底边命令条上同样有效（不只战术区）。
///
/// 返回 `(west, east, north, south)`。光标在窗外则全假。
pub fn edge_scroll_axes(
    cursor_x: f64,
    cursor_y: f64,
    surface_w: u32,
    surface_h: u32,
    margin_px: f32,
) -> (bool, bool, bool, bool) {
    let w = surface_w.max(1) as f32;
    let h = surface_h.max(1) as f32;
    let cx = cursor_x as f32;
    let cy = cursor_y as f32;
    if cx < 0.0 || cy < 0.0 || cx >= w || cy >= h {
        return (false, false, false, false);
    }
    let margin = margin_px.clamp(1.0, (w.min(h) * 0.45).max(1.0));
    let west = cx <= margin;
    let east = cx >= w - margin;
    let north = cy <= margin;
    let south = cy >= h - margin;
    (west, east, north, south)
}

/// 由整窗边缘与光标位置计算本帧相机平移（屏幕像素，交给 `pan_world`）。
///
/// 右栏与底边命令条上的边缘同样触发。靠近左边 → 正 `dx`（镜头左移），右边 → 负 `dx`。
pub fn edge_scroll_screen_delta(
    cursor_x: f64,
    cursor_y: f64,
    surface_w: u32,
    surface_h: u32,
    margin_px: f32,
    speed_px_per_sec: f32,
    dt_secs: f64,
) -> (f32, f32) {
    if surface_w == 0 || surface_h == 0 || dt_secs <= 0.0 || speed_px_per_sec <= 0.0 {
        return (0.0, 0.0);
    }
    let (west, east, north, south) = edge_scroll_axes(cursor_x, cursor_y, surface_w, surface_h, margin_px);
    let step = (speed_px_per_sec as f64 * dt_secs) as f32;
    let mut dx = 0.0_f32;
    let mut dy = 0.0_f32;
    if west {
        dx += step;
    }
    if east {
        dx -= step;
    }
    if north {
        dy += step;
    }
    if south {
        dy -= step;
    }
    (dx, dy)
}

/// 由边缘意图与「该轴是否还能平移」合成光标（方向不同；贴边则为禁止态）。
pub fn edge_scroll_cursor_for(
    west: bool,
    east: bool,
    north: bool,
    south: bool,
    can_scroll_west: bool,
    can_scroll_east: bool,
    can_scroll_north: bool,
    can_scroll_south: bool,
) -> EdgeScrollCursor {
    let dir = EdgeScrollDir::from_axes(west, east, north, south);
    if dir == EdgeScrollDir::None {
        return EdgeScrollCursor::Default;
    }
    let want_w = west && !can_scroll_west;
    let want_e = east && !can_scroll_east;
    let want_n = north && !can_scroll_north;
    let want_s = south && !can_scroll_south;
    // 所请求的每一轴都不可滚 → 禁止态；对角时任一可滚轴仍显示可滚方向。
    let blocked = match dir {
        EdgeScrollDir::West => want_w,
        EdgeScrollDir::East => want_e,
        EdgeScrollDir::North => want_n,
        EdgeScrollDir::South => want_s,
        EdgeScrollDir::NorthWest => want_w && want_n,
        EdgeScrollDir::NorthEast => want_e && want_n,
        EdgeScrollDir::SouthWest => want_w && want_s,
        EdgeScrollDir::SouthEast => want_e && want_s,
        EdgeScrollDir::None => false,
    };
    if blocked {
        EdgeScrollCursor::Blocked(dir)
    }
    else {
        EdgeScrollCursor::Scroll(dir)
    }
}

/// 屏幕轴对齐矩形（窗口像素，已归一化使 `w/h >= 0`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ScreenRect {
    /// 由拖拽起点与当前点构造归一化矩形。
    pub fn from_drag(origin_x: f64, origin_y: f64, cur_x: f64, cur_y: f64) -> Self {
        let x0 = origin_x.min(cur_x) as f32;
        let y0 = origin_y.min(cur_y) as f32;
        let x1 = origin_x.max(cur_x) as f32;
        let y1 = origin_y.max(cur_y) as f32;
        Self {
            x: x0,
            y: y0,
            w: (x1 - x0).max(0.0),
            h: (y1 - y0).max(0.0),
        }
    }

    /// 与另一矩形是否相交（边界相触也算）。
    pub fn intersects(&self, other: &ScreenRect) -> bool {
        self.x <= other.x + other.w
            && other.x <= self.x + self.w
            && self.y <= other.y + other.h
            && other.y <= self.y + self.h
    }

    /// 以中心点与半宽半高构造命中盒。
    pub fn from_center_half(cx: f32, cy: f32, half: f32) -> Self {
        let h = half.max(0.0);
        Self {
            x: cx - h,
            y: cy - h,
            w: h * 2.0,
            h: h * 2.0,
        }
    }
}

/// 左键手势状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeftGesture {
    /// 无左键手势。
    Idle,
    /// 已按下，位移尚未超过点选阈值。
    MaybeClick {
        origin_x: f64,
        origin_y: f64,
        distance: f32,
    },
    /// 已进入框选，拖拽期间更新终点。
    Marquee {
        origin_x: f64,
        origin_y: f64,
        cur_x: f64,
        cur_y: f64,
    },
}

impl LeftGesture {
    /// 在战术区内按下左键。
    pub fn begin(origin_x: f64, origin_y: f64) -> Self {
        Self::MaybeClick {
            origin_x,
            origin_y,
            distance: 0.0,
        }
    }

    /// 光标移动：相对按下点超阈值则进入框选。**不**平移相机。
    pub fn on_cursor_moved(self, x: f64, y: f64) -> Self {
        match self {
            Self::Idle => Self::Idle,
            Self::MaybeClick {
                origin_x,
                origin_y,
                ..
            } => {
                let dx = (x - origin_x) as f32;
                let dy = (y - origin_y) as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist >= CLICK_SLOP_PX {
                    Self::Marquee {
                        origin_x,
                        origin_y,
                        cur_x: x,
                        cur_y: y,
                    }
                }
                else {
                    Self::MaybeClick {
                        origin_x,
                        origin_y,
                        distance: dist,
                    }
                }
            }
            Self::Marquee {
                origin_x,
                origin_y,
                ..
            } => Self::Marquee {
                origin_x,
                origin_y,
                cur_x: x,
                cur_y: y,
            },
        }
    }

    /// 当前框选矩形（仅 `Marquee`）。
    pub fn marquee_rect(&self) -> Option<ScreenRect> {
        match *self {
            Self::Marquee {
                origin_x,
                origin_y,
                cur_x,
                cur_y,
            } => Some(ScreenRect::from_drag(origin_x, origin_y, cur_x, cur_y)),
            _ => None,
        }
    }
}

/// 左键释放后的语义。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeftReleaseAction {
    /// 无操作（未武装）。
    None,
    /// 点选（小位移）。
    Click,
    /// 框选（大位移）。
    Marquee(ScreenRect),
}

impl LeftGesture {
    /// 释放左键并清空手势，返回应执行的选择动作。
    pub fn release(self) -> (Self, LeftReleaseAction) {
        let action = match self {
            Self::Idle => LeftReleaseAction::None,
            Self::MaybeClick { .. } => LeftReleaseAction::Click,
            Self::Marquee {
                origin_x,
                origin_y,
                cur_x,
                cur_y,
            } => LeftReleaseAction::Marquee(ScreenRect::from_drag(origin_x, origin_y, cur_x, cur_y)),
        };
        (Self::Idle, action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_move_stays_maybe_click_and_releases_as_click() {
        let g = LeftGesture::begin(100.0, 100.0);
        let g = g.on_cursor_moved(103.0, 102.0);
        assert!(matches!(g, LeftGesture::MaybeClick { .. }));
        let (idle, action) = g.release();
        assert_eq!(idle, LeftGesture::Idle);
        assert_eq!(action, LeftReleaseAction::Click);
    }

    #[test]
    fn large_move_enters_marquee_and_does_not_need_pan() {
        let g = LeftGesture::begin(10.0, 10.0);
        let g = g.on_cursor_moved(30.0, 40.0);
        match g {
            LeftGesture::Marquee {
                origin_x,
                origin_y,
                cur_x,
                cur_y,
            } => {
                assert_eq!((origin_x, origin_y), (10.0, 10.0));
                assert_eq!((cur_x, cur_y), (30.0, 40.0));
            }
            other => panic!("expected Marquee, got {other:?}"),
        }
        let (_, action) = g.release();
        assert_eq!(
            action,
            LeftReleaseAction::Marquee(ScreenRect {
                x: 10.0,
                y: 10.0,
                w: 20.0,
                h: 30.0
            })
        );
    }

    #[test]
    fn screen_rect_intersects_entity_hitbox() {
        let drag = ScreenRect::from_drag(0.0, 0.0, 50.0, 50.0);
        let hit = ScreenRect::from_center_half(40.0, 40.0, MARQUEE_HIT_HALF_PX);
        assert!(drag.intersects(&hit));
        let miss = ScreenRect::from_center_half(200.0, 200.0, MARQUEE_HIT_HALF_PX);
        assert!(!drag.intersects(&miss));
    }

    #[test]
    fn edge_scroll_left_and_right_oppose() {
        let (dx_l, dy_l) = edge_scroll_screen_delta(5.0, 100.0, 800, 600, 16.0, 640.0, 0.1);
        assert!(dx_l > 0.0);
        assert_eq!(dy_l, 0.0);
        let (dx_r, _) = edge_scroll_screen_delta(790.0, 100.0, 800, 600, 16.0, 640.0, 0.1);
        assert!(dx_r < 0.0);
        let (dx_mid, dy_mid) = edge_scroll_screen_delta(400.0, 300.0, 800, 600, 16.0, 640.0, 0.1);
        assert_eq!((dx_mid, dy_mid), (0.0, 0.0));
    }

    #[test]
    fn edge_scroll_works_on_sidebar_and_command_bar() {
        // 右栏内侧靠窗右缘：应向东滚。
        let (dx, dy) = edge_scroll_screen_delta(1270.0, 200.0, 1280, 720, 16.0, 640.0, 0.1);
        assert!(dx < 0.0, "sidebar right edge must scroll");
        assert_eq!(dy, 0.0);
        // 底边命令条：应向南滚。
        let (dx2, dy2) = edge_scroll_screen_delta(400.0, 710.0, 1280, 720, 16.0, 640.0, 0.1);
        assert_eq!(dx2, 0.0);
        assert!(dy2 < 0.0, "command bar bottom edge must scroll");
    }

    #[test]
    fn edge_scroll_ignores_cursor_outside_window() {
        let (dx, dy) = edge_scroll_screen_delta(900.0, 100.0, 800, 600, 16.0, 640.0, 0.1);
        assert_eq!((dx, dy), (0.0, 0.0));
    }

    #[test]
    fn south_blocked_when_cannot_scroll_south() {
        let cur = edge_scroll_cursor_for(false, false, false, true, true, true, true, false);
        assert_eq!(cur, EdgeScrollCursor::Blocked(EdgeScrollDir::South));
        let cur_ok = edge_scroll_cursor_for(false, false, false, true, true, true, true, true);
        assert_eq!(cur_ok, EdgeScrollCursor::Scroll(EdgeScrollDir::South));
    }
}
