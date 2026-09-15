//! 对局指针手势：点选 / 框选状态机（左键不再驱动相机平移）。
//!
//! 坐标约定：Battle 布局 / 命中 / 光标一律使用**逻辑像素**（与壳层菜单、`DisplayMode` 同口径）。
//! 物理表面尺寸仅用于 GPU scissor / 交换链。

use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::window::Window;

/// 左键位移小于该像素阈值时视为点选，否则进入框选。
pub const CLICK_SLOP_PX: f32 = 6.0;

/// 框选命中：步兵屏幕半宽/半高（格心锚点）。
pub const MARQUEE_HIT_HALF_INFANTRY_PX: f32 = 18.0;
/// 框选命中：载具 / 飞行器屏幕半宽/半高（覆盖 VXL 车身，避免只框车身选不中）。
pub const MARQUEE_HIT_HALF_VEHICLE_PX: f32 = 48.0;
/// 载具框选锚点相对格心上移（图像像素；VXL 主体在脚点上方）。
pub const MARQUEE_VEHICLE_LIFT_PX: f32 = 28.0;
/// 兼容旧名：默认步兵半宽。
pub const MARQUEE_HIT_HALF_PX: f32 = MARQUEE_HIT_HALF_INFANTRY_PX;

/// 战术区边缘滚屏触发带宽（窗口像素）。
pub const EDGE_SCROLL_MARGIN_PX: f32 = 16.0;

/// 边缘滚屏速度（屏幕像素 / 秒）。
pub const EDGE_SCROLL_SPEED_PX_PER_SEC: f32 = 640.0;

/// 方向键镜头平移速度（屏幕像素 / 秒）。与边缘滚屏同速；跟逻辑 tick / 系统按键重复无关。
pub const KEYBOARD_PAN_SPEED_PX_PER_SEC: f32 = EDGE_SCROLL_SPEED_PX_PER_SEC;

/// 对局表面度量：逻辑布局与物理表面拆分，命中与合成只读逻辑尺寸。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattleSurfaceMetrics {
    /// 逻辑客户区宽（与 `DisplayMode` / 菜单 `window_width` 同口径）。
    pub logical_width: u32,
    /// 逻辑客户区高。
    pub logical_height: u32,
    /// 物理表面宽（交换链 / `inner_size`）。
    pub physical_width: u32,
    /// 物理表面高。
    pub physical_height: u32,
    /// `winit` 缩放因子。
    pub scale_factor: f64,
}

impl BattleSurfaceMetrics {
    /// 由窗口当前物理尺寸与缩放因子构造。
    pub fn from_window(window: &Window) -> Self {
        let scale = window.scale_factor().max(0.0001);
        let physical = window.inner_size();
        let logical: LogicalSize<f64> = physical.to_logical(scale);
        Self {
            logical_width: logical.width.round().max(1.0) as u32,
            logical_height: logical.height.round().max(1.0) as u32,
            physical_width: physical.width.max(1),
            physical_height: physical.height.max(1),
            scale_factor: scale,
        }
    }

    /// 物理光标位置 → 逻辑像素（与菜单 `CursorMoved` 同转换）。
    pub fn cursor_from_physical(self, position: PhysicalPosition<f64>) -> (f64, f64) {
        let logical = position.to_logical::<f64>(self.scale_factor);
        (logical.x, logical.y)
    }

    /// 逻辑矩形 → 物理表面矩形（供 GPU `set_viewport` / scissor）。
    pub fn layout_rect_to_physical(self, x: u32, y: u32, w: u32, h: u32) -> (u32, u32, u32, u32) {
        let s = self.scale_factor;
        let px = (f64::from(x) * s).round().max(0.0) as u32;
        let py = (f64::from(y) * s).round().max(0.0) as u32;
        let mut pw = (f64::from(w) * s).round().max(1.0) as u32;
        let mut ph = (f64::from(h) * s).round().max(1.0) as u32;
        // 夹到物理表面，避免舍入越界。
        if px >= self.physical_width || py >= self.physical_height {
            return (0, 0, 1, 1);
        }
        pw = pw.min(self.physical_width.saturating_sub(px).max(1));
        ph = ph.min(self.physical_height.saturating_sub(py).max(1));
        (px, py, pw, ph)
    }
}

