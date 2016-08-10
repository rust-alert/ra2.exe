use crate::{
    game::reject::CommandReject,
    state::{
        BattleState,
        components::{AnimationState, AttackState, Health, Identity, MovementState, Owner, ProductionQueue, Transform},
    },
};
use ra_map::{MapEntityKind, iso_to_screen};
use ra_types::{EntityId, GameEdition};

use super::{
    outcome::{BattleOutcome, BattleStats},
    session::BattleSession,
    types::{AnimState, SessionScreen},
};

/// 一帧呈现用的不可变快照（渲染器应逐步只消费此类数据）。
#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    /// 当前游戏版本。
    pub edition: GameEdition,
    /// 世界已推进的仿真 tick 数。
    pub tick: u64,
    /// 世界状态哈希（联机摘要用）。
    pub state_hash: u64,
    /// 可绘移动单位列表。
    pub units: Vec<SnapshotUnit>,
    /// 玩家经济与电力（供 HUD）。
    pub players: Vec<SnapshotPlayer>,
    /// 工厂生产队列与集结点。
    pub produce_queues: Vec<SnapshotProduceQueue>,
    /// 上一 tick 的命令拒绝（供错误反馈）。
    pub last_rejects: Vec<CommandReject>,
    /// 当前选中实体的稳定 ID（与 `units[].id` 对齐）。
    pub selected: Vec<EntityId>,
    /// 对局结束结果；未结束时为 `None`。
    pub outcome: Option<BattleOutcome>,
    /// 是否暂停（`pump` 不推进）。
    pub paused: bool,
    /// 暂停原因文案（胜负、手动暂停、摘要不一致等）。
    pub pause_reason: Option<String>,
    /// 结算统计；未结束时为 `None`。
    pub battle_stats: Option<BattleStats>,
    /// 当前会话画面（设置 / 对局中 / 结算）。
    pub screen: SessionScreen,
}

/// HUD / 标题栏用的轻量投影（不含单位表，避免第二遍全表扫描）。
#[derive(Debug, Clone)]
pub struct HudSnapshot {
    /// 世界已推进的仿真 tick 数。
    pub tick: u64,
    /// 玩家经济与电力。
    pub players: Vec<SnapshotPlayer>,
    /// 工厂生产队列。
    pub produce_queues: Vec<SnapshotProduceQueue>,
    /// 上一 tick 的命令拒绝。
    pub last_rejects: Vec<CommandReject>,
    /// 对局结束结果。
    pub outcome: Option<BattleOutcome>,
    /// 是否暂停。
    pub paused: bool,
    /// 暂停原因。
    pub pause_reason: Option<String>,
    /// 结算统计。
    pub battle_stats: Option<BattleStats>,
}

/// 快照中的玩家经济状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPlayer {
    /// 阵营 / 房主名称。
    pub house: std::sync::Arc<str>,
    /// 当前资金。
    pub funds: i32,
    /// 供电量。
    pub power_output: i32,
    /// 耗电量。
    pub power_drain: i32,
    /// 是否低电。
    pub low_power: bool,
}

/// 快照中的一条工厂生产队列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotProduceQueue {
    /// 工厂实体的稳定 ID。
    pub factory: EntityId,
    /// 正在生产的类型 ID。
    pub type_id: std::sync::Arc<str>,
    /// 剩余 tick（完工件为 0）。
    pub remaining_ticks: u32,
    /// 本单总 tick（来自 `BuildTime`；完工件仍保留开单时口径，缺定义则为 0）。
    pub total_ticks: u32,
    /// 集结格 X。
    pub rally_x: Option<u16>,
    /// 集结格 Y。
    pub rally_y: Option<u16>,
}

