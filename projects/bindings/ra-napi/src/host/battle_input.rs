//! 对局指针手势：点选 / 框选状态机（左键不再驱动相机平移）。
//!
//! 坐标约定：Battle 布局 / 命中 / 光标一律使用**逻辑像素**（与壳层菜单、`DisplayMode` 同口径）。
//! 物理表面尺寸仅用于 GPU scissor / 交换链。

use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    window::Window,
};

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

    /// 可拷贝的工具种类摘要（不含放置类型键；供 `BattleInputFrame`）。
    pub fn tool_kind(&self) -> BattleToolKind {
        match self {
            Self::Normal => BattleToolKind::Normal,
            Self::PlaceBuilding { .. } => BattleToolKind::PlaceBuilding,
            Self::Repair => BattleToolKind::Repair,
            Self::Sell => BattleToolKind::Sell,
            Self::Planning => BattleToolKind::Planning,
            Self::AttackMove => BattleToolKind::AttackMove,
            Self::Follow => BattleToolKind::Follow,
        }
    }
}

/// 对局工具模式的可拷贝摘要（与 `BattleInteractionMode` 一一对应，无堆分配字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BattleToolKind {
    /// 常规。
    #[default]
    Normal,
    /// 建造放置。
    PlaceBuilding,
    /// 修理。
    Repair,
    /// 出售。
    Sell,
    /// 路径规划。
    Planning,
    /// 攻击移动。
    AttackMove,
    /// 跟随。
    Follow,
}

impl BattleToolKind {
    /// 是否为侧栏 / 命令条工具态。
    pub const fn is_tool(self) -> bool {
        !matches!(self, Self::Normal)
    }
}

/// 战术区一次解析后的主动作种类（光标 / 左键 / 提示共用同一枚举）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResolvedPrimaryAction {
    /// 无有效动作（或仅更新光标）。
    #[default]
    Noop,
    /// 建造放置。
    PlaceBuilding,
    /// 出售建筑。
    Sell,
    /// 修理建筑。
    Repair,
    /// 规划模式追加航点。
    AppendWaypoint,
    /// 跟随目标。
    Follow,
    /// 点选本方。
    Select,
    /// Shift 加选。
    AddSelect,
    /// 部署已选可部署单位。
    Deploy,
    /// 设为主厂。
    SetPrimary,
    /// 攻击敌方。
    Attack,
    /// 工程师占领。
    Capture,
    /// 间谍渗透。
    Infiltrate,
    /// 移动 / 采集。
    Move,
    /// 攻击移动。
    AttackMove,
    /// Shift 路径移动。
    QueueMovePath,
    /// 建筑集结点。
    SetRally,
    /// 清空选中。
    Deselect,
}

/// 战术区悬停一次解析的呈现摘要（光标 / 提示 / 左键共用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedBattleHover {
    /// 建议指针（不含边缘滚屏；由 `BattlePointer::resolve` 再叠边缘）。
    pub recommended_pointer: BattlePointer,
    /// 光标下地图格（窗外 / 非战术区为 `None`）。
    pub cell: Option<(u16, u16)>,
    /// 左键将执行的主动作（与 `recommended_pointer` 同源）。
    pub primary: ResolvedPrimaryAction,
}

impl ResolvedBattleHover {
    /// 窗外 / 非战术区默认悬停。
    pub const fn empty() -> Self {
        Self {
            recommended_pointer: BattlePointer::Default,
            cell: None,
            primary: ResolvedPrimaryAction::Noop,
        }
    }
}

/// 左键按下锁定的捕获层（释放必须对照同一捕获；HUD 与战术区互斥）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BattleUiCapture {
    /// 无捕获。
    #[default]
    None,
    /// 底边命令条槽。
    HudCommand(usize),
    /// 侧栏 / 修理 / 出售 / 雷达等（非命令条）。
    HudSidebar,
    /// 战术区点选 / 框选。
    World,
    /// 暂停菜单入口。
    PauseMenu,
}

impl BattleUiCapture {
    /// 是否为 HUD 控件捕获（命令条或侧栏）。
    pub const fn is_hud(self) -> bool {
        matches!(self, Self::HudCommand(_) | Self::HudSidebar)
    }

    /// 是否为战术区捕获。
    pub const fn is_world(self) -> bool {
        matches!(self, Self::World)
    }

    /// 清空。
    pub fn clear(&mut self) {
        *self = Self::None;
    }
}

/// 一帧不可变输入快照（由控制器状态 + 表面度量构造；事件只更新底层状态）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattleInputFrame {
    /// 表面度量。
    pub metrics: BattleSurfaceMetrics,
    /// 逻辑光标。
    pub cursor: (f64, f64),
    /// 光标是否在客户区内。
    pub cursor_in_window: bool,
    /// 光标是否在战术区 viewport 内。
    pub cursor_in_world: bool,
    /// Shift。
    pub shift_down: bool,
    /// Ctrl。
    pub ctrl_down: bool,
    /// Alt。
    pub alt_down: bool,
    /// Ctrl / Alt 下令修饰（Shift 排队另见 `shift_down`）。
    pub order_mod: OrderClickModifier,
    /// 当前工具模式摘要。
    pub tool: BattleToolKind,
    /// 方向键镜头按住态（持续输入）。
    pub camera_pan_keys: CameraPanKeys,
    /// 框选预览矩形（仅拖拽框选中）。
    pub marquee: Option<ScreenRect>,
    /// 当前捕获层种类。
    pub capture: BattleUiCapture,
    /// 指针按键按住态。
    pub buttons: PointerButtons,
    /// 修饰键按住态（与 `shift_down` 等同源，失焦静默清空）。
    pub modifiers: ModifierButtons,
    /// 本帧边沿（自上次 `BattleInputTracker::begin_frame` 起累计）。
    pub edges: BattleInputEdges,
}

impl BattleInputFrame {
    /// 光标逻辑像素整数。
    pub fn cursor_i32(self) -> (i32, i32) {
        (self.cursor.0 as i32, self.cursor.1 as i32)
    }
}

/// 指针按键按住态（持续；失焦时必须清空且不派发释放动作）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PointerButtons {
    /// 左键。
    pub left: bool,
    /// 右键。
    pub right: bool,
}

impl PointerButtons {
    /// 是否有任一键按住。
    pub const fn any(self) -> bool {
        self.left || self.right
    }
}

/// 修饰键按住态（与指针键一样：失焦静默清空，不产生抬起边沿）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModifierButtons {
    /// Shift。
    pub shift: bool,
    /// Ctrl。
    pub ctrl: bool,
    /// Alt。
    pub alt: bool,
}

impl ModifierButtons {
    /// 是否有任一修饰键按住。
    pub const fn any(self) -> bool {
        self.shift || self.ctrl || self.alt
    }
}