/// 对局互斥交互模式（工具态 / 命令条模式；Shift 排队等属修饰键，不进此枚举）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BattleInteractionMode {
    /// 常规点选 / 下令。
    #[default]
    Normal,
    /// 建造放置（建筑类型键）。
    PlaceBuilding {
        /// rules / art 类型 id。
        type_id: String,
    },
    /// 侧栏修理工具。
    Repair,
    /// 侧栏出售工具。
    Sell,
    /// 命令条路径点规划。
    Planning,
    /// 命令条攻击移动（下一次左键空地 / 敌方）。
    AttackMove,
    /// 跟随模式（左键点选目标）。
    Follow,
}

impl BattleInteractionMode {
    /// 当前是否为建造放置，并返回类型键。
    pub fn place_type_id(&self) -> Option<&str> {
        match self {
            Self::PlaceBuilding { type_id } => Some(type_id.as_str()),
            _ => None,
        }
    }

    /// 是否为修理工具。
    pub fn is_repair(&self) -> bool {
        matches!(self, Self::Repair)
    }

    /// 是否为出售工具。
    pub fn is_sell(&self) -> bool {
        matches!(self, Self::Sell)
    }

    /// 是否为路径规划。
    pub fn is_planning(&self) -> bool {
        matches!(self, Self::Planning)
    }

    /// 是否为攻击移动。
    pub fn is_attack_move(&self) -> bool {
        matches!(self, Self::AttackMove)
    }

    /// 是否为跟随。
    pub fn is_follow(&self) -> bool {
        matches!(self, Self::Follow)
    }

    /// 是否为非 `Normal` 工具 / 命令模式（右键可取消）。
    pub fn is_tool(&self) -> bool {
        !matches!(self, Self::Normal)
    }
}

/// 对局呈现快照：壳层只应用，不重新跑业务判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePresentationState {
    /// 当前应显示的指针。
    pub pointer: BattlePointer,
}

impl Default for BattlePresentationState {
    fn default() -> Self {
        Self { pointer: BattlePointer::Default }
    }
}

/// 方向键按住状态（由渲染帧 `dt` 推进镜头，不靠 OS key-repeat 跳格）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CameraPanKeys {
    /// 左方向键。
    pub left: bool,
    /// 右方向键。
    pub right: bool,
    /// 上方向键。
    pub up: bool,
    /// 下方向键。
    pub down: bool,
}

impl CameraPanKeys {
    /// 清空按住状态（失焦 / 暂停时调用）。
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// 是否有任一方向键按下。
    pub fn any(self) -> bool {
        self.left || self.right || self.up || self.down
    }
}

/// 由方向键按住状态计算本帧相机平移（屏幕像素，交给 `pan_world`）。
///
/// 左 → 正 `dx`（镜头左移），右 → 负 `dx`；上 → 正 `dy`，下 → 负 `dy`。
pub fn keyboard_pan_screen_delta(keys: CameraPanKeys, speed_px_per_sec: f32, dt_secs: f64) -> (f32, f32) {
    if dt_secs <= 0.0 || speed_px_per_sec <= 0.0 || !keys.any() {
        return (0.0, 0.0);
    }
    let step = (f64::from(speed_px_per_sec) * dt_secs) as f32;
    let mut dx = 0.0_f32;
    let mut dy = 0.0_f32;
    if keys.left {
        dx += step;
    }
    if keys.right {
        dx -= step;
    }
    if keys.up {
        dy += step;
    }
    if keys.down {
        dy -= step;
    }
    (dx, dy)
}

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