/// 快照中的一个可绘实体。
#[derive(Debug, Clone)]
pub struct SnapshotUnit {
    /// 实体稳定 ID。
    pub id: EntityId,
    /// 实体种类（单位 / 步兵 / 飞行器）。
    pub kind: MapEntityKind,
    /// 规则中的类型 ID（与权威实体共享 `Arc`）。
    pub type_id: std::sync::Arc<str>,
    /// 所属阵营 owner（与权威实体共享 `Arc`）。
    pub owner: std::sync::Arc<str>,
    /// 地图格 X。
    pub x: u16,
    /// 地图格 Y。
    pub y: u16,
    /// 相对预览图画布的像素 X（已减 `preview_origin`）。
    pub screen_x: i32,
    /// 相对预览图画布的像素 Y（已减 `preview_origin`）。
    pub screen_y: i32,
    /// 车体朝向（0–255）。
    pub facing: u8,
    /// 炮塔朝向（0–255）。
    pub turret_facing: u8,
    /// 当前 HVA 动画帧。
    pub hva_frame: u16,
    /// 呈现用动画状态（由仿真快照派生，不推进 BattleState tick）。
    pub anim_state: AnimState,
    /// 当前生命值。
    pub health: u32,
    /// 最大生命值。
    pub max_health: u32,
    /// 是否已死亡。
    pub dead: bool,
    /// 是否可部署（如 MCV）；选中时呈现部署标记。
    pub deployable: bool,
    /// 移动最终目标相对预览图的锚点（无目标为 `None`；供选中行动线）。
    pub move_goal_screen: Option<(i32, i32)>,
    /// 当前攻击目标实体。
    pub attack_target: Option<EntityId>,
    /// 攻击目标相对预览图的锚点（供选中行动线终点）。
    pub attack_target_screen: Option<(i32, i32)>,
    /// 建筑占地宽（格）；非建筑为 0。
    pub foundation_w: u16,
    /// 建筑占地高（格）；非建筑为 0。
    pub foundation_h: u16,
    /// 建筑 `Height`（缺省 2）；非建筑为 0。用于选中框竖向抬升。
    pub art_height: u16,
    /// `PixelSelectionBracketDelta`：选中血条竖直像素偏移（负值上移）。
    pub bracket_delta: i32,
}

impl SnapshotUnit {
    /// 是否为建筑标记（相对菱形单位用方块绘制）。
    pub fn is_structure(&self) -> bool {
        matches!(self.kind, MapEntityKind::Structure)
    }

    /// 是否为步兵（影响 `pipbrd` 帧与 pip 段数）。
    pub fn is_infantry(&self) -> bool {
        matches!(self.kind, MapEntityKind::Infantry)
    }
}

