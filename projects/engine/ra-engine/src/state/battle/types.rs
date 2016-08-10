use std::{collections::HashSet, sync::Arc};

use ra_assets::TechnoKind;
use ra_map::{MapInfo, PassGrid};
use ra_types::{EntityId, GameEdition, PlayerId, RuntimeDefinitions, ScheduledCommand};

use super::super::{ecs_registry::EcsRegistry, entities::WorldEntity, players::PlayerState};
use crate::{
    game::{CommandReject, InputFrame},
    presentation::DirtyEntitySet,
};

/// 走一格所需的移动点（预览用常量，非零售精确换算）。
pub const CELL_MOVE_COST: u32 = 64;

/// 炮塔每 tick 最多转过的朝向单位（0..=255 环）。
pub const TURRET_TURN_STEP: u8 = 16;

/// 预览用默认攻击射程（曼哈顿格）；rules 无 `Sight` 时回退。
pub const DEFAULT_ATTACK_RANGE: u32 = 4;

/// 预览用默认单次伤害。
pub const DEFAULT_ATTACK_DAMAGE: u32 = 50;

/// 两次开火之间的 tick 数；rules 无 `ROF` 时回退。
pub const ATTACK_COOLDOWN_TICKS: u32 = 8;

/// 受击闪白剩余 tick（呈现 `TakeDamage`）。
pub const HIT_FLASH_TICKS: u32 = 4;

/// 矿车在矿格上完成一趟采集所需的 tick 数（竖切简化，无独立装载动画）。
pub const ORE_TRIP_TICKS: u32 = 30;

/// 矿车向矿场卸货后给所属房主增加的资金。
pub const ORE_INCOME_PER_TRIP: u32 = 700;

/// 工厂完成一件生产所需的 tick 数（Alpha 简化）。
/// 缺省 `BuildTime`（INI 为 0 或未写）时的生产时长（逻辑 tick）。
pub const PRODUCE_TICKS: u32 = 20;

/// 每个 INI `BuildTime` 单位对应的逻辑 tick（可调比例，非零售精确换算）。
pub const BUILD_TIME_TICKS_PER_UNIT: u32 = 4;

/// ECS 战斗静态参数只读视图（测试与诊断）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EcsCombatView {
    /// 护甲名。
    pub armor: String,
    /// 攻击射程。
    pub attack_range: u32,
    /// 单次基础伤害。
    pub attack_damage: u32,
    /// 开火冷却上限。
    pub attack_cooldown_max: u32,
    /// 弹头对各护甲的伤害百分比。
    pub attack_verses: [u32; 11],
    /// 对应 techno 种类。
    pub techno_kind: Option<TechnoKind>,
}

/// 确定性仿真世界：实体、通行格与按 tick 消费的命令。
#[derive(Debug, Clone)]
pub struct BattleState {
    /// 当前游戏版本。
    pub edition: GameEdition,
    /// 已推进的逻辑 tick 计数。
    pub tick: u64,
    /// 地图信息（尺寸、放置实体等）。
    pub map: MapInfo,
    /// Overlay 类型表（含可采标记，供采矿查询）。
    pub overlay_types: ra_assets::OverlayTypeRegistry,
    /// 通行格（由地图结构派生，可被重寻路使用）。
    pub pass_grid: PassGrid,
    /// 世界实体投影槽（与地图播种顺序对应；权威在 ECS）。
    pub(crate) entities: Vec<WorldEntity>,
    /// 玩家状态（资金、电力等）。
    pub players: Vec<PlayerState>,
    /// 本地玩家 ID。
    pub local_player: PlayerId,
    /// 冻结运行时定义（adaptor 生成；玩法查询只走此表）。
    pub definitions: Arc<RuntimeDefinitions>,
    /// 下一枚可分配的稳定实体 ID（从 1 起）。
    pub(crate) next_entity_id: u64,
    /// 下一枚可分配的命令 ID（从 1 起）。
    pub(crate) next_command_id: u64,
    /// 待本 tick 消费的已调度命令（先进先出）。
    pub(crate) pending_commands: Vec<ScheduledCommand>,
    /// 上一 tick 实际消费的输入帧（含空帧）。
    pub(crate) last_input_frame: InputFrame,
    /// 上一 tick 产生的命令拒绝记录。
    pub(crate) last_rejects: Vec<CommandReject>,
    /// 本局已消费过的 `CommandId`（用于拒绝重复调度）。
    pub(crate) seen_command_ids: HashSet<u64>,
    pub(crate) state_hash: u64,
    /// 呈现脏实体集（增量 `RenderWorld` 用；与全量 snapshot 并存）。
    pub(crate) presentation_dirty: DirtyEntitySet,
    /// 当前闪电风暴（同时最多一场；驱动 Ion 光照档）。
    pub lightning_storm: Option<crate::gameplay::LightningStormState>,
    /// 各 house 超级武器充能。
    pub super_weapon_runtime: crate::gameplay::SuperWeaponRuntime,
    /// 地图触发运行时。
    pub trigger_runtime: crate::gameplay::TriggerRuntime,
    /// 小队 ScriptTypes 运行时。
    pub script_team_runtime: crate::gameplay::ScriptTeamRuntime,
    /// 地图 AITriggerTypes 运行时。
    pub ai_trigger_runtime: crate::gameplay::AiTriggerRuntime,
    /// `SpawnsTiberium` 矿柱产矿动画状态。
    pub terrain_spawners: Vec<crate::gameplay::TerrainSpawnerState>,
    /// 本局写入后待叠画的 overlay 格（产矿等；呈现层 `take_overlay_paint_dirty` 消费）。
    pub(crate) overlay_paint_dirty: Vec<(u16, u16)>,
    /// 待重绘的建筑实体（占领换色等；呈现层 `take_structure_paint_dirty` 消费）。
    pub(crate) structure_paint_dirty: Vec<EntityId>,
    /// 待播 Buildup 的新建建筑（放置 / 部署；呈现层 `take_structure_buildup_dirty` 消费）。
    pub(crate) structure_buildup_dirty: Vec<EntityId>,
    /// 对局随机种子（产矿掷骰等；由 `BattleSession::set_match_seed` 写入）。
    pub match_seed: u64,
    /// 本 tick 玩法侧排队的 EVA 提示（按 house；壳层只播本机）。
    pub(crate) pending_eva_cues: Vec<crate::state::EvaCue>,
    /// 基地遇袭 EVA 近距/时间去重窗口。
    pub(crate) eva_base_under_attack: Vec<crate::state::EvaBaseUnderAttackGate>,
    /// `[AudioVisual] SpeakDelay` 换算后的资金唠叨周期（逻辑 tick；0 表示关闭）。
    pub(crate) speak_delay_ticks: u32,
    /// 内部 ECS 世界与 `EntityId` 映射（玩法权威；`entities` 仅为投影槽）。
    pub(crate) ecs: EcsRegistry,
}
