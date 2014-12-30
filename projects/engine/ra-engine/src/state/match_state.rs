//! 确定性世界推进。不依赖渲染器与文件系统。

use std::{collections::HashSet, sync::Arc};

use ra_adaptor::{RulesDb, build_runtime_definitions};
use ra_assets::TechnoKind;
use ra_map::{MapInfo, PassGrid};
use ra_types::{CommandId, EntityId, GameEdition, PlayerId, RuntimeDefinitions, ScheduledCommand, TechnoClass, Tick};

use super::{
    components::{
        AnimationState, AttackState, CombatStats, EntitySpawnBundle, HarvesterState, Health, Identity, Locomotor,
        MovementState, Owner, ProductionQueue, Transform,
    },
    ecs_registry::EcsRegistry,
    entities::WorldEntity,
    players::PlayerState,
};
use crate::{
    game::{CommandReject, GameCommand, InputFrame},
    gameplay::verses_for,
    presentation::DirtyEntitySet,
    spatial::is_mobile,
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

/// 矿场完成一趟采矿所需的 tick 数（Alpha 简化，无独立采矿车）。
pub const ORE_TRIP_TICKS: u32 = 30;

/// 矿场每趟采矿给所属房主增加的资金。
pub const ORE_INCOME_PER_TRIP: u32 = 700;

/// 工厂完成一件生产所需的 tick 数（Alpha 简化）。
pub const PRODUCE_TICKS: u32 = 20;

/// 确定性仿真世界：实体、通行格与按 tick 消费的命令。
#[derive(Debug, Clone)]
pub struct MatchState {
    /// 当前游戏版本。
    pub edition: GameEdition,
    /// 已推进的逻辑 tick 计数。
    pub tick: u64,
    /// 地图信息（尺寸、放置实体等）。
    pub map: MapInfo,
    /// 通行格（由地图结构派生，可被重寻路使用）。
    pub pass_grid: PassGrid,
    /// 世界实体列表（与地图播种顺序对应）。
    pub entities: Vec<WorldEntity>,
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
    /// 内部 ECS 世界与 `EntityId` 映射（玩法字段仍以 `entities` 为权威）。
    pub(crate) ecs: EcsRegistry,
}

impl MatchState {
    /// 由规则与地图播种新世界，并为移动单位预计算路径。
    pub fn new(edition: GameEdition, rules: &RulesDb, map: MapInfo) -> Self {
        let pass_grid = PassGrid::from_map(&map);
        let definitions = Arc::new(build_runtime_definitions(rules));
        let mut next_entity_id = 1u64;
        let mut house_order: Vec<String> = Vec::new();
        let ecs = EcsRegistry::new();
        let mut seed_bundles: Vec<EntitySpawnBundle> = Vec::with_capacity(map.entities.len());
        for e in &map.entities {
            if !house_order.iter().any(|h| h == &e.owner) {
                house_order.push(e.owner.clone());
            }
            let tt = definitions.techno.get(&e.type_id);
            let max_health = tt.map(|t| t.strength).unwrap_or(1).max(1);
            let health = (u64::from(max_health) * u64::from(e.health) / 256) as u32;
            let speed = tt.map(|t| t.speed).unwrap_or(0);
            // 无 techno 定义时禁止发明默认射程/伤害（否则会变成可战斗幽灵单位）。
            let attack_range = tt.map(|t| if t.range > 0 { t.range } else { t.sight.max(1) }).unwrap_or(0);
            let attack_damage = tt.map(|t| if t.damage > 0 { t.damage } else { (t.strength / 4).max(1) }).unwrap_or(0);
            let attack_cooldown_max = tt.map(|t| if t.rof > 0 { t.rof } else { ATTACK_COOLDOWN_TICKS }).unwrap_or(0);
            let armor = tt.map(|t| t.armor.clone()).unwrap_or_else(|| "none".into());
            let attack_verses = tt.map(|t| verses_for(&definitions, &t.warhead)).unwrap_or([0; 11]);
            let techno_kind = tt.map(|t| techno_class_to_kind(t.class));
            let id = EntityId(next_entity_id);
            next_entity_id = next_entity_id.saturating_add(1);
            seed_bundles.push(EntitySpawnBundle {
                identity: Identity {
                    entity_id: id,
                    type_id: Arc::<str>::from(e.type_id.as_ref()),
                    kind: e.kind,
                },
                owner: Owner { house: Arc::<str>::from(e.owner.as_ref()) },
                transform: Transform {
                    x: e.x,
                    y: e.y,
                    facing: e.facing,
                    turret_facing: e.facing,
                    sub_cell: e.sub_cell,
                },
                health: Health { current: health, maximum: max_health, dead: false },
                locomotor: Locomotor { speed },
                movement: MovementState {
                    destination_x: None,
                    destination_y: None,
                    path: Vec::new(),
                    move_accum: 0,
                },
                combat: CombatStats {
                    armor,
                    attack_range,
                    attack_damage,
                    attack_cooldown_max,
                    attack_verses,
                    techno_kind,
                },
                attack: AttackState { target: None, cooldown: 0 },
                production: ProductionQueue { item: None, rally_x: None, rally_y: None },
                harvester: HarvesterState { ore_trip_accum: 0 },
                animation: AnimationState { hva_frame: 0, hit_flash: 0 },
            });
        }
        let players: Vec<PlayerState> =
            house_order.into_iter().enumerate().map(|(i, house)| PlayerState::new(PlayerId(i as u8), house)).collect();
        let mut world = Self {
            edition,
            tick: 0,
            map,
            pass_grid,
            entities: Vec::with_capacity(seed_bundles.len()),
            players,
            local_player: PlayerId(0),
            definitions,
            next_entity_id,
            next_command_id: 1,
            pending_commands: Vec::new(),
            last_input_frame: InputFrame::empty(0),
            last_rejects: Vec::new(),
            seen_command_ids: HashSet::new(),
            state_hash: 0,
            presentation_dirty: DirtyEntitySet::new(),
            ecs,
        };
        for bundle in seed_bundles {
            let id = bundle.identity.entity_id;
            let index = world.spawn_from_bundle(bundle);
            world.repath_entity_at(index);
            world.mark_entity_dirty(id);
        }
        world.rehash();
        world
    }

    /// 若存在同名 house，将 `local_player` 切到该玩家；否则保持原值并返回 `false`。
    pub fn prefer_local_house(&mut self, house: &str) -> bool {
        if let Some(p) = self.players.iter().find(|p| p.house.as_ref() == house) {
            self.local_player = p.id;
            true
        }
        else {
            false
        }
    }

    /// 标记实体对呈现层变脏。
    pub fn mark_entity_dirty(&mut self, id: EntityId) {
        self.presentation_dirty.mark(id);
    }

    /// 只读查看当前脏集（可能含重复）。
    pub fn presentation_dirty(&self) -> &DirtyEntitySet {
        &self.presentation_dirty
    }

    /// 取出并清空脏集（排序去重），供帧构建消费。
    pub fn take_presentation_dirty(&mut self) -> Vec<EntityId> {
        self.presentation_dirty.drain()
    }

    /// 入队命令载荷；自动包装为 [`ScheduledCommand`]（发出者为本地玩家，tick 为下一消费 tick）。
    pub fn push_command(&mut self, cmd: GameCommand) {
        self.push_player_command(self.local_player, cmd);
    }

    /// 以指定发出者入队命令载荷并包装调度信封。
    ///
    /// 信封 `player` 始终以本参数为准，**不得**被 `PlaceBuilding` / `Produce` 载荷内的 `player` 改写。
    /// 载荷与信封不一致时由 `apply_commands` 拒绝（`WrongOwner`）。
    pub fn push_player_command(&mut self, player: PlayerId, cmd: GameCommand) {
        let id = CommandId(self.next_command_id);
        self.next_command_id = self.next_command_id.saturating_add(1);
        let tick = Tick(self.tick.wrapping_add(1));
        self.pending_commands.push(ScheduledCommand::new(id, player, tick, cmd));
    }

    /// 直接入队已调度命令（测试 / 网络回放）。
    pub fn push_scheduled(&mut self, cmd: ScheduledCommand) {
        self.pending_commands.push(cmd);
    }

    /// 上一 tick 的输入帧（无操作时也为空命令列表）。
    pub fn last_input_frame(&self) -> &InputFrame {
        &self.last_input_frame
    }

    /// 上一 tick 产生的命令拒绝记录。
    pub fn last_rejects(&self) -> &[CommandReject] {
        &self.last_rejects
    }

    /// 按稳定 ID 查找实体下标。
    pub fn entity_index(&self, id: EntityId) -> Option<usize> {
        self.entities.iter().position(|e| e.id == id)
    }

    /// 分配下一枚稳定实体 ID（供后续生成建筑/单位使用）。
    pub fn alloc_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id = self.next_entity_id.saturating_add(1);
        id
    }

    /// 以 ECS 组件包生成实体：占投影槽 → 写权威组件 → 投影回槽位。
    pub(crate) fn spawn_from_bundle(&mut self, bundle: crate::state::components::EntitySpawnBundle) -> usize {
        let id = bundle.identity.entity_id;
        self.entities.push(WorldEntity::projection_slot(id));
        let index = self.entities.len() - 1;
        self.ecs.bind_bundle(id, bundle);
        self.project_entity_from_ecs(id);
        index
    }

    /// 将单个实体的全部 ECS 组件投影回 `WorldEntity`。
    pub(crate) fn project_entity_from_ecs(&mut self, id: EntityId) {
        let Some(handle) = self.ecs.resolve(id) else {
            return;
        };
        let Some(index) = self.entity_index(id) else {
            return;
        };
        let identity = self.ecs.world().get::<crate::state::components::Identity>(handle).cloned();
        let owner = self.ecs.world().get::<crate::state::components::Owner>(handle).cloned();
        let health = self.ecs.world().get::<crate::state::components::Health>(handle).copied();
        let transform = self.ecs.world().get::<crate::state::components::Transform>(handle).copied();
        let loco = self.ecs.world().get::<crate::state::components::Locomotor>(handle).copied();
        let movement = self.ecs.world().get::<crate::state::components::MovementState>(handle).cloned();
        let stats = self.ecs.world().get::<crate::state::components::CombatStats>(handle).cloned();
        let attack = self.ecs.world().get::<crate::state::components::AttackState>(handle).copied();
        let queue = self.ecs.world().get::<crate::state::components::ProductionQueue>(handle).cloned();
        let harvester = self.ecs.world().get::<crate::state::components::HarvesterState>(handle).copied();
        let anim = self.ecs.world().get::<crate::state::components::AnimationState>(handle).copied();

        let entity = &mut self.entities[index];
        if let Some(identity) = identity {
            entity.type_id = identity.type_id;
            entity.kind = identity.kind;
        }
        if let Some(owner) = owner {
            entity.owner = owner.house;
        }
        if let Some(health) = health {
            entity.health = health.current;
            entity.max_health = health.maximum;
            entity.dead = health.dead;
        }
        if let Some(transform) = transform {
            entity.x = transform.x;
            entity.y = transform.y;
            entity.facing = transform.facing;
            entity.turret_facing = transform.turret_facing;
            entity.sub_cell = transform.sub_cell;
        }
        if let Some(loco) = loco {
            entity.speed = loco.speed;
        }
        if let Some(movement) = movement {
            entity.target_x = movement.destination_x;
            entity.target_y = movement.destination_y;
            entity.path = movement.path;
            entity.move_accum = movement.move_accum;
        }
        if let Some(stats) = stats {
            entity.armor = stats.armor;
            entity.attack_range = stats.attack_range;
            entity.attack_damage = stats.attack_damage;
            entity.attack_cooldown_max = stats.attack_cooldown_max;
            entity.attack_verses = stats.attack_verses;
            entity.techno_kind = stats.techno_kind;
        }
        if let Some(attack) = attack {
            entity.attack_target = attack.target;
            entity.attack_cooldown = attack.cooldown;
        }
        if let Some(queue) = queue {
            entity.produce_queue = queue.item;
            entity.rally_x = queue.rally_x;
            entity.rally_y = queue.rally_y;
        }
        if let Some(harvester) = harvester {
            entity.ore_trip_accum = harvester.ore_trip_accum;
        }
        if let Some(anim) = anim {
            entity.hva_frame = anim.hva_frame;
            entity.hit_flash = anim.hit_flash;
        }
    }

    /// 按稳定 ID 只读取 ECS 组件（玩法系统应优先走此路径，而非读投影）。
    pub(crate) fn ecs_get<T: ra_ecs::Component>(&self, id: EntityId) -> Option<&T> {
        let handle = self.ecs.resolve(id)?;
        self.ecs.world().get::<T>(handle)
    }

    /// 把 ECS 权威组件投影回 `WorldEntity`（兼容尚未改读组件的路径）。
    ///
    /// 不再从 `WorldEntity` 回写 ECS，避免双真相。
    pub(crate) fn sync_ecs_components(&mut self) {
        let ids: Vec<EntityId> = self.entities.iter().map(|e| e.id).collect();
        for id in ids {
            self.project_entity_from_ecs(id);
        }
    }

    fn with_component_mut<T: ra_ecs::Component, R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        let handle = self.ecs.resolve(id)?;
        let result = {
            let component = self.ecs.world_mut().get_mut::<T>(handle)?;
            f(component)
        };
        self.project_entity_from_ecs(id);
        Some(result)
    }

    /// 以 ECS 为权威修改身份，并立即投影回 `WorldEntity`。
    pub(crate) fn with_identity_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::Identity) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改移动能力，并立即投影回 `WorldEntity`。
    pub(crate) fn with_locomotor_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::Locomotor) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改战斗参数，并立即投影回 `WorldEntity`。
    pub(crate) fn with_combat_stats_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::CombatStats) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改生命，并立即投影回 `WorldEntity`。
    pub(crate) fn with_health_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::Health) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改空间变换，并立即投影回 `WorldEntity`。
    pub(crate) fn with_transform_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::Transform) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改移动状态，并立即投影回 `WorldEntity`。
    pub(crate) fn with_movement_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::MovementState) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 为指定下标实体重算路径并写入 ECS `MovementState`。
    pub(crate) fn repath_entity_at(&mut self, index: usize) {
        let path = self.compute_repath_at(index);
        let id = self.entities[index].id;
        let _ = self.with_movement_mut(id, |movement| {
            movement.path = path;
        });
    }

    /// 以 ECS 为权威修改攻击状态，并立即投影回 `WorldEntity`。
    pub(crate) fn with_attack_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::AttackState) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改生产队列，并立即投影回 `WorldEntity`。
    pub(crate) fn with_production_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::ProductionQueue) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改采矿行程，并立即投影回 `WorldEntity`。
    pub(crate) fn with_harvester_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::HarvesterState) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改动画桥接状态，并立即投影回 `WorldEntity`。
    pub(crate) fn with_animation_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::AnimationState) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 稳定 ID 是否已在内部 ECS 注册且句柄仍有效。
    pub fn has_ecs_entity(&self, id: EntityId) -> bool {
        self.ecs.resolve(id).is_some()
    }

    /// 已注册的 ECS 映射条目数（应与 `entities.len()` 对齐）。
    pub fn ecs_registry_len(&self) -> usize {
        self.ecs.len()
    }

    /// ECS 内存中存活实体数（播种与生成路径上应与映射表一致）。
    pub fn ecs_alive_count(&self) -> usize {
        self.ecs.ecs_len()
    }

    /// 读取 ECS 中同步后的生命组件（迁移期诊断用）。
    pub fn ecs_health(&self, id: EntityId) -> Option<(u32, u32, bool)> {
        let handle = self.ecs.resolve(id)?;
        let health = self.ecs.world().get::<crate::state::components::Health>(handle)?;
        Some((health.current, health.maximum, health.dead))
    }

    /// 读取 ECS 中同步后的坐标（迁移期诊断用）。
    pub fn ecs_transform(&self, id: EntityId) -> Option<(u16, u16, u8)> {
        let handle = self.ecs.resolve(id)?;
        let transform = self.ecs.world().get::<crate::state::components::Transform>(handle)?;
        Some((transform.x, transform.y, transform.facing))
    }

    /// 读取 ECS 中同步后的移动目的地（迁移期诊断用）。
    pub fn ecs_move_destination(&self, id: EntityId) -> Option<(Option<u16>, Option<u16>)> {
        let handle = self.ecs.resolve(id)?;
        let movement = self.ecs.world().get::<crate::state::components::MovementState>(handle)?;
        Some((movement.destination_x, movement.destination_y))
    }

    /// 读取 ECS 中同步后的攻击目标与冷却（迁移期诊断用）。
    pub fn ecs_attack_state(&self, id: EntityId) -> Option<(Option<EntityId>, u32)> {
        let handle = self.ecs.resolve(id)?;
        let attack = self.ecs.world().get::<crate::state::components::AttackState>(handle)?;
        Some((attack.target, attack.cooldown))
    }

    /// 读取 ECS 中同步后的生产队列剩余 tick（迁移期诊断用）。
    pub fn ecs_produce_remaining(&self, id: EntityId) -> Option<Option<u32>> {
        let handle = self.ecs.resolve(id)?;
        let queue = self.ecs.world().get::<crate::state::components::ProductionQueue>(handle)?;
        Some(queue.item.as_ref().map(|(_, rem)| *rem))
    }

    /// 读取 ECS 中同步后的动画桥接字段（迁移期诊断用）。
    pub fn ecs_animation(&self, id: EntityId) -> Option<(u16, u32)> {
        let handle = self.ecs.resolve(id)?;
        let anim = self.ecs.world().get::<crate::state::components::AnimationState>(handle)?;
        Some((anim.hva_frame, anim.hit_flash))
    }

    /// 测试 / 调试：写入 ECS `Health` 并投影。
    pub fn set_ecs_health(&mut self, id: EntityId, current: u32, maximum: u32, dead: bool) -> bool {
        self.with_health_mut(id, |health| {
            health.current = current;
            health.maximum = maximum;
            health.dead = dead;
        })
        .is_some()
    }

    /// 测试 / 调试：写入 ECS 攻击参数并投影。
    pub fn set_ecs_attack_power(&mut self, id: EntityId, damage: u32, range: u32, cooldown_max: u32) -> bool {
        self.with_combat_stats_mut(id, |stats| {
            stats.attack_damage = damage;
            stats.attack_range = range;
            stats.attack_cooldown_max = cooldown_max;
        })
        .is_some()
    }

    /// 测试 / 调试：写入 ECS 身份类型并投影。
    pub fn set_ecs_type_id(&mut self, id: EntityId, type_id: impl Into<std::sync::Arc<str>>, kind: ra_map::MapEntityKind) -> bool {
        let type_id = type_id.into();
        self.with_identity_mut(id, |identity| {
            identity.type_id = type_id;
            identity.kind = kind;
        })
        .is_some()
    }

    /// 测试 / 调试：写入 ECS 移动速度并投影。
    pub fn set_ecs_speed(&mut self, id: EntityId, speed: u32) -> bool {
        self.with_locomotor_mut(id, |loco| {
            loco.speed = speed;
        })
        .is_some()
    }

    /// 测试 / 调试：清空 ECS 移动目的地与路径并投影。
    pub fn clear_ecs_movement(&mut self, id: EntityId) -> bool {
        self.with_movement_mut(id, |movement| {
            movement.destination_x = None;
            movement.destination_y = None;
            movement.path.clear();
            movement.move_accum = 0;
        })
        .is_some()
    }

    /// 按 house 名称设置资金（启动与测试播种用）。
    pub fn set_house_funds(&mut self, house: &str, funds: i32) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house) {
            player.funds = funds;
            self.rehash();
            true
        }
        else {
            false
        }
    }

    /// 将所有玩家资金设为同一起始值（遭遇战大厅资金滑条）。
    pub fn set_all_players_funds(&mut self, funds: i32) {
        if self.players.is_empty() {
            return;
        }
        for player in &mut self.players {
            player.funds = funds;
        }
        self.rehash();
    }

    /// 按 house 名称读取资金。
    pub fn house_funds(&self, house: &str) -> Option<i32> {
        self.players.iter().find(|p| p.house.as_ref() == house).map(|p| p.funds)
    }

    /// 查询规则造价；未知类型为 `None`。
    pub fn techno_cost(&self, type_id: &str) -> Option<u32> {
        self.definitions.techno.get(type_id).map(|t| t.cost.max(0) as u32)
    }

    /// 推进一个逻辑 tick（使用默认 [`SystemSchedule`]）。
    pub fn advance_tick(&mut self) {
        self.advance_scheduled_tick(&crate::engine::SystemSchedule::standard());
    }

    /// 按调度表推进一个逻辑 tick（阶段顺序的唯一来源）。
    ///
    /// 只消费 `tick <= 当前 tick` 的待执行命令。更晚的调度命令留在队列中，禁止提前执行。
    pub fn advance_scheduled_tick(&mut self, schedule: &crate::engine::SystemSchedule) {
        use crate::engine::SystemPhase;

        self.tick = self.tick.wrapping_add(1);
        let mut due = Vec::new();
        let mut deferred = Vec::new();
        for cmd in std::mem::take(&mut self.pending_commands) {
            if cmd.tick.0 <= self.tick {
                due.push(cmd);
            }
            else {
                deferred.push(cmd);
            }
        }
        self.pending_commands = deferred;
        self.last_input_frame = InputFrame { tick: self.tick, commands: due.clone() };
        self.last_rejects.clear();
        for phase in &schedule.phases {
            match *phase {
                SystemPhase::ApplyCommands => self.apply_commands(&due),
                SystemPhase::Movement => self.advance_movement(),
                SystemPhase::HitFlash => self.tick_hit_flash(),
                SystemPhase::Combat => self.resolve_combat(),
                SystemPhase::Turrets => self.advance_turrets(),
                SystemPhase::RefineryIncome => self.advance_refinery_income(),
                SystemPhase::Production => self.advance_production(),
                SystemPhase::Rehash => {
                    self.sync_ecs_components();
                    self.rehash();
                }
            }
        }
    }

    /// 目标格是否可放置单格建筑（界内、可通行、无占用实体）。
    pub fn can_place_structure(&self, x: u16, y: u16) -> bool {
        use crate::state::components::{Health, Transform};

        if !self.pass_grid.in_bounds(x, y) {
            return false;
        }
        if !self.pass_grid.is_passable(x, y) {
            return false;
        }
        !self.entities.iter().any(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Transform>(id).map(|t| t.x == x && t.y == y).unwrap_or(false)
        })
    }

    /// 当前确定性状态哈希（锁步校验用）。
    pub fn state_hash(&self) -> u64 {
        self.state_hash
    }

    /// 已绑定 techno 规则的实体数量。
    pub fn bound_techno_count(&self) -> usize {
        use crate::state::components::CombatStats;

        self.entities
            .iter()
            .filter(|e| self.ecs_get::<CombatStats>(e.id).and_then(|s| s.techno_kind).is_some())
            .count()
    }

    /// 通行表变更后，为全部移动单位重算路径。
    pub fn repath_mobiles(&mut self) {
        use crate::state::components::Identity;

        for i in 0..self.entities.len() {
            let id = self.entities[i].id;
            if self.ecs_get::<Identity>(id).map(|identity| is_mobile(identity.kind)).unwrap_or(false) {
                self.repath_entity_at(i);
            }
        }
        self.rehash();
    }
}

fn techno_class_to_kind(class: TechnoClass) -> TechnoKind {
    match class {
        TechnoClass::Infantry => TechnoKind::Infantry,
        TechnoClass::Vehicle => TechnoKind::Vehicle,
        TechnoClass::Aircraft => TechnoKind::Aircraft,
        TechnoClass::Building => TechnoKind::Building,
    }
}