/// 对局指针优先级：边缘滚屏 > 工具光标 > 攻击 / 移动 / 部署上下文 > 点选 > 默认。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BattlePointer {
    /// 默认箭头（`mouse.shp` #0）。
    Default,
    /// 悬停可点选本方单位（`mouse.shp` Select）。
    Select,
    /// 选中单位可移动到光标格（Move）；表示**左键**将下令。
    Move,
    /// 选中单位不可到达光标格（NoMove）。
    NoMove,
    /// 选中单位可攻击光标下敌方（Attack）；表示**左键**将下令。
    Attack,
    /// 出售工具光标。
    Sell,
    /// 修理工具光标。
    Repair,
    /// 悬停已选可部署单位时的部署光标（西木：点单位 / `D` 即部署，无单独工具态）。
    Deploy,
    /// 已选可部署单位但当前落点不可部署（占位；地形闸未接前少用）。
    NoDeploy,
    /// 整窗边缘滚屏光标。
    Edge(EdgeScrollCursor),
}

impl BattlePointer {
    /// 边缘滚屏优先；否则按战术区悬停上下文。
    pub fn resolve(edge: EdgeScrollCursor, context: BattlePointer) -> Self {
        match edge {
            EdgeScrollCursor::Default => context,
            other => Self::Edge(other),
        }
    }
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
        matches!(self, Self::East | Self::West | Self::NorthEast | Self::NorthWest | Self::SouthEast | Self::SouthWest)
    }

    /// 该方向是否包含垂直分量。
    pub fn has_vertical(self) -> bool {
        matches!(self, Self::North | Self::South | Self::NorthEast | Self::NorthWest | Self::SouthEast | Self::SouthWest)
    }
}

/// 整窗边缘意图：右栏 / 底边命令条上同样有效（不只战术区）。
///
/// 返回 `(west, east, north, south)`。光标在窗外则全假。
pub fn edge_scroll_axes(cursor_x: f64, cursor_y: f64, surface_w: u32, surface_h: u32, margin_px: f32) -> (bool, bool, bool, bool) {
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
    if blocked { EdgeScrollCursor::Blocked(dir) } else { EdgeScrollCursor::Scroll(dir) }
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
        Self { x: x0, y: y0, w: (x1 - x0).max(0.0), h: (y1 - y0).max(0.0) }
    }

    /// 与另一矩形是否相交（边界相触也算）。
    pub fn intersects(&self, other: &ScreenRect) -> bool {
        self.x <= other.x + other.w && other.x <= self.x + self.w && self.y <= other.y + other.h && other.y <= self.y + self.h
    }

    /// 以中心点与半宽半高构造命中盒。
    pub fn from_center_half(cx: f32, cy: f32, half: f32) -> Self {
        let h = half.max(0.0);
        Self { x: cx - h, y: cy - h, w: h * 2.0, h: h * 2.0 }
    }

    /// 中心点是否落在矩形内（含边界）。
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    /// 点到矩形的最短距离（在内则为 0）。
    pub fn distance_to_point(&self, px: f32, py: f32) -> f32 {
        let cx = px.clamp(self.x, self.x + self.w);
        let cy = py.clamp(self.y, self.y + self.h);
        let dx = px - cx;
        let dy = py - cy;
        (dx * dx + dy * dy).sqrt()
    }
}

/// 左键手势状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeftGesture {
    /// 无左键手势。
    Idle,
    /// 已按下，位移尚未超过点选阈值。
    MaybeClick { origin_x: f64, origin_y: f64, distance: f32 },
    /// 已进入框选，拖拽期间更新终点。
    Marquee { origin_x: f64, origin_y: f64, cur_x: f64, cur_y: f64 },
}

impl LeftGesture {
    /// 在战术区内按下左键。
    pub fn begin(origin_x: f64, origin_y: f64) -> Self {
        Self::MaybeClick { origin_x, origin_y, distance: 0.0 }
    }