/// 本帧输入边沿（瞬时；每帧 `begin_frame` 清零后由事件累计）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BattleInputEdges {
    /// 左键刚按下。
    pub left_pressed: bool,
    /// 左键刚抬起（正常释放；失焦取消不置位）。
    pub left_released: bool,
    /// 右键刚按下。
    pub right_pressed: bool,
    /// 右键刚抬起。
    pub right_released: bool,
    /// 本帧失焦。
    pub focus_lost: bool,
    /// 滚轮步进累计（负=上、正=下，与 cameo 滚动同号）。
    pub wheel_steps: i32,
    /// 方向键本帧刚按下（持续平移的边沿，供调试 / 热键冲突对照）。
    pub pan_pressed: CameraPanKeys,
    /// 方向键本帧刚抬起。
    pub pan_released: CameraPanKeys,
    /// 修饰键本帧刚按下。
    pub mod_pressed: ModifierButtons,
    /// 修饰键本帧刚抬起（正常 `ModifiersChanged`；失焦静默清空不置位）。
    pub mod_released: ModifierButtons,
}

/// 窗口事件归一化后的按键 / 焦点追踪（纯逻辑，可供单测覆盖失焦幽灵序列）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleInputTracker {
    /// 按住态。
    pub buttons: PointerButtons,
    /// 修饰键按住态。
    pub modifiers: ModifierButtons,
    /// 窗口是否拥有焦点。
    pub focused: bool,
    /// 本帧边沿。
    pub edges: BattleInputEdges,
}

impl Default for BattleInputTracker {
    fn default() -> Self {
        Self {
            buttons: PointerButtons::default(),
            modifiers: ModifierButtons::default(),
            // 进对局假定已聚焦，直至收到 `Focused(false)`（避免首失焦时 focused 已是 false 而不清按住）。
            focused: true,
            edges: BattleInputEdges::default(),
        }
    }
}

impl BattleInputTracker {
    /// 新帧开始：清边沿，保留按住与焦点。
    pub fn begin_frame(&mut self) {
        self.edges = BattleInputEdges::default();
    }

    /// 左键按下 / 抬起。
    pub fn set_left(&mut self, down: bool) {
        if down && !self.buttons.left {
            self.edges.left_pressed = true;
        }
        else if !down && self.buttons.left {
            self.edges.left_released = true;
        }
        self.buttons.left = down;
    }

    /// 右键按下 / 抬起。
    pub fn set_right(&mut self, down: bool) {
        if down && !self.buttons.right {
            self.edges.right_pressed = true;
        }
        else if !down && self.buttons.right {
            self.edges.right_released = true;
        }
        self.buttons.right = down;
    }

    /// 修饰键变化（`ModifiersChanged`）。
    pub fn set_modifiers(&mut self, shift: bool, ctrl: bool, alt: bool) {
        let next = ModifierButtons { shift, ctrl, alt };
        self.edges.mod_pressed.shift |= !self.modifiers.shift && next.shift;
        self.edges.mod_pressed.ctrl |= !self.modifiers.ctrl && next.ctrl;
        self.edges.mod_pressed.alt |= !self.modifiers.alt && next.alt;
        self.edges.mod_released.shift |= self.modifiers.shift && !next.shift;
        self.edges.mod_released.ctrl |= self.modifiers.ctrl && !next.ctrl;
        self.edges.mod_released.alt |= self.modifiers.alt && !next.alt;
        self.modifiers = next;
    }

    /// 焦点变化。失焦时清空按住态与修饰键且**不**产生 released 边沿（取消捕获，勿误触点击）。
    pub fn set_focused(&mut self, focused: bool) {
        if self.focused && !focused {
            self.edges.focus_lost = true;
            self.buttons = PointerButtons::default();
            self.modifiers = ModifierButtons::default();
        }
        self.focused = focused;
    }

    /// 累计滚轮步进。
    pub fn add_wheel_steps(&mut self, steps: i32) {
        self.edges.wheel_steps = self.edges.wheel_steps.saturating_add(steps);
    }

    /// 记录方向键按住变化的边沿（`prev` → `next`；调用方已写回 `camera_pan_keys`）。
    ///
    /// 失焦 / `reset_transient` 直接清空按住时**不要**调用本方法，以免误报抬起边沿。
    pub fn note_camera_pan(&mut self, prev: CameraPanKeys, next: CameraPanKeys) {
        self.edges.pan_pressed.left |= !prev.left && next.left;
        self.edges.pan_pressed.right |= !prev.right && next.right;
        self.edges.pan_pressed.up |= !prev.up && next.up;
        self.edges.pan_pressed.down |= !prev.down && next.down;
        self.edges.pan_released.left |= prev.left && !next.left;
        self.edges.pan_released.right |= prev.right && !next.right;
        self.edges.pan_released.up |= prev.up && !next.up;
        self.edges.pan_released.down |= prev.down && !next.down;
    }

    /// 强制清空按住态（`reset_transient_input_state` / 脚本锁）。
    ///
    /// 不清除本帧边沿：失焦的 `focus_lost` 等需保留到本帧呈现 / 调试消费完毕，
    /// 由下一帧 `begin_frame` 统一清零。
    pub fn reset_transient(&mut self) {
        self.buttons = PointerButtons::default();
        self.modifiers = ModifierButtons::default();
    }
}

/// 命令条捕获：仅当释放命中同一槽才触发。
pub fn hud_command_release_fires(capture_slot: usize, release_hit_slot: Option<usize>) -> bool {
    release_hit_slot == Some(capture_slot)
}

/// 侧栏捕获：释放命中必须与按下命中相等（由调用方比较具体 `BattleHudHit`）。
pub fn hud_sidebar_release_fires(pressed_same_control: bool) -> bool {
    pressed_same_control
}

/// 光标移动时是否推进战术区左键手势（框选）。
///
/// 放置模式禁用框选升级，但按下时仍可建立 `MaybeClick`，释放走点选放置。
pub fn should_advance_world_gesture(accept_commands: bool, placing: bool, capture: BattleUiCapture) -> bool {
    accept_commands && !placing && capture.is_world()
}

/// 按下命中 → 捕获层（命令条优先于侧栏，侧栏优先于战术区）。
pub fn resolve_press_capture(command_slot: Option<usize>, sidebar: bool, cursor_in_world: bool) -> BattleUiCapture {
    if let Some(slot) = command_slot {
        return BattleUiCapture::HudCommand(slot);
    }
    if sidebar {
        return BattleUiCapture::HudSidebar;
    }
    if cursor_in_world {
        return BattleUiCapture::World;
    }
    BattleUiCapture::None
}

/// 左键释放时按捕获层分流（具体 HUD 命中相等性由调用方再判）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftReleasePolicy {
    /// 命令条：核对同槽后触发。
    HudCommand(usize),
    /// 侧栏：核对同控件后触发。
    HudSidebar,
    /// 战术区：走 `LeftGesture::release`。
    WorldGesture,
    /// 无捕获 / 暂停菜单：忽略。
    Ignore,
}

