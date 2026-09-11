use super::{
    super::components::{Identity, Owner},
    types::{BattleState, EcsCombatView},
};
use ra_types::EntityId;

impl BattleState {
    /// 按稳定 ID 查找实体下标。
    pub fn entity_index(&self, id: EntityId) -> Option<usize> {
        self.entities.iter().position(|e| e.id == id)
    }

    /// 投影槽实体数量（应与 ECS 映射对齐）。
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// 按下标取稳定实体 ID。
    pub fn entity_id_at(&self, index: usize) -> Option<EntityId> {
        self.entities.get(index).map(|e| e.id)
    }

    /// 全部稳定实体 ID（投影槽顺序）。
    pub fn entity_ids(&self) -> Vec<EntityId> {
        self.entities.iter().map(|e| e.id).collect()
    }

    /// 按房主与类型键查找首个实体 ID。
    pub fn find_entity_id_by_owner_type(&self, owner: &str, type_id: &str) -> Option<EntityId> {
        self.entities.iter().find_map(|e| {
            let id = e.id;
            let house_ok = self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == owner).unwrap_or(false);
            let type_ok = self.ecs_get::<Identity>(id).map(|i| i.type_id.as_ref() == type_id).unwrap_or(false);
            (house_ok && type_ok).then_some(id)
        })
    }

    /// 按类型键查找首个实体 ID。
    pub fn find_entity_id_by_type(&self, type_id: &str) -> Option<EntityId> {
        self.entities.iter().find_map(|e| {
            let id = e.id;
            self.ecs_get::<Identity>(id).filter(|i| i.type_id.as_ref() == type_id).map(|_| id)
        })
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

    /// 读取 ECS `Health`（测试与诊断）。
    pub fn ecs_health(&self, id: EntityId) -> Option<(u32, u32, bool)> {
        let health = self.ecs_get::<crate::state::components::Health>(id)?;
        Some((health.current, health.maximum, health.dead))
    }

    /// 建筑是否处于侧栏扳手持续修理中（测试与诊断）。
    pub fn is_repairing(&self, id: EntityId) -> bool {
        self.ecs_get::<crate::state::components::Repairing>(id).is_some()
    }

    /// 读取 ECS `Identity`（测试与诊断）。
    pub fn ecs_identity(&self, id: EntityId) -> Option<(std::sync::Arc<str>, ra_map::MapEntityKind)> {
        let identity = self.ecs_get::<crate::state::components::Identity>(id)?;
        Some((std::sync::Arc::clone(&identity.type_id), identity.kind))
    }

    /// 读取 ECS `Identity.mission`（地图放置任务态；测试与诊断）。
    pub fn ecs_mission(&self, id: EntityId) -> Option<String> {
        self.ecs_get::<crate::state::components::Identity>(id).map(|i| i.mission.clone())
    }

    /// 读取 ECS `Owner` 房主名（测试与诊断）。
    pub fn ecs_owner(&self, id: EntityId) -> Option<std::sync::Arc<str>> {
        self.ecs_get::<crate::state::components::Owner>(id).map(|o| std::sync::Arc::clone(&o.house))
    }

    /// 读取 ECS `Transform` 坐标与车身朝向（测试与诊断）。
    pub fn ecs_transform(&self, id: EntityId) -> Option<(u16, u16, u8)> {
        let transform = self.ecs_get::<crate::state::components::Transform>(id)?;
        Some((transform.x, transform.y, transform.facing))
    }

    /// 读取 ECS 炮塔朝向（测试与诊断）。
    pub fn ecs_turret_facing(&self, id: EntityId) -> Option<u8> {
        self.ecs_get::<crate::state::components::Transform>(id).map(|t| t.turret_facing)
    }

    /// 读取 ECS `Locomotor` 速度（测试与诊断）。
    pub fn ecs_speed(&self, id: EntityId) -> Option<u32> {
        self.ecs_get::<crate::state::components::Locomotor>(id).map(|l| l.speed)
    }

    /// 读取 ECS `MovementState` 目的地（测试与诊断）。
    pub fn ecs_move_destination(&self, id: EntityId) -> Option<(Option<u16>, Option<u16>)> {
        let movement = self.ecs_get::<crate::state::components::MovementState>(id)?;
        Some((movement.destination_x, movement.destination_y))
    }

    /// 读取 ECS 移动路径（测试与诊断）。
    pub fn ecs_path(&self, id: EntityId) -> Option<Vec<(u16, u16)>> {
        self.ecs_get::<crate::state::components::MovementState>(id).map(|m| m.path.clone())
    }

    /// 读取 ECS `MovementState::waypoints`（路径点规划剩余航点）。
    pub fn ecs_waypoints(&self, id: EntityId) -> Option<Vec<(u16, u16)>> {
        self.ecs_get::<crate::state::components::MovementState>(id).map(|m| m.waypoints.clone())
    }

    /// 读取 ECS `MovementState::move_accum`（格内滑移进度）。
    pub fn ecs_move_accum(&self, id: EntityId) -> Option<u32> {
        self.ecs_get::<crate::state::components::MovementState>(id).map(|m| m.move_accum)
    }

    /// 读取 ECS `AttackState`（测试与诊断）。
    pub fn ecs_attack_state(&self, id: EntityId) -> Option<(Option<EntityId>, u32)> {
        let attack = self.ecs_get::<crate::state::components::AttackState>(id)?;
        Some((attack.target, attack.cooldown))
    }

    /// 读取 ECS 战斗静态参数（测试与诊断）。
    pub fn ecs_combat_view(&self, id: EntityId) -> Option<EcsCombatView> {
        let stats = self.ecs_get::<crate::state::components::CombatStats>(id)?;
        Some(EcsCombatView {
            armor: stats.armor.clone(),
            attack_range: stats.attack_range,
            attack_damage: stats.attack_damage,
            attack_cooldown_max: stats.attack_cooldown_max,
            attack_verses: stats.attack_verses,
            techno_class: stats.techno_class,
        })
    }

    /// 读取 ECS 生产队列剩余 tick（测试与诊断）。
    pub fn ecs_produce_remaining(&self, id: EntityId) -> Option<Option<u32>> {
        let queue = self.ecs_get::<crate::state::components::ProductionQueue>(id)?;
        Some(queue.item.as_ref().map(|(_, rem)| *rem))
    }

    /// 读取 ECS 生产队列条目（测试与诊断）。
    pub fn ecs_produce_item(&self, id: EntityId) -> Option<Option<(std::sync::Arc<str>, u32)>> {
        self.ecs_get::<crate::state::components::ProductionQueue>(id).map(|q| q.item.clone())
    }

    /// 读取 ECS 集结格（测试与诊断）。
    pub fn ecs_rally(&self, id: EntityId) -> Option<(Option<u16>, Option<u16>)> {
        let queue = self.ecs_get::<crate::state::components::ProductionQueue>(id)?;
        Some((queue.rally_x, queue.rally_y))
    }

    /// 读取 ECS `AnimationState`（测试与诊断）。
    pub fn ecs_animation(&self, id: EntityId) -> Option<(u16, u32)> {
        let anim = self.ecs_get::<crate::state::components::AnimationState>(id)?;
        Some((anim.hva_frame, anim.hit_flash))
    }

    pub(super) fn living_at_cell(&self, x: u16, y: u16) -> bool {
        use crate::state::components::{Health, Transform};

        self.entities.iter().any(|e| {
            let dead = self.ecs_get::<Health>(e.id).map(|h| h.dead).unwrap_or(true);
            if dead {
                return false;
            }
            self.ecs_get::<Transform>(e.id).is_some_and(|t| t.x == x && t.y == y)
        })
    }
}
