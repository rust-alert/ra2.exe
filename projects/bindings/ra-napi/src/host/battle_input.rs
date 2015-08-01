//! 对局指针手势：点选 / 框选状态机（左键不再驱动相机平移）。

/// 左键位移小于该像素阈值时视为点选，否则进入框选。
pub const CLICK_SLOP_PX: f32 = 6.0;

/// 框选命中用的实体屏幕半宽/半高（与标记环量级一致）。
pub const MARQUEE_HIT_HALF_PX: f32 = 12.0;

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
}