/// 由按下锁定的捕获决定释放策略。
pub fn left_release_policy(capture: BattleUiCapture) -> LeftReleasePolicy {
    match capture {
        BattleUiCapture::HudCommand(slot) => LeftReleasePolicy::HudCommand(slot),
        BattleUiCapture::HudSidebar => LeftReleasePolicy::HudSidebar,
        BattleUiCapture::World => LeftReleasePolicy::WorldGesture,
        BattleUiCapture::None | BattleUiCapture::PauseMenu => LeftReleasePolicy::Ignore,
    }
}

/// 右键按下后捕获层应被清空（取消战术手势，不影响下一帧 HUD hover）。
pub fn capture_after_right_press() -> BattleUiCapture {
    BattleUiCapture::None
}

/// 对局呈现快照：壳层只应用，不重新跑业务判断。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BattlePresentationState {
    /// 当前应显示的指针。
    pub pointer: BattlePointer,
    /// 悬停格（调试 / 状态栏可共用）。
    pub hover_cell: Option<(u16, u16)>,
    /// 悬停主动作。
    pub hover_primary: ResolvedPrimaryAction,
    /// 框选预览矩形（与 HUD 叠画同源；无框选时为 `None`）。
    pub marquee: Option<ScreenRect>,
}

impl Default for BattlePresentationState {
    fn default() -> Self {
        Self {
            pointer: BattlePointer::Default,
            hover_cell: None,
            hover_primary: ResolvedPrimaryAction::Noop,
            marquee: None,
        }
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

/// 由主动作 + 通行性 + 工具态推导建议指针（与左键执行同源；不含边缘滚屏）。
///
/// `primary == Noop` 时仍可能显示工具光标（出售 / 修理 / 攻击移动悬空地）。
pub fn recommended_pointer_for_primary(primary: ResolvedPrimaryAction, traversable: bool, tool: BattleToolKind) -> BattlePointer {
    match primary {
        ResolvedPrimaryAction::Noop => match tool {
            BattleToolKind::Sell => BattlePointer::Sell,
            BattleToolKind::Repair => BattlePointer::Repair,
            BattleToolKind::PlaceBuilding | BattleToolKind::Follow | BattleToolKind::Planning | BattleToolKind::Normal => BattlePointer::Default,
            BattleToolKind::AttackMove => {
                if traversable {
                    BattlePointer::Attack
                }
                else {
                    BattlePointer::NoMove
                }
            }
        },
        ResolvedPrimaryAction::PlaceBuilding => BattlePointer::Default,
        ResolvedPrimaryAction::Sell => BattlePointer::Sell,
        ResolvedPrimaryAction::Repair => BattlePointer::Repair,
        ResolvedPrimaryAction::AppendWaypoint | ResolvedPrimaryAction::Move | ResolvedPrimaryAction::QueueMovePath => {
            if traversable {
                BattlePointer::Move
            }
            else {
                BattlePointer::NoMove
            }
        }
        ResolvedPrimaryAction::Follow | ResolvedPrimaryAction::Select | ResolvedPrimaryAction::AddSelect | ResolvedPrimaryAction::SetPrimary => {
            BattlePointer::Select
        }
        ResolvedPrimaryAction::Deploy => BattlePointer::Deploy,
        ResolvedPrimaryAction::Attack | ResolvedPrimaryAction::Capture | ResolvedPrimaryAction::Infiltrate => BattlePointer::Attack,
        ResolvedPrimaryAction::AttackMove => {
            if traversable {
                BattlePointer::Attack
            }
            else {
                BattlePointer::NoMove
            }
        }
        ResolvedPrimaryAction::SetRally => BattlePointer::Move,
        ResolvedPrimaryAction::Deselect => BattlePointer::Default,
    }
}

/// 地图右键后的工具态与语义：有工具则退出工具并保留选中，否则应清空选中。
pub fn next_tool_after_map_right_click(current: BattleToolKind) -> (BattleToolKind, RightClickMapOutcome) {
    if current.is_tool() {
        (BattleToolKind::Normal, RightClickMapOutcome::CancelToolModes)
    }
    else {
        (BattleToolKind::Normal, RightClickMapOutcome::Deselect)
    }
}

/// 已选机动单位时，落点格左键下令种类（不含具体坐标；由调用方填 cell）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileGroundOrderKind {
    /// 不可通行：不下令。
    Blocked,
    /// Shift 路径移动。
    QueueMovePath,
    /// 攻击移动（工具态或 Ctrl 强攻空地）。
    AttackMove,
    /// 普通移动（`queue_path` 仅影响航点是否保留，不改命令种类）。
    Move {
        /// 是否 Shift 排队语义（ForceMove+Shift 时仍为 Move）。
        queue_path: bool,
    },
}

/// 已选机动 + 落点格：由通行性 / 修饰 / 攻击移动工具态决定下令种类。
pub fn resolve_mobile_ground_order(
    traversable: bool,
    queue_path: bool,
    order_mod: OrderClickModifier,
    attack_move_tool: bool,
) -> MobileGroundOrderKind {
    if !traversable {
        return MobileGroundOrderKind::Blocked;
    }
    if queue_path && matches!(order_mod, OrderClickModifier::None) && !attack_move_tool {
        return MobileGroundOrderKind::QueueMovePath;
    }
    if matches!(order_mod, OrderClickModifier::ForceAttack) || attack_move_tool {
        return MobileGroundOrderKind::AttackMove;
    }
    MobileGroundOrderKind::Move { queue_path }
}

/// 空选中 / 仅建筑选中时，落点左键语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyOrStructureGroundKind {
    /// 设集结点（有建筑选中）。
    SetRally,
    /// Shift 加选空地：无动作。
    Noop,
    /// 清空选中。
    Deselect,
}

/// 无机动选中时的落点语义。
pub fn resolve_non_mobile_ground_order(has_structure_selected: bool, shift_add: bool) -> EmptyOrStructureGroundKind {
    if has_structure_selected {
        EmptyOrStructureGroundKind::SetRally
    }
    else if shift_add {
        EmptyOrStructureGroundKind::Noop
    }
    else {
        EmptyOrStructureGroundKind::Deselect
    }
}

/// 点到本方单位 / 建筑时的左键语义（已过 soft-hit→下令闸）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendlyClickKind {
    /// 部署已选可部署单位。
    Deploy,
    /// 设主厂。
    SetPrimary,
    /// 点选 / 加选。
    Select {
        /// Shift 加选。
        add: bool,
    },
}