    /// 光标移动：相对按下点超阈值则进入框选。**不**平移相机。
    pub fn on_cursor_moved(self, x: f64, y: f64) -> Self {
        match self {
            Self::Idle => Self::Idle,
            Self::MaybeClick { origin_x, origin_y, .. } => {
                let dx = (x - origin_x) as f32;
                let dy = (y - origin_y) as f32;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist >= CLICK_SLOP_PX {
                    Self::Marquee { origin_x, origin_y, cur_x: x, cur_y: y }
                }
                else {
                    Self::MaybeClick { origin_x, origin_y, distance: dist }
                }
            }
            Self::Marquee { origin_x, origin_y, .. } => Self::Marquee { origin_x, origin_y, cur_x: x, cur_y: y },
        }
    }

    /// 当前框选矩形（仅 `Marquee`）。
    pub fn marquee_rect(&self) -> Option<ScreenRect> {
        match *self {
            Self::Marquee { origin_x, origin_y, cur_x, cur_y } => Some(ScreenRect::from_drag(origin_x, origin_y, cur_x, cur_y)),
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

/// 西木右键地图结果（HUD cameo 取消生产另计）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightClickMapOutcome {
    /// 关闭了放置 / 修理 / 出售等工具态，保留选中。
    CancelToolModes,
    /// 无工具态：应清空选中（不是 `order_stop`）。
    Deselect,
}

/// 由「是否仍有工具态」决定右键地图语义。
pub fn classify_right_click_map(tools_active: bool) -> RightClickMapOutcome {
    if tools_active { RightClickMapOutcome::CancelToolModes } else { RightClickMapOutcome::Deselect }
}

/// 已选机动单位时，友军点选是否允许「逻辑格邻域回退」。
///
/// 有选中时悬停空地 / 矿为 Move；若仍用邻格松散命中，点邻矿会重选矿车，左键采矿无反应。
pub fn allow_cell_neighbor_friendly_pick(has_mobile_selection: bool) -> bool {
    !has_mobile_selection
}

/// 已选机动单位时，是否允许友军 **图像软命中**（`pick_*_near_image`）。
///
/// 西木：已选单位时左键空地 / 矿 = 下令。矿车等载具软半径很大，点邻矿常仍摸到车身，
/// 若先走软命中再点选，表现就是「选中矿车左击矿没反应」。有机动选中时只认落点格。
pub fn allow_friendly_image_soft_pick(has_mobile_selection: bool) -> bool {
    !has_mobile_selection
}

/// 已选机动单位时：软命中友军，但落点格不是该单位所占格 → 应按 Move 下令，勿点选。
///
/// 兜底：若仍误走了软命中，与 Move 光标对齐，左键必须下发移动 / 采集。
pub fn friendly_soft_hit_should_order_not_reselect(
    has_mobile_selection: bool,
    click_cell: Option<(u16, u16)>,
    picked_cell: Option<(u16, u16)>,
) -> bool {
    if !has_mobile_selection {
        return false;
    }
    match (click_cell, picked_cell) {
        (Some(click), Some(picked)) => click != picked,
        _ => false,
    }
}

/// 左键下令修饰（Ctrl 强制攻击 / Alt 强制移动 / Shift 排队）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderClickModifier {
    /// 无修饰。
    None,
    /// Ctrl：强制攻击。
    ForceAttack,
    /// Alt：强制移动。
    ForceMove,
}

impl OrderClickModifier {
    /// Ctrl 优先于 Alt（同时按时按强制攻击）。
    pub fn from_keys(ctrl: bool, alt: bool) -> Self {
        if ctrl {
            Self::ForceAttack
        }
        else if alt {
            Self::ForceMove
        }
        else {
            Self::None
        }
    }
}

impl LeftGesture {
    /// 释放左键并清空手势，返回应执行的选择动作。
    pub fn release(self) -> (Self, LeftReleaseAction) {
        let action = match self {
            Self::Idle => LeftReleaseAction::None,
            Self::MaybeClick { .. } => LeftReleaseAction::Click,
            Self::Marquee { origin_x, origin_y, cur_x, cur_y } => {
                LeftReleaseAction::Marquee(ScreenRect::from_drag(origin_x, origin_y, cur_x, cur_y))
            }
        };
        (Self::Idle, action)
    }
}