impl BattleSession {
    /// 将指定实体投影为呈现用 `SnapshotUnit`（跳过非战斗可视种类）。
    ///
    /// 供脏集增量路径使用。不存在的 ID 被跳过。
    pub fn project_units(&self, ids: &[EntityId]) -> Vec<SnapshotUnit> {
        let mut out = Vec::with_capacity(ids.len());
        for &id in ids {
            if self.world.entity_index(id).is_none() {
                continue;
            }
            if !self
                .world
                .ecs_get::<Identity>(id)
                .map(|identity| {
                    matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure)
                })
                .unwrap_or(false)
            {
                continue;
            }
            if let Some(unit) = self.project_entity(id) {
                out.push(unit);
            }
        }
        out
    }

    fn project_entity(&self, id: EntityId) -> Option<SnapshotUnit> {
        let identity = self.world.ecs_get::<Identity>(id)?;
        let owner = self.world.ecs_get::<Owner>(id)?;
        let xf = self.world.ecs_get::<Transform>(id).copied()?;
        let health = self.world.ecs_get::<Health>(id).copied()?;
        let hva_frame = self.world.ecs_get::<AnimationState>(id).map(|a| a.hva_frame).unwrap_or(0);
        let z = self.world.pass_grid.cell_height(xf.x, xf.y);
        let (sx, sy) = iso_to_screen(i32::from(xf.x), i32::from(xf.y), z);
        let deployable = !matches!(identity.kind, MapEntityKind::Structure)
            && crate::gameplay::deploy_into_type(&self.world.definitions, identity.type_id.as_ref()).is_some();
        let movement = self.world.ecs_get::<MovementState>(id);
        let move_goal_screen = movement.and_then(|m| {
            let dx = m.destination_x?;
            let dy = m.destination_y?;
            Some(self.cell_anchor_screen(dx, dy))
        });
        let attack_target = self.world.ecs_get::<AttackState>(id).and_then(|a| a.target);
        let attack_target_screen = attack_target.and_then(|tid| {
            let txf = self.world.ecs_get::<Transform>(tid).copied()?;
            Some(self.cell_anchor_screen(txf.x, txf.y))
        });
        let (foundation_w, foundation_h, art_height) = if matches!(identity.kind, MapEntityKind::Structure) {
            self.world
                .definitions
                .structures
                .get(identity.type_id.as_ref())
                .map(|s| (s.foundation.width, s.foundation.height, s.height.max(1)))
                .unwrap_or((1, 1, 2))
        }
        else {
            (0, 0, 0)
        };
        let bracket_delta = self.world.definitions.techno.get(identity.type_id.as_ref()).map(|t| t.pixel_selection_bracket_delta).unwrap_or(0);
        Some(SnapshotUnit {
            id,
            kind: identity.kind,
            type_id: identity.type_id.clone(),
            owner: owner.house.clone(),
            x: xf.x,
            y: xf.y,
            screen_x: sx - self.preview_origin_x,
            screen_y: sy - self.preview_origin_y,
            facing: xf.facing,
            turret_facing: xf.turret_facing,
            hva_frame,
            anim_state: derive_anim_state(&self.world, id),
            health: health.current,
            max_health: health.maximum,
            dead: health.dead,
            deployable,
            move_goal_screen,
            attack_target,
            attack_target_screen,
            foundation_w,
            foundation_h,
            art_height,
            bracket_delta,
        })
    }

    /// 逻辑格 → 预览图锚点（与选中环中心同口径：`iso` 后再加菱形视觉偏移）。
    fn cell_anchor_screen(&self, x: u16, y: u16) -> (i32, i32) {
        let z = self.world.pass_grid.cell_height(x, y);
        let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
        (sx - self.preview_origin_x + 30, sy - self.preview_origin_y + 15)
    }

    /// 从当前世界与本地选中构建一帧呈现快照。
    ///
    /// **原型路径**：每次全表扫描。单位投影经 [`Self::project_entity`]；
    /// 增量路径请优先使用脏集 + [`Self::project_units`]，HUD 用 [`Self::snapshot_hud`]。
    pub fn snapshot(&self, selected: &[EntityId]) -> RenderSnapshot {
        let units = self
            .world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if !self
                    .world
                    .ecs_get::<Identity>(id)
                    .map(|identity| {
                        matches!(
                            identity.kind,
                            MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
                        )
                    })
                    .unwrap_or(false)
                {
                    return None;
                }
                self.project_entity(id)
            })
            .collect();
        let hud = self.snapshot_hud();
        RenderSnapshot {
            edition: self.world.edition,
            tick: hud.tick,
            state_hash: self.world.state_hash(),
            units,
            players: hud.players,
            produce_queues: hud.produce_queues,
            last_rejects: hud.last_rejects,
            selected: selected.to_vec(),
            outcome: hud.outcome,
            paused: hud.paused,
            pause_reason: hud.pause_reason,
            battle_stats: hud.battle_stats,
            screen: if self.outcome.is_some() { SessionScreen::Results } else { SessionScreen::InBattle },
        }
    }

    /// 轻量 HUD 投影：玩家、队列、拒绝与结算，不含单位表。
    pub fn snapshot_hud(&self) -> HudSnapshot {
        let players = self
            .world
            .players
            .iter()
            .map(|p| SnapshotPlayer {
                house: p.house.clone(),
                funds: p.funds,
                power_output: p.power_output,
                power_drain: p.power_drain,
                low_power: p.low_power(),
            })
            .collect();
        let produce_queues = self
            .world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                let queue = self.world.ecs_get::<ProductionQueue>(id)?;
                if let Some((type_id, remaining_ticks)) = queue.item.as_ref() {
                    let total_ticks = self.world.definitions.techno.get(type_id.as_ref()).map(crate::gameplay::produce_ticks_for).unwrap_or(0);
                    return Some(SnapshotProduceQueue {
                        factory: id,
                        type_id: type_id.clone(),
                        remaining_ticks: *remaining_ticks,
                        total_ticks,
                        rally_x: queue.rally_x,
                        rally_y: queue.rally_y,
                    });
                }
                let ready = queue.ready.as_ref()?;
                let total_ticks = self.world.definitions.techno.get(ready.as_ref()).map(crate::gameplay::produce_ticks_for).unwrap_or(0);
                Some(SnapshotProduceQueue {
                    factory: id,
                    type_id: ready.clone(),
                    remaining_ticks: 0,
                    total_ticks,
                    rally_x: queue.rally_x,
                    rally_y: queue.rally_y,
                })
            })
            .collect();
        HudSnapshot {
            tick: self.world.tick,
            players,
            produce_queues,
            last_rejects: self.world.last_rejects().to_vec(),
            outcome: self.outcome.clone(),
            paused: self.paused,
            pause_reason: self.pause_reason.clone(),
            battle_stats: self.battle_stats.clone(),
        }
    }
}

fn derive_anim_state(world: &BattleState, id: EntityId) -> AnimState {
    if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
        return AnimState::Die;
    }
    if world.ecs_get::<AnimationState>(id).map(|a| a.hit_flash > 0).unwrap_or(false) {
        return AnimState::TakeDamage;
    }
    if world.ecs_get::<ProductionQueue>(id).map(|p| p.item.is_some()).unwrap_or(false) {
        return AnimState::Produce;
    }
    if world.ecs_get::<AttackState>(id).map(|a| a.target.is_some()).unwrap_or(false) {
        return AnimState::Attack;
    }
    if world.ecs_get::<MovementState>(id).map(|m| m.destination_x.is_some() || !m.path.is_empty()).unwrap_or(false) {
        return AnimState::Move;
    }
    AnimState::Idle
}