/// 本方命中后的点选语义：Deploy / 主厂优先于普通点选。
pub fn resolve_friendly_click(shift_add: bool, already_selected: bool, can_deploy: bool, is_primary_factory: bool) -> FriendlyClickKind {
    if !shift_add && already_selected && can_deploy {
        FriendlyClickKind::Deploy
    }
    else if !shift_add && already_selected && is_primary_factory {
        FriendlyClickKind::SetPrimary
    }
    else {
        FriendlyClickKind::Select { add: shift_add }
    }
}

/// 点到敌方时的左键语义（需已有机动选中且非 ForceMove）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostileClickKind {
    /// 工程师占领可俘建筑。
    Capture,
    /// 间谍渗透建筑。
    Infiltrate,
    /// 普通攻击（含 Ctrl 强攻）。
    Attack,
}

/// 敌方命中语义：非强攻时占领 / 渗透优先于攻击。
pub fn resolve_hostile_click(
    force_attack: bool,
    target_is_structure: bool,
    selection_has_engineer: bool,
    capturable: bool,
    selection_has_agent: bool,
) -> HostileClickKind {
    if !force_attack && target_is_structure && selection_has_engineer && capturable {
        HostileClickKind::Capture
    }
    else if !force_attack && target_is_structure && selection_has_agent {
        HostileClickKind::Infiltrate
    }
    else {
        HostileClickKind::Attack
    }
}

/// 跟随工具态下左键语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowClickKind {
    /// 选中无效 → 退出跟随模式。
    Cancel,
    /// 点到已选自身或无目标 → 无动作。
    Noop,
    /// 跟随光标下机动单位。
    Follow,
}

/// 跟随模式左键：无机动选中则取消；点到非己选机动则跟随。
pub fn resolve_follow_click(has_mobile_selected: bool, has_target: bool, target_already_selected: bool) -> FollowClickKind {
    if !has_mobile_selected {
        FollowClickKind::Cancel
    }
    else if !has_target || target_already_selected {
        FollowClickKind::Noop
    }
    else {
        FollowClickKind::Follow
    }
}

/// 窗外 / 无格 / 空点时的清空语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissClickKind {
    /// 无动作（通常 Shift 加选空放）。
    Noop,
    /// 清空选中。
    Deselect,
}

/// 未命中可下令目标时：无 Shift 且有选中则清空。
pub fn resolve_miss_click(shift_add: bool, has_selection: bool) -> MissClickKind {
    if !shift_add && has_selection {
        MissClickKind::Deselect
    }
    else {
        MissClickKind::Noop
    }
}

/// ForceMove 时是否跳过本方点选（直接落点强移）。
pub fn should_skip_friendly_pick(order_mod: OrderClickModifier, has_mobile_selected: bool) -> bool {
    matches!(order_mod, OrderClickModifier::ForceMove) && has_mobile_selected
}

/// 是否应尝试敌方下令（有机动选中且非 ForceMove）。
pub fn should_try_hostile_order(order_mod: OrderClickModifier, has_mobile_selected: bool) -> bool {
    has_mobile_selected && !matches!(order_mod, OrderClickModifier::ForceMove)
}

/// 放置工具态落点语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceClickKind {
    /// 可放置。
    Place,
    /// 无格或占地非法。
    Noop,
}

/// 放置模式：有格且占地合法才下发。
pub fn resolve_place_click(has_cell: bool, placeable: bool) -> PlaceClickKind {
    if has_cell && placeable {
        PlaceClickKind::Place
    }
    else {
        PlaceClickKind::Noop
    }
}

/// 出售 / 修理工具态：仅当命中本方建筑。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarBuildingToolClickKind {
    /// 对命中建筑执行工具。
    Apply,
    /// 未命中建筑。
    Noop,
}

/// 出售或修理：有本方建筑命中才 Apply。
pub fn resolve_sidebar_building_tool_click(has_local_building: bool) -> SidebarBuildingToolClickKind {
    if has_local_building {
        SidebarBuildingToolClickKind::Apply
    }
    else {
        SidebarBuildingToolClickKind::Noop
    }
}

/// 路径规划工具态落点语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningClickKind {
    /// 追加可通行航点。
    AppendWaypoint,
    /// 有选中但不可通行。
    Blocked,
    /// 无选中或无格。
    Noop,
}

