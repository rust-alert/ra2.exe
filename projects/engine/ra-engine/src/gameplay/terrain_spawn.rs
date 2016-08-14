//! `SpawnsTiberium` 矿柱产矿概率状态机。
//!
//! Idle 时每 tick 按 `AnimationProbability` 掷骰；命中后从第 0 帧播到中点，
//! 中点触发邻格产矿并回到 Idle。呈现只读 [`TerrainSpawnerState::render_frame`]。

use ra_map::{MapInfo, OverlayCell, TerrainObject};
use ra_types::{OverlayTypeRegistry, TerrainSpawnerDefinitions};

/// 概率分母（与零售 `random % 1_000_000` 对齐）。
pub const PROBABILITY_DENOMINATOR: u32 = 1_000_000;

/// 邻格产矿默认密度等级（`OverlayData`）。
pub const SPAWN_ORE_DENSITY: u8 = 3;

/// 无 SHP 元数据时的零售矿柱总帧数（含落影半幅）。
pub const STOCK_TIBTRE_FRAME_COUNT: u16 = 22;

/// 8 邻方向。
pub const ADJACENT: [(i32, i32); 8] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)];

/// 单个矿柱的动画相位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum TerrainSpawnerPhase {
    /// 静止；呈现固定第 0 帧。
    Idle,
    /// 正在播放；`current_frame` 为呈现帧下标。
    Active {
        /// 当前主体帧（0..midpoint）。
        current_frame: u16,
        /// 距下一帧剩余逻辑 tick。
        ticks_until_next: u16,
    },
}

/// 一次 tick 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum TerrainSpawnerTick {
    /// 仍在 Idle。
    Idle,
    /// 本 tick 刚开始动画。
    Started,
    /// 动画推进中。
    Active,
    /// 到达中点：应产矿并已回到 Idle。
    SpawnDue,
}

/// 地图上一座 `SpawnsTiberium` 矿柱的运行时状态。
#[derive(Debug, Clone, PartialEq)]
#[doc(hidden)]
pub struct TerrainSpawnerState {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// rules 节名（如 `TIBTRE01`）。
    pub type_name: String,
    /// `AnimationProbability` × 1_000_000。
    pub animation_probability_micros: u32,
    /// `AnimationRate`（逻辑 tick / 动画帧）。
    pub animation_rate_ticks: u16,
    /// SHP 总帧数（含落影；中点 = 此值 / 2）。
    pub frame_count: u16,
    /// 产矿并复位的帧下标（通常 `frame_count / 2`）。
    pub midpoint_frame: u16,
    /// 当前相位。
    pub phase: TerrainSpawnerPhase,
}

impl TerrainSpawnerState {
    /// 由 rules 字段与帧数构造（初始 Idle）。
    pub fn new(
        x: u16,
        y: u16,
        type_name: impl Into<String>,
        animation_probability_micros: u32,
        animation_rate_ticks: u16,
        frame_count: u16,
    ) -> Self {
        let frame_count = frame_count.max(1);
        Self {
            x,
            y,
            type_name: type_name.into(),
            animation_probability_micros: animation_probability_micros.min(PROBABILITY_DENOMINATOR),
            animation_rate_ticks: animation_rate_ticks.max(1),
            frame_count,
            midpoint_frame: frame_count / 2,
            phase: TerrainSpawnerPhase::Idle,
        }
    }

    /// 呈现用帧号：Idle → 0；Active → `current_frame`（不超过主体半幅）。
    pub fn render_frame(&self) -> u16 {
        match self.phase {
            TerrainSpawnerPhase::Idle => 0,
            TerrainSpawnerPhase::Active { current_frame, .. } => {
                let body = self.body_frame_count();
                if body == 0 { 0 } else { current_frame.min((body - 1) as u16) }
            }
        }
    }

    /// 主体帧数（有落影半幅时为前半）。
    pub fn body_frame_count(&self) -> usize {
        if self.frame_count > 0 && self.frame_count % 2 == 0 { usize::from(self.frame_count / 2) } else { usize::from(self.frame_count.max(1)) }
    }

