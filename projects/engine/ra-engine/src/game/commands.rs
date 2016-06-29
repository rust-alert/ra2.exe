//! 玩家/AI 注入的确定性命令（按 tick 排序消费）。
//!
//! 对外调度形状为 [`ScheduledCommand`]；[`GameCommand`] 是其中的可执行载荷（与 `ra_types::CommandBody` 同一类型）。

use ra_types::{CommandBody, CommandId, EntityId, PlayerId, ScheduledCommand, Tick};
use std::sync::Arc;

/// 可执行命令载荷（跨层与 `ra_types::CommandBody` 共用）。
pub type GameCommand = CommandBody;

/// 一个仿真 tick 的完整输入帧。
///
/// 联机锁步要求：**每个 tick 都必须有一帧**；本 tick 无操作时 `commands` 为空。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InputFrame {
    /// 本帧对应的逻辑 tick。
    pub tick: u64,
    /// 本 tick 消费的已调度命令（可为空）。
    pub commands: Vec<ScheduledCommand>,
}

impl InputFrame {
    /// 构造仅含 tick、无命令的空输入帧。
    pub fn empty(tick: u64) -> Self {
        Self { tick, commands: Vec::new() }
    }

    /// 本帧是否没有任何命令。
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// 编码单条命令载荷为网络 `payload` 字节（不含调度信封）。
pub fn encode_command(cmd: &GameCommand) -> Vec<u8> {
    let mut b = Vec::new();
    match *cmd {
        GameCommand::MoveTo { entity, x, y } => {
            b.push(1);
            b.extend_from_slice(&entity.0.to_be_bytes());
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::MovePath { entity, ref points } => {
            b.push(13);
            b.extend_from_slice(&entity.0.to_be_bytes());
            let n = points.len().min(u16::MAX as usize) as u16;
            b.extend_from_slice(&n.to_be_bytes());
            for &(x, y) in points.iter().take(n as usize) {
                b.extend_from_slice(&x.to_be_bytes());
                b.extend_from_slice(&y.to_be_bytes());
            }
        }
        GameCommand::Attack { attacker, target } => {
            b.push(2);
            b.extend_from_slice(&attacker.0.to_be_bytes());
            b.extend_from_slice(&target.0.to_be_bytes());
        }
        GameCommand::Deploy { entity } => {
            b.push(3);
            b.extend_from_slice(&entity.0.to_be_bytes());
        }
        GameCommand::PlaceBuilding { player, ref type_id, x, y } => {
            b.push(4);
            b.push(player.0);
            let id_bytes = type_id.as_bytes();
            b.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
            b.extend_from_slice(id_bytes);
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::Produce { player, ref type_id } => {
            b.push(5);
            b.push(player.0);
            let id_bytes = type_id.as_bytes();
            b.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
            b.extend_from_slice(id_bytes);
        }
        GameCommand::SetRallyPoint { factory, x, y } => {
            b.push(6);
            b.extend_from_slice(&factory.0.to_be_bytes());
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::Infiltrate { agent, building } => {
            b.push(7);
            b.extend_from_slice(&agent.0.to_be_bytes());
            b.extend_from_slice(&building.0.to_be_bytes());
        }
        GameCommand::CancelProduce { player, ref type_id } => {
            b.push(8);
            b.push(player.0);
            let id_bytes = type_id.as_bytes();
            b.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
            b.extend_from_slice(id_bytes);
        }
        GameCommand::CaptureBuilding { engineer, building } => {
            b.push(9);
            b.extend_from_slice(&engineer.0.to_be_bytes());
            b.extend_from_slice(&building.0.to_be_bytes());
        }
        GameCommand::Guard { entity } => {
            b.push(10);
            b.extend_from_slice(&entity.0.to_be_bytes());
        }
        GameCommand::SellBuilding { player, building } => {
            b.push(11);
            b.push(player.0);
            b.extend_from_slice(&building.0.to_be_bytes());
        }
        GameCommand::RepairBuilding { player, building } => {
            b.push(12);
            b.push(player.0);
            b.extend_from_slice(&building.0.to_be_bytes());
        }
    }
    b
}

/// 解码单条命令；失败返回 `None`。
pub fn decode_command(bytes: &[u8]) -> Option<GameCommand> {
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        1 => {
            if bytes.len() < 1 + 8 + 2 + 2 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let x = u16::from_be_bytes(bytes[9..11].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[11..13].try_into().ok()?);
            Some(GameCommand::MoveTo { entity, x, y })
        }
        2 => {
            if bytes.len() < 1 + 8 + 8 {
                return None;
            }
            let attacker = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let target = EntityId(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
            Some(GameCommand::Attack { attacker, target })
        }
        3 => {
            if bytes.len() < 1 + 8 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            Some(GameCommand::Deploy { entity })
        }
        4 => {
            if bytes.len() < 1 + 1 + 2 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let id_len = u16::from_be_bytes(bytes[2..4].try_into().ok()?) as usize;
            if bytes.len() < 1 + 1 + 2 + id_len + 2 + 2 {
                return None;
            }
            let type_id = std::str::from_utf8(&bytes[4..4 + id_len]).ok()?.to_string();
            let xy = 4 + id_len;
            let x = u16::from_be_bytes(bytes[xy..xy + 2].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[xy + 2..xy + 4].try_into().ok()?);
            Some(GameCommand::PlaceBuilding { player, type_id, x, y })
        }
        5 => {
            if bytes.len() < 1 + 1 + 2 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let id_len = u16::from_be_bytes(bytes[2..4].try_into().ok()?) as usize;
            if bytes.len() < 1 + 1 + 2 + id_len {
                return None;
            }
            let type_id = std::str::from_utf8(&bytes[4..4 + id_len]).ok()?.to_string();
            Some(GameCommand::Produce { player, type_id })
        }
        6 => {
            if bytes.len() < 1 + 8 + 2 + 2 {
                return None;
            }
            let factory = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let x = u16::from_be_bytes(bytes[9..11].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[11..13].try_into().ok()?);
            Some(GameCommand::SetRallyPoint { factory, x, y })
        }
        7 => {
            if bytes.len() < 1 + 8 + 8 {
                return None;
            }
            let agent = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let building = EntityId(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
            Some(GameCommand::Infiltrate { agent, building })
        }
        8 => {
            if bytes.len() < 1 + 1 + 2 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let id_len = u16::from_be_bytes(bytes[2..4].try_into().ok()?) as usize;
            if bytes.len() < 1 + 1 + 2 + id_len {
                return None;
            }
            let type_id = std::str::from_utf8(&bytes[4..4 + id_len]).ok()?.to_string();
            Some(GameCommand::CancelProduce { player, type_id })
        }
        9 => {
            if bytes.len() < 1 + 8 + 8 {
                return None;
            }
            let engineer = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let building = EntityId(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
            Some(GameCommand::CaptureBuilding { engineer, building })
        }
        10 => {
            if bytes.len() < 1 + 8 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            Some(GameCommand::Guard { entity })
        }
        11 => {
            if bytes.len() < 1 + 1 + 8 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let building = EntityId(u64::from_be_bytes(bytes[2..10].try_into().ok()?));
            Some(GameCommand::SellBuilding { player, building })
        }
        12 => {
            if bytes.len() < 1 + 1 + 8 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let building = EntityId(u64::from_be_bytes(bytes[2..10].try_into().ok()?));
            Some(GameCommand::RepairBuilding { player, building })
        }
        13 => {
            if bytes.len() < 1 + 8 + 2 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let n = u16::from_be_bytes(bytes[9..11].try_into().ok()?) as usize;
            let need = 1 + 8 + 2 + n * 4;
            if bytes.len() < need {
                return None;
            }
            let mut points = Vec::with_capacity(n);
            let mut off = 11;
            for _ in 0..n {
                let x = u16::from_be_bytes(bytes[off..off + 2].try_into().ok()?);
                let y = u16::from_be_bytes(bytes[off + 2..off + 4].try_into().ok()?);
                points.push((x, y));
                off += 4;
            }
            Some(GameCommand::MovePath { entity, points })
        }
        _ => None,
    }
}

/// 编码命令列表：`[u16 count][cmd…]`（每条前缀 `u16` 长度）。
pub fn encode_commands(cmds: &[GameCommand]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&(cmds.len() as u16).to_be_bytes());
    for cmd in cmds {
        let one = encode_command(cmd);
        b.extend_from_slice(&(one.len() as u16).to_be_bytes());
        b.extend_from_slice(&one);
    }
    b
}

/// 解码命令载荷列表。
pub fn decode_commands(bytes: &[u8]) -> Option<Vec<GameCommand>> {
    if bytes.len() < 2 {
        return None;
    }
    let count = u16::from_be_bytes(bytes[0..2].try_into().ok()?) as usize;
    let mut i = 2usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if i + 2 > bytes.len() {
            return None;
        }
        let n = u16::from_be_bytes(bytes[i..i + 2].try_into().ok()?) as usize;
        i += 2;
        if i + n > bytes.len() {
            return None;
        }
        let cmd = decode_command(&bytes[i..i + n])?;
        out.push(cmd);
        i += n;
    }
    Some(out)
}

/// 编码已调度命令：信封 + 载荷。
pub fn encode_scheduled(cmd: &ScheduledCommand) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&cmd.id.0.to_be_bytes());
    b.push(cmd.player.0);
    b.extend_from_slice(&cmd.tick.0.to_be_bytes());
    let body = encode_command(&cmd.body);
    b.extend_from_slice(&(body.len() as u16).to_be_bytes());
    b.extend_from_slice(&body);
    b
}

/// 解码已调度命令。
pub fn decode_scheduled(bytes: &[u8]) -> Option<ScheduledCommand> {
    if bytes.len() < 8 + 1 + 8 + 2 {
        return None;
    }
    let id = CommandId(u64::from_be_bytes(bytes[0..8].try_into().ok()?));
    let player = PlayerId(bytes[8]);
    let tick = Tick(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
    let n = u16::from_be_bytes(bytes[17..19].try_into().ok()?) as usize;
    if bytes.len() < 19 + n {
        return None;
    }
    let body = decode_command(&bytes[19..19 + n])?;
    Some(ScheduledCommand::new(id, player, tick, body))
}

impl crate::state::BattleState {
    pub(crate) fn apply_commands(&mut self, cmds: &[ScheduledCommand]) {
        use ra_assets::TechnoKind;
        use ra_map::MapEntityKind;
        use ra_types::TechnoClass;

        use crate::{
            game::CommandRejectReason,
            gameplay::{
                building_power, build_limit_reached, deploy_into_type, full_verses, is_agent, is_capturable,
                is_construction_yard, is_engineer, is_production_factory, is_type_eligible, living_structure_keys,
                produce_ticks_for, requires_power_plant, TechTreePlayer,
            },
            spatial::{is_mobile, nearest_adjacent_to_footprint},
            state::components::{
                AnimationState, AttackState, CombatStats, EntitySpawnBundle, HarvesterState, Health, Identity,
                Locomotor, MovementState, Owner, ProductionQueue, Transform,
            },
        };

        for (command_index, scheduled) in cmds.iter().enumerate() {
            if !self.seen_command_ids.insert(scheduled.id.0) {
                self.reject(command_index, CommandRejectReason::DuplicateCommand);
                continue;
            }
            match scheduled.body.clone() {
                GameCommand::MoveTo { entity, x, y } => {
                    let Some(entity_index) = self.entity_index(entity)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let id = self.entities[entity_index].id;
                    if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, entity_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    if !self.ecs_get::<Identity>(id).map(|i| is_mobile(i.kind)).unwrap_or(false) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    let _ = self.with_identity_mut(id, |identity| {
                        identity.mission.clear();
                    });
                    let _ = self.with_attack_mut(id, |attack| {
                        attack.target = None;
                        attack.infiltrate_target = None;
                        attack.capture_target = None;
                    });
                    let _ = self.with_movement_mut(id, |movement| {
                        movement.destination_x = Some(x);
                        movement.destination_y = Some(y);
                        movement.waypoints.clear();
                        movement.path.clear();
                        movement.move_accum = 0;
                    });
                    self.repath_entity_at(entity_index);
                }
                GameCommand::MovePath { entity, ref points } => {
                    let Some(entity_index) = self.entity_index(entity)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let id = self.entities[entity_index].id;
                    if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, entity_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    if !self.ecs_get::<Identity>(id).map(|i| is_mobile(i.kind)).unwrap_or(false) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    let Some(&(x, y)) = points.first()
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    let rest: Vec<(u16, u16)> = points.iter().skip(1).copied().collect();
                    let _ = self.with_identity_mut(id, |identity| {
                        identity.mission.clear();
                    });
                    let _ = self.with_attack_mut(id, |attack| {
                        attack.target = None;
                        attack.infiltrate_target = None;
                        attack.capture_target = None;
                    });
                    let _ = self.with_movement_mut(id, |movement| {
                        movement.destination_x = Some(x);
                        movement.destination_y = Some(y);
                        movement.waypoints = rest;
                        movement.path.clear();
                        movement.move_accum = 0;
                    });
                    self.repath_entity_at(entity_index);
                }
                GameCommand::Attack { attacker, target } => {
                    let Some(attacker_index) = self.entity_index(attacker)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let Some(target_index) = self.entity_index(target)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    if attacker == target {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let attacker_id = self.entities[attacker_index].id;
                    let target_id = self.entities[target_index].id;
                    if self.ecs_get::<Health>(attacker_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, attacker_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    if self.ecs_get::<Health>(target_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !self.ecs_get::<Identity>(attacker_id).map(|i| is_mobile(i.kind)).unwrap_or(false) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    let Some(target_xf) = self.ecs_get::<Transform>(target_id).copied()
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    let _ = self.with_identity_mut(attacker_id, |identity| {
                        identity.mission.clear();
                    });
                    let _ = self.with_attack_mut(attacker_id, |attack| {
                        attack.target = Some(target);
                        attack.infiltrate_target = None;
                        attack.capture_target = None;
                    });
                    let _ = self.with_movement_mut(attacker_id, |movement| {
                        movement.destination_x = Some(target_xf.x);
                        movement.destination_y = Some(target_xf.y);
                        movement.waypoints.clear();
                        movement.path.clear();
                        movement.move_accum = 0;
                    });
                    self.repath_entity_at(attacker_index);
                }
                GameCommand::Deploy { entity } => {
                    let Some(entity_index) = self.entity_index(entity)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let dirty_id = self.entities[entity_index].id;
                    if self.ecs_get::<Health>(dirty_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, entity_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(type_id) = self.ecs_get::<Identity>(dirty_id).map(|i| i.type_id.clone())
                    else {
                        self.reject(command_index, CommandRejectReason::CannotDeploy);
                        continue;
                    };
                    let Some(building_type) = deploy_into_type(&self.definitions, &type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::CannotDeploy);
                        continue;
                    };
                    let armor = self.definitions.techno.get(building_type).map(|t| t.armor.clone()).unwrap_or_else(|| "none".into());
                    let building_type = Arc::<str>::from(building_type);
                    let _ = self.with_identity_mut(dirty_id, |identity| {
                        identity.kind = MapEntityKind::Structure;
                        identity.type_id = Arc::clone(&building_type);
                        identity.mission.clear();
                    });
                    let _ = self.with_locomotor_mut(dirty_id, |loco| {
                        loco.speed = 0;
                    });
                    let _ = self.with_combat_stats_mut(dirty_id, |stats| {
                        stats.attack_range = 0;
                        stats.attack_damage = 0;
                        stats.attack_verses = full_verses();
                        stats.armor = armor;
                        stats.techno_kind = Some(TechnoKind::Building);
                    });
                    let _ = self.with_animation_mut(dirty_id, |anim| {
                        anim.hva_frame = 0;
                    });
                    let _ = self.with_attack_mut(dirty_id, |attack| {
                        attack.target = None;
                        attack.cooldown = 0;
                    });
                    let _ = self.with_movement_mut(dirty_id, |movement| {
                        movement.destination_x = None;
                        movement.destination_y = None;
                        movement.waypoints.clear();
                        movement.path.clear();
                        movement.move_accum = 0;
                    });
                    self.mark_entity_dirty(dirty_id);
                    // 展开后离开移动单位层，需烤进建筑底图（含 AI 部署，不依赖本机 deploy_watch）。
                    self.structure_paint_dirty.push(dirty_id);
                }
                GameCommand::PlaceBuilding { player, ref type_id, x, y } => {
                    if player != scheduled.player {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(player_index) = self.players.iter().position(|p| p.id == player)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let house = self.players[player_index].house.clone();
                    let tech_player = TechTreePlayer::from_player(&self.players[player_index]);
                    let Some(tt) = self.definitions.techno.get(type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    };
                    if tt.class != TechnoClass::Building {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    if is_construction_yard(&self.definitions, type_id) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    if !self.house_has_living_yard(&house) {
                        self.reject(command_index, CommandRejectReason::MissingPrerequisite);
                        continue;
                    }
                    let living = living_structure_keys(self, house.as_ref());
                    if !is_type_eligible(&self.definitions, tech_player, &living, type_id) {
                        self.reject(command_index, CommandRejectReason::MissingPrerequisite);
                        continue;
                    }
                    if build_limit_reached(self, house.as_ref(), tt) {
                        self.reject(command_index, CommandRejectReason::QueueFull);
                        continue;
                    }
                    if requires_power_plant(&self.definitions, type_id) && !self.house_has_living_power(&house) {
                        self.reject(command_index, CommandRejectReason::InsufficientPower);
                        continue;
                    }
                    let foundation = self
                        .definitions
                        .structures
                        .get(type_id)
                        .map(|s| s.foundation.clone())
                        .unwrap_or_default();
                    if !self.can_place_structure_footprint(x, y, foundation.width, foundation.height) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    let cost = tt.cost;
                    if self.players[player_index].funds < cost {
                        self.reject(command_index, CommandRejectReason::InsufficientFunds);
                        continue;
                    }
                    let power = building_power(&self.definitions, type_id);
                    let max_health = tt.strength.max(1);
                    let armor = tt.armor.clone();
                    let id = self.alloc_entity_id();
                    self.players[player_index].funds -= cost;
                    self.players[player_index].funds_spent = self.players[player_index].funds_spent.saturating_add(cost);
                    self.players[player_index].power_output = self.players[player_index].power_output.saturating_add(power.output);
                    self.players[player_index].power_drain = self.players[player_index].power_drain.saturating_add(power.drain);
                    self.players[player_index].built = self.players[player_index].built.saturating_add(1);
                    self.seal_structure_footprint(x, y, foundation.width, foundation.height);
                    self.spawn_from_bundle(EntitySpawnBundle {
                        identity: Identity {
                            entity_id: id,
                            type_id: Arc::<str>::from(type_id.to_ascii_uppercase()),
                            kind: MapEntityKind::Structure,
                            mission: String::new(),
                            tag: String::new(),
                        },
                        owner: Owner { house },
                        transform: Transform { x, y, facing: 0, turret_facing: 0, sub_cell: 0 },
                        health: Health { current: max_health, maximum: max_health, dead: false },
                        locomotor: Locomotor { speed: 0 },
                        movement: MovementState {
                            destination_x: None,
                            destination_y: None,
                            waypoints: Vec::new(),
                            path: Vec::new(),
                            move_accum: 0,
                        },
                        combat: CombatStats {
                            armor,
                            attack_range: 0,
                            attack_damage: 0,
                            attack_cooldown_max: 0,
                            attack_verses: full_verses(),
                            techno_kind: Some(TechnoKind::Building),
                        },
                        attack: AttackState { target: None, cooldown: 0, infiltrate_target: None, capture_target: None },
                        production: ProductionQueue { item: None, rally_x: None, rally_y: None },
                        harvester: HarvesterState { ore_trip_accum: 0, cargo: 0 },
                        animation: AnimationState { hva_frame: 0, hit_flash: 0 },
                    });
                    self.mark_entity_dirty(id);
                    // 新建筑只在 ECS 落地时不会出现在移动单位重绘层，必须进建筑脏绘。
                    self.structure_paint_dirty.push(id);
                }
                GameCommand::Produce { player, ref type_id } => {
                    if player != scheduled.player {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(player_index) = self.players.iter().position(|p| p.id == player)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let house = self.players[player_index].house.clone();
                    let tech_player = TechTreePlayer::from_player(&self.players[player_index]);
                    let Some(tt) = self.definitions.techno.get(type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    };
                    if !matches!(tt.class, TechnoClass::Infantry | TechnoClass::Vehicle) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    let living = living_structure_keys(self, house.as_ref());
                    if !is_type_eligible(&self.definitions, tech_player, &living, type_id) {
                        self.reject(command_index, CommandRejectReason::MissingPrerequisite);
                        continue;
                    }
                    if build_limit_reached(self, house.as_ref(), tt) {
                        self.reject(command_index, CommandRejectReason::QueueFull);
                        continue;
                    }
                    let kind = match tt.class {
                        TechnoClass::Infantry => TechnoKind::Infantry,
                        TechnoClass::Vehicle => TechnoKind::Vehicle,
                        TechnoClass::Aircraft => TechnoKind::Aircraft,
                        TechnoClass::Building => TechnoKind::Building,
                    };
                    let Some(factory_index) = self.find_idle_factory(&house, kind)
                    else {
                        let has_busy = self.find_factory(&house, kind).is_some();
                        if has_busy {
                            self.reject(command_index, CommandRejectReason::QueueFull);
                        }
                        else {
                            self.reject(command_index, CommandRejectReason::MissingPrerequisite);
                        }
                        continue;
                    };
                    let cost = tt.cost;
                    if self.players[player_index].funds < cost {
                        self.reject(command_index, CommandRejectReason::InsufficientFunds);
                        continue;
                    }
                    self.players[player_index].funds -= cost;
                    self.players[player_index].funds_spent = self.players[player_index].funds_spent.saturating_add(cost);
                    let factory_id = self.entities[factory_index].id;
                    let queued = Arc::<str>::from(type_id.to_ascii_uppercase());
                    let ticks = produce_ticks_for(tt);
                    let _ = self.with_production_mut(factory_id, |queue| {
                        queue.item = Some((queued, ticks));
                    });
                    self.mark_entity_dirty(factory_id);
                }
                GameCommand::CancelProduce { player, ref type_id } => {
                    if player != scheduled.player {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(player_index) = self.players.iter().position(|p| p.id == player)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let house = self.players[player_index].house.clone();
                    let needle = type_id.to_ascii_uppercase();
                    let Some(factory_id) = self.entities.iter().find_map(|e| {
                        let id = e.id;
                        if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                            return None;
                        }
                        if !self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house.as_ref()).unwrap_or(false) {
                            return None;
                        }
                        let matches = self
                            .ecs_get::<ProductionQueue>(id)
                            .and_then(|q| q.item.as_ref())
                            .map(|(queued, _)| queued.as_ref() == needle.as_str())
                            .unwrap_or(false);
                        matches.then_some(id)
                    })
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    let refund = self
                        .definitions
                        .techno
                        .get(needle.as_str())
                        .map(|tt| tt.cost)
                        .unwrap_or(0);
                    let _ = self.with_production_mut(factory_id, |queue| {
                        queue.item = None;
                    });
                    if refund > 0 {
                        self.players[player_index].funds = self.players[player_index].funds.saturating_add(refund);
                        self.players[player_index].funds_spent =
                            self.players[player_index].funds_spent.saturating_sub(refund);
                    }
                    self.mark_entity_dirty(factory_id);
                    self.push_eva_cue(house.as_ref(), "EVA_Canceled");
                }
                GameCommand::SetRallyPoint { factory, x, y } => {
                    let Some(factory_index) = self.entity_index(factory)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let id = self.entities[factory_index].id;
                    if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, factory_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(type_id) = self.ecs_get::<Identity>(id).map(|i| i.type_id.clone())
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    if !is_production_factory(&self.definitions, &type_id) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !self.pass_grid.in_bounds(x, y) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    let _ = self.with_production_mut(id, |queue| {
                        queue.rally_x = Some(x);
                        queue.rally_y = Some(y);
                    });
                    self.mark_entity_dirty(id);
                }
                GameCommand::Infiltrate { agent, building } => {
                    let Some(agent_index) = self.entity_index(agent)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let Some(building_index) = self.entity_index(building)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    if agent == building {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let agent_id = self.entities[agent_index].id;
                    let building_id = self.entities[building_index].id;
                    if self.ecs_get::<Health>(agent_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, agent_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(agent_type) = self.ecs_get::<Identity>(agent_id).map(|i| i.type_id.clone())
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    if !is_agent(&self.definitions, agent_type.as_ref()) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !self.ecs_get::<Identity>(agent_id).map(|i| is_mobile(i.kind)).unwrap_or(false) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    if self.ecs_get::<Health>(building_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !self
                        .ecs_get::<Identity>(building_id)
                        .map(|i| i.kind == MapEntityKind::Structure)
                        .unwrap_or(false)
                    {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let agent_house = self.ecs_get::<Owner>(agent_id).map(|o| o.house.clone());
                    let building_house = self.ecs_get::<Owner>(building_id).map(|o| o.house.clone());
                    match (agent_house, building_house) {
                        (Some(a), Some(b)) if a != b => {}
                        _ => {
                            self.reject(command_index, CommandRejectReason::InvalidTarget);
                            continue;
                        }
                    }
                    let Some(building_xf) = self.ecs_get::<Transform>(building_id).copied()
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    let agent_xf = self.ecs_get::<Transform>(agent_id).copied().unwrap_or(building_xf);
                    let foundation = self
                        .definitions
                        .structures
                        .get(
                            self.ecs_get::<Identity>(building_id)
                                .map(|i| i.type_id.as_ref())
                                .unwrap_or(""),
                        )
                        .map(|s| s.foundation.clone())
                        .unwrap_or_default();
                    let (dest_x, dest_y) = nearest_adjacent_to_footprint(
                        agent_xf.x,
                        agent_xf.y,
                        building_xf.x,
                        building_xf.y,
                        foundation.width,
                        foundation.height,
                    );
                    let _ = self.with_identity_mut(agent_id, |identity| {
                        identity.mission.clear();
                    });
                    let _ = self.with_attack_mut(agent_id, |attack| {
                        attack.target = None;
                        attack.infiltrate_target = Some(building);
                        attack.capture_target = None;
                    });
                    let _ = self.with_movement_mut(agent_id, |movement| {
                        movement.destination_x = Some(dest_x);
                        movement.destination_y = Some(dest_y);
                        movement.waypoints.clear();
                        movement.path.clear();
                        movement.move_accum = 0;
                    });
                    self.repath_entity_at(agent_index);
                    self.mark_entity_dirty(agent_id);
                }
                GameCommand::CaptureBuilding { engineer, building } => {
                    let Some(engineer_index) = self.entity_index(engineer)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let Some(building_index) = self.entity_index(building)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    if engineer == building {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let engineer_id = self.entities[engineer_index].id;
                    let building_id = self.entities[building_index].id;
                    if self.ecs_get::<Health>(engineer_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, engineer_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(engineer_type) = self.ecs_get::<Identity>(engineer_id).map(|i| i.type_id.clone())
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    if !is_engineer(&self.definitions, engineer_type.as_ref()) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !self.ecs_get::<Identity>(engineer_id).map(|i| is_mobile(i.kind)).unwrap_or(false) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    if self.ecs_get::<Health>(building_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !self
                        .ecs_get::<Identity>(building_id)
                        .map(|i| i.kind == MapEntityKind::Structure)
                        .unwrap_or(false)
                    {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let Some(building_type) = self.ecs_get::<Identity>(building_id).map(|i| i.type_id.clone())
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    if !is_capturable(&self.definitions, building_type.as_ref()) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let engineer_house = self.ecs_get::<Owner>(engineer_id).map(|o| o.house.clone());
                    let building_house = self.ecs_get::<Owner>(building_id).map(|o| o.house.clone());
                    match (engineer_house, building_house) {
                        (Some(a), Some(b)) if a != b => {}
                        _ => {
                            self.reject(command_index, CommandRejectReason::InvalidTarget);
                            continue;
                        }
                    }
                    let Some(building_xf) = self.ecs_get::<Transform>(building_id).copied()
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    let engineer_xf = self.ecs_get::<Transform>(engineer_id).copied().unwrap_or(building_xf);
                    let foundation = self
                        .definitions
                        .structures
                        .get(building_type.as_ref())
                        .map(|s| s.foundation.clone())
                        .unwrap_or_default();
                    let (dest_x, dest_y) = nearest_adjacent_to_footprint(
                        engineer_xf.x,
                        engineer_xf.y,
                        building_xf.x,
                        building_xf.y,
                        foundation.width,
                        foundation.height,
                    );
                    let _ = self.with_identity_mut(engineer_id, |identity| {
                        identity.mission.clear();
                    });
                    let _ = self.with_attack_mut(engineer_id, |attack| {
                        attack.target = None;
                        attack.infiltrate_target = None;
                        attack.capture_target = Some(building);
                    });
                    let _ = self.with_movement_mut(engineer_id, |movement| {
                        movement.destination_x = Some(dest_x);
                        movement.destination_y = Some(dest_y);
                        movement.waypoints.clear();
                        movement.path.clear();
                        movement.move_accum = 0;
                    });
                    self.repath_entity_at(engineer_index);
                    self.mark_entity_dirty(engineer_id);
                }
                GameCommand::Guard { entity } => {
                    let Some(entity_index) = self.entity_index(entity)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let id = self.entities[entity_index].id;
                    if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, entity_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    if !self.ecs_get::<Identity>(id).map(|i| is_mobile(i.kind)).unwrap_or(false) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    let _ = self.clear_ecs_movement(id);
                    let _ = self.with_attack_mut(id, |attack| {
                        attack.target = None;
                        attack.infiltrate_target = None;
                        attack.capture_target = None;
                    });
                    let _ = self.with_identity_mut(id, |identity| {
                        identity.mission = "Guard".into();
                    });
                    self.mark_entity_dirty(id);
                }
                GameCommand::SellBuilding { player, building } => {
                    if player != scheduled.player {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(player_index) = self.players.iter().position(|p| p.id == player)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let Some(building_index) = self.entity_index(building)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let building_id = self.entities[building_index].id;
                    if self.ecs_get::<Health>(building_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, building_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(identity) = self.ecs_get::<Identity>(building_id).cloned()
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    if identity.kind != MapEntityKind::Structure {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let Some(xf) = self.ecs_get::<Transform>(building_id).copied()
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    let house = self.players[player_index].house.clone();
                    let cost = self
                        .definitions
                        .techno
                        .get(identity.type_id.as_ref())
                        .map(|tt| tt.cost)
                        .unwrap_or(0);
                    // 原版侧栏出售约退半价。
                    let refund = (cost / 2).max(0);
                    let foundation = self
                        .definitions
                        .structures
                        .get(identity.type_id.as_ref())
                        .map(|s| s.foundation.clone())
                        .unwrap_or_default();
                    let _ = self.with_health_mut(building_id, |health| {
                        health.current = 0;
                        health.dead = true;
                    });
                    let _ = self.with_production_mut(building_id, |queue| {
                        queue.item = None;
                    });
                    self.unseal_structure_footprint(xf.x, xf.y, foundation.width, foundation.height);
                    self.revoke_structure_power(house.as_ref(), identity.type_id.as_ref());
                    if refund > 0 {
                        self.players[player_index].funds =
                            self.players[player_index].funds.saturating_add(refund);
                        self.players[player_index].funds_spent =
                            self.players[player_index].funds_spent.saturating_sub(refund);
                    }
                    self.mark_entity_dirty(building_id);
                    self.repath_mobiles();
                }
                GameCommand::RepairBuilding { player, building } => {
                    if player != scheduled.player {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(building_index) = self.entity_index(building)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let building_id = self.entities[building_index].id;
                    if self.ecs_get::<Health>(building_id).map(|h| h.dead).unwrap_or(true) {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, building_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(identity) = self.ecs_get::<Identity>(building_id)
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    };
                    if identity.kind != MapEntityKind::Structure {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    let Some(handle) = self.ecs.resolve(building_id)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    // 原版扳手：再点同一建筑则取消修理，否则挂上持续修理。
                    if self.ecs.world().get::<crate::state::components::Repairing>(handle).is_some() {
                        let _ = self.ecs.world_mut().remove::<crate::state::components::Repairing>(handle);
                    } else {
                        let _ = self
                            .ecs
                            .world_mut()
                            .insert(handle, crate::state::components::Repairing);
                    }
                    self.mark_entity_dirty(building_id);
                }
            }
        }
    }

    /// 信封玩家是否拥有该实体（按 house 名对齐）。
    fn player_owns_entity(&self, player: ra_types::PlayerId, entity_index: usize) -> bool {
        let Some(p) = self.players.iter().find(|p| p.id == player)
        else {
            return false;
        };
        let id = self.entities[entity_index].id;
        self.ecs_get::<crate::state::components::Owner>(id)
            .map(|o| o.house.as_ref() == p.house.as_ref())
            .unwrap_or(false)
    }

    pub(crate) fn reject(&mut self, command_index: usize, reason: crate::game::CommandRejectReason) {
        self.last_rejects.push(crate::game::CommandReject { command_index, reason });
    }
}