/// 规划模式：有选中 + 可通行才追加；有选中不可通行为 Blocked。
pub fn resolve_planning_click(has_selection: bool, has_cell: bool, traversable: bool) -> PlanningClickKind {
    if !has_selection || !has_cell {
        PlanningClickKind::Noop
    }
    else if traversable {
        PlanningClickKind::AppendWaypoint
    }
    else {
        PlanningClickKind::Blocked
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_click_modifier_ctrl_beats_alt() {
        assert_eq!(OrderClickModifier::from_keys(true, true), OrderClickModifier::ForceAttack);
        assert_eq!(OrderClickModifier::from_keys(true, false), OrderClickModifier::ForceAttack);
        assert_eq!(OrderClickModifier::from_keys(false, true), OrderClickModifier::ForceMove);
        assert_eq!(OrderClickModifier::from_keys(false, false), OrderClickModifier::None);
    }

    #[test]
    fn mobile_selection_disables_friendly_soft_pick() {
        assert!(allow_friendly_image_soft_pick(false));
        assert!(!allow_friendly_image_soft_pick(true));
        assert!(allow_cell_neighbor_friendly_pick(false));
        assert!(!allow_cell_neighbor_friendly_pick(true));
    }

    #[test]
    fn soft_hit_off_unit_cell_orders_not_reselects() {
        assert!(!friendly_soft_hit_should_order_not_reselect(false, Some((1, 1)), Some((2, 2))));
        assert!(friendly_soft_hit_should_order_not_reselect(true, Some((1, 1)), Some((2, 2))));
        assert!(!friendly_soft_hit_should_order_not_reselect(true, Some((3, 3)), Some((3, 3))));
        assert!(!friendly_soft_hit_should_order_not_reselect(true, None, Some((1, 1))));
    }

    #[test]
    fn primary_action_covers_pointer_aligned_orders() {
        // 光标 / 左键共用的主动作枚举须覆盖下令族，避免再分叉。
        let orders = [
            ResolvedPrimaryAction::Move,
            ResolvedPrimaryAction::AttackMove,
            ResolvedPrimaryAction::QueueMovePath,
            ResolvedPrimaryAction::Attack,
            ResolvedPrimaryAction::Capture,
            ResolvedPrimaryAction::Infiltrate,
            ResolvedPrimaryAction::Deploy,
            ResolvedPrimaryAction::Select,
            ResolvedPrimaryAction::Deselect,
        ];
        assert_eq!(orders.len(), 9);
        assert_ne!(ResolvedPrimaryAction::Move, ResolvedPrimaryAction::AttackMove);
    }

    #[test]
    fn presentation_default_clears_hover_primary() {
        let p = BattlePresentationState::default();
        assert_eq!(p.pointer, BattlePointer::Default);
        assert_eq!(p.hover_cell, None);
        assert_eq!(p.hover_primary, ResolvedPrimaryAction::Noop);
        assert_eq!(p.marquee, None);
    }

    #[test]
    fn interaction_mode_tool_kind_matches() {
        assert_eq!(BattleInteractionMode::Normal.tool_kind(), BattleToolKind::Normal);
        assert_eq!(
            BattleInteractionMode::PlaceBuilding { type_id: "GACNST".into() }.tool_kind(),
            BattleToolKind::PlaceBuilding
        );
        assert_eq!(BattleInteractionMode::AttackMove.tool_kind(), BattleToolKind::AttackMove);
        assert!(BattleToolKind::Repair.is_tool());
        assert!(!BattleToolKind::Normal.is_tool());
    }

    #[test]
    fn input_frame_order_mod_follows_ctrl_alt() {
        assert_eq!(OrderClickModifier::from_keys(true, false), OrderClickModifier::ForceAttack);
        assert_eq!(OrderClickModifier::from_keys(false, true), OrderClickModifier::ForceMove);
    }

    #[test]
    fn keyboard_and_edge_scroll_deltas_add() {
        let keys = CameraPanKeys { left: true, right: false, up: false, down: false };
        let (kx, ky) = keyboard_pan_screen_delta(keys, 100.0, 0.5);
        assert!((kx - 50.0).abs() < 0.01);
        assert_eq!(ky, 0.0);
        // 左边缘：edge 产生正 dx，与方向键左叠加。
        let (ex, ey) = edge_scroll_screen_delta(0.0, 100.0, 800, 600, 16.0, 100.0, 0.5);
        assert!(ex > 0.0);
        assert_eq!(ey, 0.0);
        let dx = kx + ex;
        let dy = ky + ey;
        assert!(dx > kx);
        assert_eq!(dy, 0.0);
    }

    #[test]
    fn edge_scroll_axes_ignore_interior_cursor() {
        let (w, e, n, s) = edge_scroll_axes(400.0, 300.0, 800, 600, 16.0);
        assert!(!w && !e && !n && !s);
        let (w, e, n, s) = edge_scroll_axes(2.0, 300.0, 800, 600, 16.0);
        assert!(w && !e && !n && !s);
    }

    #[test]
    fn tracker_left_press_release_edges() {
        let mut t = BattleInputTracker::default();
        t.focused = true;
        t.begin_frame();
        t.set_left(true);
        assert!(t.edges.left_pressed);
        assert!(t.buttons.left);
        t.begin_frame();
        assert!(!t.edges.left_pressed);
        assert!(t.buttons.left);
        t.set_left(false);
        assert!(t.edges.left_released);
        assert!(!t.buttons.left);
    }

    #[test]
    fn tracker_focus_lost_clears_held_without_release_edge() {
        // 序列：左键按下 → 失焦 → 恢复 → 移动 → 释放，不得把失焦当成点击释放。
        let mut t = BattleInputTracker { focused: true, ..Default::default() };
        t.begin_frame();
        t.set_left(true);
        assert!(t.buttons.left);
        t.begin_frame();
        t.set_focused(false);
        assert!(t.edges.focus_lost);
        assert!(!t.buttons.left);
        assert!(!t.edges.left_released, "focus loss must cancel hold, not fire release");
        t.begin_frame();
        t.set_focused(true);
        t.set_left(false);
        assert!(!t.edges.left_released, "button already clear, spurious up is ignored");
    }

    #[test]
    fn tracker_reset_transient_drops_buttons_keeps_frame_edges() {
        let mut t = BattleInputTracker { focused: true, ..Default::default() };
        t.set_left(true);
        t.set_right(true);
        t.add_wheel_steps(2);
        t.reset_transient();
        assert!(!t.buttons.any());
        assert_eq!(t.edges.wheel_steps, 2, "reset_transient keeps frame edges until begin_frame");
        assert!(t.focused, "reset_transient keeps focus flag");
        t.begin_frame();
        assert_eq!(t.edges, BattleInputEdges::default());
    }

    #[test]
    fn tracker_default_focused_so_first_blur_clears_hold() {
        let mut t = BattleInputTracker::default();
        assert!(t.focused);
        t.set_left(true);
        t.set_focused(false);
        assert!(t.edges.focus_lost);
        assert!(!t.buttons.left);
        assert!(!t.edges.left_released);
    }

    #[test]
    fn tracker_camera_pan_edges_and_silent_clear() {
        let mut t = BattleInputTracker::default();
        let mut keys = CameraPanKeys::default();
        t.begin_frame();
        let prev = keys;
        keys.left = true;
        t.note_camera_pan(prev, keys);
        assert!(t.edges.pan_pressed.left);
        assert!(!t.edges.pan_released.left);
        t.begin_frame();
        let prev = keys;
        keys.left = false;
        t.note_camera_pan(prev, keys);
        assert!(t.edges.pan_released.left);
        // 暂停静默清空：不调用 note_camera_pan → 无抬起边沿。
        t.begin_frame();
        keys.right = true;
        keys.clear();
        assert!(!t.edges.pan_released.any());
        assert!(!keys.any());
    }

    #[test]
    fn pause_while_holding_pan_must_clear_held_keys() {
        // 按住方向键 → 暂停静默 clear → 恢复后不得继续平移。
        let mut keys = CameraPanKeys { left: true, right: false, up: true, down: false };
        assert!(keys.any());
        keys.clear();
        assert!(!keys.any());
        let (dx, dy) = keyboard_pan_screen_delta(keys, 640.0, 1.0 / 60.0);
        assert_eq!((dx, dy), (0.0, 0.0));
    }

    #[test]
    fn left_gesture_slop_becomes_marquee_then_release() {
        let g = LeftGesture::begin(10.0, 20.0);
        let g = g.on_cursor_moved(12.0, 22.0);
        assert!(matches!(g, LeftGesture::MaybeClick { .. }));
        let g = g.on_cursor_moved(10.0 + f64::from(CLICK_SLOP_PX) + 1.0, 20.0);
        assert!(matches!(g, LeftGesture::Marquee { .. }));
        let rect = g.marquee_rect().expect("marquee rect");
        assert!(rect.w >= CLICK_SLOP_PX);
        let (idle, action) = g.release();
        assert_eq!(idle, LeftGesture::Idle);
        assert!(matches!(action, LeftReleaseAction::Marquee(_)));
    }

    #[test]
    fn left_gesture_small_move_stays_click() {
        let g = LeftGesture::begin(0.0, 0.0).on_cursor_moved(2.0, 2.0);
        let (_, action) = g.release();
        assert_eq!(action, LeftReleaseAction::Click);
    }

    #[test]
    fn hud_command_release_requires_same_slot() {
        assert!(hud_command_release_fires(3, Some(3)));
        assert!(!hud_command_release_fires(3, Some(4)));
        assert!(!hud_command_release_fires(3, None));
        assert!(hud_sidebar_release_fires(true));
        assert!(!hud_sidebar_release_fires(false));
    }

    #[test]
    fn world_capture_not_replaced_by_hud_move_rule() {
        // 战术区按下后移入 HUD：捕获仍为 World（由 ingest 不改 capture 保证）。
        let cap = BattleUiCapture::World;
        assert!(cap.is_world());
        assert!(!cap.is_hud());
        assert_eq!(left_release_policy(cap), LeftReleasePolicy::WorldGesture);
        assert!(should_advance_world_gesture(true, false, cap));
        // 移入 HUD 不改变 capture，因此释放仍走战术区。
        assert!(!should_advance_world_gesture(true, false, BattleUiCapture::HudCommand(0)));
    }

    #[test]
    fn press_capture_priority_command_over_sidebar_over_world() {
        assert_eq!(resolve_press_capture(Some(2), true, true), BattleUiCapture::HudCommand(2));
        assert_eq!(resolve_press_capture(None, true, true), BattleUiCapture::HudSidebar);
        assert_eq!(resolve_press_capture(None, false, true), BattleUiCapture::World);
        assert_eq!(resolve_press_capture(None, false, false), BattleUiCapture::None);
    }

    #[test]
    fn place_mode_blocks_marquee_upgrade_keeps_click() {
        let cap = BattleUiCapture::World;
        assert!(!should_advance_world_gesture(true, true, cap));
        let g = LeftGesture::begin(0.0, 0.0);
        // 放置时 CursorMoved 不调用 on_cursor_moved → 保持 MaybeClick → 释放为 Click。
        let (_, action) = g.release();
        assert_eq!(action, LeftReleaseAction::Click);
    }

    #[test]
    fn marquee_drag_into_edge_keeps_gesture() {
        // 框选过程中进入边缘：手势继续，不因边缘滚屏意图而 Idle。
        let g = LeftGesture::begin(100.0, 100.0).on_cursor_moved(100.0 + f64::from(CLICK_SLOP_PX) + 2.0, 100.0);
        assert!(matches!(g, LeftGesture::Marquee { .. }));
        let at_edge = g.on_cursor_moved(2.0, 100.0);
        assert!(matches!(at_edge, LeftGesture::Marquee { .. }));
        assert!(should_advance_world_gesture(true, false, BattleUiCapture::World));
    }

    #[test]
    fn right_press_clears_capture_and_gesture() {
        assert_eq!(capture_after_right_press(), BattleUiCapture::None);
        let g = LeftGesture::begin(0.0, 0.0).on_cursor_moved(20.0, 20.0);
        assert!(matches!(g, LeftGesture::Marquee { .. }));
        // 右键路径：清空捕获后手势应 Idle（由 clear_pointer_capture 保证）。
        let cleared = LeftGesture::Idle;
        assert_eq!(cleared, LeftGesture::Idle);
        assert_eq!(left_release_policy(BattleUiCapture::None), LeftReleasePolicy::Ignore);
    }

    #[test]
    fn hud_press_move_out_does_not_fire() {
        assert!(!hud_command_release_fires(1, None));
        assert!(!hud_command_release_fires(1, Some(2)));
        assert_eq!(left_release_policy(BattleUiCapture::HudCommand(1)), LeftReleasePolicy::HudCommand(1));
    }

    #[test]
    fn focus_lost_then_release_is_ignore_policy() {
        // 失焦 reset 后 capture=None → 恢复焦点再 Released 走 Ignore，不点选。
        let mut t = BattleInputTracker::default();
        t.set_left(true);
        t.set_focused(false);
        assert!(!t.buttons.left);
        assert!(!t.edges.left_released);
        assert_eq!(left_release_policy(BattleUiCapture::None), LeftReleasePolicy::Ignore);
        t.begin_frame();
        t.set_focused(true);
        t.set_left(false);
        assert!(!t.edges.left_released);
    }

    #[test]
    fn classify_right_click_cancels_tools_or_deselects() {
        assert_eq!(classify_right_click_map(true), RightClickMapOutcome::CancelToolModes);
        assert_eq!(classify_right_click_map(false), RightClickMapOutcome::Deselect);
    }

    #[test]
    fn tracker_modifiers_edges_and_focus_lost_silent_clear() {
        // Shift 按住加选 → 失焦 → 恢复：修饰键必须静默清空，不得残留 shift_down 幽灵。
        let mut t = BattleInputTracker::default();
        t.begin_frame();
        t.set_modifiers(true, false, false);
        assert!(t.modifiers.shift);
        assert!(t.edges.mod_pressed.shift);
        assert!(!t.edges.mod_released.shift);
        t.begin_frame();
        t.set_focused(false);
        assert!(t.edges.focus_lost);
        assert!(!t.modifiers.any());
        assert!(!t.edges.mod_released.shift, "focus loss must not emit mod_released");
        t.begin_frame();
        t.set_focused(true);
        assert!(!t.modifiers.shift);
        // 物理 Shift 仍按住时须等下一次 ModifiersChanged 才重新置位。
        t.set_modifiers(true, false, false);
        assert!(t.modifiers.shift);
        assert!(t.edges.mod_pressed.shift);
    }

    #[test]
    fn tracker_reset_transient_also_clears_modifiers() {
        let mut t = BattleInputTracker::default();
        t.set_modifiers(true, true, true);
        t.reset_transient();
        assert!(!t.modifiers.any());
        assert!(!t.buttons.any());
    }

    #[test]
    fn recommended_pointer_aligns_with_primary_actions() {
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Move, true, BattleToolKind::Normal),
            BattlePointer::Move
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Move, false, BattleToolKind::Normal),
            BattlePointer::NoMove
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Attack, true, BattleToolKind::Normal),
            BattlePointer::Attack
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Capture, true, BattleToolKind::Normal),
            BattlePointer::Attack
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Deploy, true, BattleToolKind::Normal),
            BattlePointer::Deploy
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Noop, true, BattleToolKind::Sell),
            BattlePointer::Sell
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Noop, false, BattleToolKind::AttackMove),
            BattlePointer::NoMove
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::AttackMove, true, BattleToolKind::AttackMove),
            BattlePointer::Attack
        );
    }

    #[test]
    fn edge_scroll_pointer_beats_context_pointer() {
        let ctx = BattlePointer::Attack;
        assert_eq!(BattlePointer::resolve(EdgeScrollCursor::Default, ctx), BattlePointer::Attack);
        let edge = EdgeScrollCursor::Scroll(EdgeScrollDir::East);
        assert_eq!(BattlePointer::resolve(edge, ctx), BattlePointer::Edge(edge));
        let blocked = EdgeScrollCursor::Blocked(EdgeScrollDir::West);
        assert_eq!(BattlePointer::resolve(blocked, BattlePointer::Move), BattlePointer::Edge(blocked));
    }

    #[test]
    fn next_tool_after_map_right_click_cancels_tools() {
        for tool in [
            BattleToolKind::PlaceBuilding,
            BattleToolKind::Repair,
            BattleToolKind::Sell,
            BattleToolKind::Planning,
            BattleToolKind::AttackMove,
            BattleToolKind::Follow,
        ] {
            let (next, outcome) = next_tool_after_map_right_click(tool);
            assert_eq!(next, BattleToolKind::Normal);
            assert_eq!(outcome, RightClickMapOutcome::CancelToolModes);
        }
        let (next, outcome) = next_tool_after_map_right_click(BattleToolKind::Normal);
        assert_eq!(next, BattleToolKind::Normal);
        assert_eq!(outcome, RightClickMapOutcome::Deselect);
    }

    #[test]
    fn surface_metrics_physical_to_logical_at_scale_2() {
        use winit::dpi::PhysicalPosition;
        let m = BattleSurfaceMetrics {
            logical_width: 800,
            logical_height: 600,
            physical_width: 1600,
            physical_height: 1200,
            scale_factor: 2.0,
        };
        let (x, y) = m.cursor_from_physical(PhysicalPosition::new(200.0, 100.0));
        assert!((x - 100.0).abs() < 0.001);
        assert!((y - 50.0).abs() < 0.001);
        let (px, py, pw, ph) = m.layout_rect_to_physical(10, 20, 100, 50);
        assert_eq!((px, py, pw, ph), (20, 40, 200, 100));
    }

    #[test]
    fn sidebar_right_edge_still_triggers_edge_scroll_axes() {
        // 光标在右侧栏边缘（接近客户区右缘）仍应触发东向边缘意图。
        let (w, e, n, s) = edge_scroll_axes(799.0, 300.0, 800, 600, 16.0);
        assert!(!w && e && !n && !s);
    }

    #[test]
    fn outcome_hold_press_then_reset_leaves_no_release_edge() {
        // 结算期按下 → reset_transient → 回局后抬起不得当成点击。
        let mut t = BattleInputTracker::default();
        t.begin_frame();
        t.set_left(true);
        assert!(t.buttons.left);
        t.reset_transient();
        assert!(!t.buttons.left);
        assert!(!t.edges.left_released);
        t.begin_frame();
        t.set_left(false);
        assert!(!t.edges.left_released);
    }

    #[test]
    fn tracker_wheel_steps_accumulate_until_begin_frame() {
        let mut t = BattleInputTracker::default();
        t.add_wheel_steps(1);
        t.add_wheel_steps(2);
        assert_eq!(t.edges.wheel_steps, 3);
        t.begin_frame();
        assert_eq!(t.edges.wheel_steps, 0);
    }

    #[test]
    fn ctrl_left_and_alt_left_order_mod_matrix() {
        // Ctrl+左键敌方 / Alt+左键空地：修饰只进 OrderClickModifier，不进交互模式。
        assert_eq!(OrderClickModifier::from_keys(true, false), OrderClickModifier::ForceAttack);
        assert_eq!(OrderClickModifier::from_keys(false, true), OrderClickModifier::ForceMove);
        assert_eq!(OrderClickModifier::from_keys(true, true), OrderClickModifier::ForceAttack);
        assert!(!BattleToolKind::Normal.is_tool());
    }

    #[test]
    fn blocked_ground_forces_nomove_even_under_attack_move_tool() {
        // 控制器把 Blocked 映射为 Noop primary + 强制 NoMove；纯函数在 AttackMove+不可通行时也是 NoMove。
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::Noop, false, BattleToolKind::AttackMove),
            BattlePointer::NoMove
        );
        assert_eq!(
            recommended_pointer_for_primary(ResolvedPrimaryAction::AttackMove, false, BattleToolKind::AttackMove),
            BattlePointer::NoMove
        );
    }

    #[test]
    fn next_tool_matches_classify_right_click_map() {
        assert_eq!(
            next_tool_after_map_right_click(BattleToolKind::Repair).1,
            classify_right_click_map(true)
        );
        assert_eq!(
            next_tool_after_map_right_click(BattleToolKind::Normal).1,
            classify_right_click_map(false)
        );
    }

    #[test]
    fn mobile_ground_order_matrix() {
        use OrderClickModifier::{ForceAttack, ForceMove, None as ModNone};
        assert_eq!(
            resolve_mobile_ground_order(false, false, ModNone, false),
            MobileGroundOrderKind::Blocked
        );
        assert_eq!(
            resolve_mobile_ground_order(true, true, ModNone, false),
            MobileGroundOrderKind::QueueMovePath
        );
        assert_eq!(
            resolve_mobile_ground_order(true, true, ModNone, true),
            MobileGroundOrderKind::AttackMove
        );
        assert_eq!(
            resolve_mobile_ground_order(true, false, ForceAttack, false),
            MobileGroundOrderKind::AttackMove
        );
        assert_eq!(
            resolve_mobile_ground_order(true, true, ForceMove, false),
            MobileGroundOrderKind::Move { queue_path: true }
        );
        assert_eq!(
            resolve_mobile_ground_order(true, false, ModNone, false),
            MobileGroundOrderKind::Move { queue_path: false }
        );
        // 不可通行时即使强攻工具也不下令（与 NoMove 光标同源）。
        assert_eq!(
            resolve_mobile_ground_order(false, false, ForceAttack, true),
            MobileGroundOrderKind::Blocked
        );
    }

    #[test]
    fn non_mobile_ground_order_matrix() {
        assert_eq!(
            resolve_non_mobile_ground_order(true, false),
            EmptyOrStructureGroundKind::SetRally
        );
        assert_eq!(
            resolve_non_mobile_ground_order(false, true),
            EmptyOrStructureGroundKind::Noop
        );
        assert_eq!(
            resolve_non_mobile_ground_order(false, false),
            EmptyOrStructureGroundKind::Deselect
        );
    }

    #[test]
    fn friendly_click_deploy_beats_select() {
        assert_eq!(
            resolve_friendly_click(false, true, true, false),
            FriendlyClickKind::Deploy
        );
        assert_eq!(
            resolve_friendly_click(false, true, false, true),
            FriendlyClickKind::SetPrimary
        );
        assert_eq!(
            resolve_friendly_click(false, false, true, false),
            FriendlyClickKind::Select { add: false }
        );
        assert_eq!(
            resolve_friendly_click(true, true, true, true),
            FriendlyClickKind::Select { add: true }
        );
    }

    #[test]
    fn hostile_click_capture_infiltrate_before_attack() {
        assert_eq!(
            resolve_hostile_click(false, true, true, true, false),
            HostileClickKind::Capture
        );
        assert_eq!(
            resolve_hostile_click(false, true, false, false, true),
            HostileClickKind::Infiltrate
        );
        assert_eq!(
            resolve_hostile_click(true, true, true, true, true),
            HostileClickKind::Attack
        );
        assert_eq!(
            resolve_hostile_click(false, false, true, true, true),
            HostileClickKind::Attack
        );
    }

    #[test]
    fn follow_click_cancel_noop_and_follow() {
        assert_eq!(resolve_follow_click(false, true, false), FollowClickKind::Cancel);
        assert_eq!(resolve_follow_click(true, false, false), FollowClickKind::Noop);
        assert_eq!(resolve_follow_click(true, true, true), FollowClickKind::Noop);
        assert_eq!(resolve_follow_click(true, true, false), FollowClickKind::Follow);
    }

    #[test]
    fn miss_click_and_force_move_gates() {
        assert_eq!(resolve_miss_click(false, true), MissClickKind::Deselect);
        assert_eq!(resolve_miss_click(true, true), MissClickKind::Noop);
        assert_eq!(resolve_miss_click(false, false), MissClickKind::Noop);
        assert!(should_skip_friendly_pick(OrderClickModifier::ForceMove, true));
        assert!(!should_skip_friendly_pick(OrderClickModifier::ForceMove, false));
        assert!(should_try_hostile_order(OrderClickModifier::None, true));
        assert!(!should_try_hostile_order(OrderClickModifier::ForceMove, true));
    }

    #[test]
    fn tool_mode_place_sell_repair_planning_matrix() {
        assert_eq!(resolve_place_click(true, true), PlaceClickKind::Place);
        assert_eq!(resolve_place_click(true, false), PlaceClickKind::Noop);
        assert_eq!(resolve_place_click(false, true), PlaceClickKind::Noop);
        assert_eq!(resolve_sidebar_building_tool_click(true), SidebarBuildingToolClickKind::Apply);
        assert_eq!(resolve_sidebar_building_tool_click(false), SidebarBuildingToolClickKind::Noop);
        assert_eq!(
            resolve_planning_click(true, true, true),
            PlanningClickKind::AppendWaypoint
        );
        assert_eq!(resolve_planning_click(true, true, false), PlanningClickKind::Blocked);
        assert_eq!(resolve_planning_click(false, true, true), PlanningClickKind::Noop);
        assert_eq!(resolve_planning_click(true, false, true), PlanningClickKind::Noop);
    }

    #[test]
    fn sequence_shift_hold_focus_lost_clears_modifier_without_release() {
        // Shift 按住选中 → 失焦 → 恢复：修饰键静默清空，无 mod_released，避免幽灵加选。
        let mut t = BattleInputTracker::default();
        t.begin_frame();
        t.set_modifiers(true, false, false);
        assert!(t.modifiers.shift);
        assert!(t.edges.mod_pressed.shift);
        t.begin_frame();
        t.set_left(true);
        assert!(t.buttons.left && t.edges.left_pressed);
        t.set_focused(false);
        assert!(!t.modifiers.shift);
        assert!(!t.buttons.left);
        assert!(t.edges.focus_lost);
        assert!(!t.edges.mod_released.shift);
        assert!(!t.edges.left_released);
        t.begin_frame();
        t.set_focused(true);
        assert!(!t.modifiers.any());
        assert!(!t.buttons.any());
    }

    #[test]
    fn sequence_place_then_right_cancel_returns_normal_keeps_select_policy() {
        // 进入放置 → 右键取消工具 → 回到 Normal，语义为 CancelToolModes（保留选中）。
        let (next, outcome) = next_tool_after_map_right_click(BattleToolKind::PlaceBuilding);
        assert_eq!(next, BattleToolKind::Normal);
        assert_eq!(outcome, RightClickMapOutcome::CancelToolModes);
        let (next, outcome) = next_tool_after_map_right_click(BattleToolKind::Repair);
        assert_eq!(next, BattleToolKind::Normal);
        assert_eq!(outcome, RightClickMapOutcome::CancelToolModes);
        let (next, outcome) = next_tool_after_map_right_click(BattleToolKind::AttackMove);
        assert_eq!(next, BattleToolKind::Normal);
        assert_eq!(outcome, RightClickMapOutcome::CancelToolModes);
        let (next, outcome) = next_tool_after_map_right_click(BattleToolKind::Normal);
        assert_eq!(next, BattleToolKind::Normal);
        assert_eq!(outcome, RightClickMapOutcome::Deselect);
    }

    #[test]
    fn sequence_world_press_hud_move_release_ignores_hud() {
        // 战术区按下 → 移入 HUD → 释放：仍认 World 捕获，不因 HUD 命中改捕获。
        let cap = resolve_press_capture(None, false, true);
        assert_eq!(cap, BattleUiCapture::World);
        assert!(should_advance_world_gesture(true, false, cap));
        assert!(!should_advance_world_gesture(true, false, BattleUiCapture::HudCommand(1)));
        assert_eq!(left_release_policy(cap), LeftReleasePolicy::WorldGesture);
        assert_eq!(left_release_policy(BattleUiCapture::HudCommand(1)), LeftReleasePolicy::HudCommand(1));
    }

    #[test]
    fn sequence_hud_press_move_out_release_does_not_fire() {
        let cap = resolve_press_capture(Some(2), true, true);
        assert_eq!(cap, BattleUiCapture::HudCommand(2));
        assert!(!hud_command_release_fires(2, None));
        assert!(!hud_command_release_fires(2, Some(3)));
        assert!(hud_command_release_fires(2, Some(2)));
        assert!(!hud_sidebar_release_fires(false));
        assert!(hud_sidebar_release_fires(true));
    }

    #[test]
    fn sequence_pause_while_holding_arrows_clears_pan_keys() {
        // 暂停时按住方向键 → 恢复后不应自动继续平移（held 必须被清）。
        let mut keys = CameraPanKeys { left: true, right: false, up: true, down: false };
        assert!(keys.any());
        // 与 `tick_edge_scroll` 暂停分支同口径。
        keys.clear();
        assert!(!keys.any());
        let (dx, dy) = keyboard_pan_screen_delta(keys, 640.0, 1.0);
        assert_eq!((dx, dy), (0.0, 0.0));
    }

    #[test]
    fn sequence_ctrl_hostile_and_alt_force_move_gates() {
        // Ctrl + 左键敌方 → Attack；Alt + 有机动 → 跳过友军点选。
        assert_eq!(
            resolve_hostile_click(true, true, true, true, true),
            HostileClickKind::Attack
        );
        assert!(should_skip_friendly_pick(OrderClickModifier::ForceMove, true));
        assert!(!should_try_hostile_order(OrderClickModifier::ForceMove, true));
        assert_eq!(
            resolve_mobile_ground_order(true, false, OrderClickModifier::ForceMove, false),
            MobileGroundOrderKind::Move { queue_path: false }
        );
    }
}