    /// 推进一个逻辑 tick。
    pub fn tick(&mut self, sample_micros: u32) -> TerrainSpawnerTick {
        match self.phase {
            TerrainSpawnerPhase::Idle => {
                if self.animation_probability_micros == 0 || self.midpoint_frame == 0 {
                    return TerrainSpawnerTick::Idle;
                }
                if sample_micros < self.animation_probability_micros {
                    self.phase = TerrainSpawnerPhase::Active { current_frame: 0, ticks_until_next: self.animation_rate_ticks };
                    TerrainSpawnerTick::Started
                }
                else {
                    TerrainSpawnerTick::Idle
                }
            }
            TerrainSpawnerPhase::Active { current_frame, ticks_until_next } => {
                let next_timer = ticks_until_next.saturating_sub(1);
                if next_timer > 0 {
                    self.phase = TerrainSpawnerPhase::Active { current_frame, ticks_until_next: next_timer };
                    return TerrainSpawnerTick::Active;
                }
                let next_frame = current_frame.saturating_add(1);
                if next_frame >= self.midpoint_frame {
                    self.phase = TerrainSpawnerPhase::Idle;
                    TerrainSpawnerTick::SpawnDue
                }
                else {
                    self.phase = TerrainSpawnerPhase::Active { current_frame: next_frame, ticks_until_next: self.animation_rate_ticks };
                    TerrainSpawnerTick::Active
                }
            }
        }
    }
}

/// 从地图 `[Terrain]` 与冻结产矿定义播种矿柱状态。
pub fn seed_terrain_spawners(map: &MapInfo, spawners: &TerrainSpawnerDefinitions) -> Vec<TerrainSpawnerState> {
    let mut out = Vec::new();
    for obj in &map.terrain_objects {
        if let Some(state) = spawner_from_terrain_object(obj, spawners) {
            out.push(state);
        }
    }
    out
}

#[doc(hidden)]
pub fn spawner_from_terrain_object(obj: &TerrainObject, spawners: &TerrainSpawnerDefinitions) -> Option<TerrainSpawnerState> {
    let def = spawners.get(&obj.name)?;
    Some(TerrainSpawnerState::new(
        obj.x,
        obj.y,
        obj.name.clone(),
        def.animation_probability_micros,
        def.animation_rate_ticks,
        STOCK_TIBTRE_FRAME_COUNT,
    ))
}

/// 确定性掷骰样本（0..1_000_000）。
pub fn terrain_spawn_sample(match_seed: u64, tick: u64, x: u16, y: u16) -> u32 {
    let mut h = match_seed ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (u64::from(x) << 32) ^ u64::from(y);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    (h % u64::from(PROBABILITY_DENOMINATOR)) as u32
}

/// 各矿柱当前呈现帧签名（刷新预览用）。
pub fn terrain_spawner_frame_signature(spawners: &[TerrainSpawnerState]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for s in spawners {
        h ^= u64::from(s.render_frame());
        h = h.wrapping_mul(0x0100_0000_01b3);
        h ^= u64::from(s.x) << 16 | u64::from(s.y);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// 在矿柱邻格放置一格可采 overlay（已有则抬密度；无空位则跳过）。
///
/// 成功时返回被写入的格子坐标，供呈现层脏刷新。
pub fn place_spawned_ore(
    overlays: &mut Vec<OverlayCell>,
    overlay_types: &OverlayTypeRegistry,
    ox: u16,
    oy: u16,
    pass_in_bounds: impl Fn(u16, u16) -> bool,
) -> Option<(u16, u16)> {
    let ore_id = first_harvestable_id(overlay_types)?;
    for (dx, dy) in ADJACENT {
        let x = i32::from(ox) + dx;
        let y = i32::from(oy) + dy;
        if x < 0 || y < 0 {
            continue;
        }
        let (x, y) = (x as u16, y as u16);
        if !pass_in_bounds(x, y) {
            continue;
        }
        if let Some(cell) = overlays.iter_mut().find(|c| c.x == x && c.y == y) {
            if overlay_types.is_harvestable(cell.overlay_id) {
                cell.data = cell.data.saturating_add(1).min(11);
                return Some((x, y));
            }
            continue;
        }
        overlays.push(OverlayCell { x, y, overlay_id: ore_id, data: SPAWN_ORE_DENSITY });
        return Some((x, y));
    }
    None
}

#[doc(hidden)]
pub fn first_harvestable_id(reg: &OverlayTypeRegistry) -> Option<u8> {
    for id in 0..reg.len().min(256) {
        let id = id as u8;
        if reg.is_harvestable(id) {
            return Some(id);
        }
    }
    None
}
