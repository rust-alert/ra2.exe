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

impl crate::state::MatchState {
    pub(crate) fn apply_commands(&mut self, cmds: &[ScheduledCommand]) {
        use ra_assets::TechnoKind;
        use ra_map::MapEntityKind;
        use ra_types::TechnoClass;

        use crate::{
            game::CommandRejectReason,
            gameplay::{building_power, deploy_into_type, full_verses, is_construction_yard, is_production_factory, requires_power_plant},
            spatial::{is_mobile, repath_at},
            state::{PRODUCE_TICKS, WorldEntity},
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
                    if self.entities[entity_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, entity_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    if !is_mobile(self.entities[entity_index].kind) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    let e = &mut self.entities[entity_index];
                    e.attack_target = None;
                    e.target_x = Some(x);
                    e.target_y = Some(y);
                    e.path.clear();
                    e.move_accum = 0;
                    repath_at(&mut self.entities, entity_index, &self.pass_grid);
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
                    if self.entities[attacker_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, attacker_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    if self.entities[target_index].dead {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !is_mobile(self.entities[attacker_index].kind) {
                        self.reject(command_index, CommandRejectReason::NotMobile);
                        continue;
                    }
                    let (tx, ty) = (self.entities[target_index].x, self.entities[target_index].y);
                    let a = &mut self.entities[attacker_index];
                    a.attack_target = Some(target);
                    a.target_x = Some(tx);
                    a.target_y = Some(ty);
                    a.path.clear();
                    a.move_accum = 0;
                    repath_at(&mut self.entities, attacker_index, &self.pass_grid);
                }
                GameCommand::Deploy { entity } => {
                    let Some(entity_index) = self.entity_index(entity)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    if self.entities[entity_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, entity_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    let Some(building_type) = deploy_into_type(&self.definitions, &self.entities[entity_index].type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::CannotDeploy);
                        continue;
                    };
                    let armor = self.definitions.techno.get(building_type).map(|t| t.armor.clone()).unwrap_or_else(|| "none".into());
                    let e = &mut self.entities[entity_index];
                    e.kind = MapEntityKind::Structure;
                    e.type_id = Arc::<str>::from(building_type);
                    e.speed = 0;
                    e.target_x = None;
                    e.target_y = None;
                    e.path.clear();
                    e.move_accum = 0;
                    e.attack_target = None;
                    e.attack_range = 0;
                    e.attack_damage = 0;
                    e.attack_cooldown = 0;
                    e.attack_verses = full_verses();
                    e.armor = armor;
                    e.techno_kind = Some(TechnoKind::Building);
                    e.hva_frame = 0;
                    let dirty_id = e.id;
                    self.mark_entity_dirty(dirty_id);
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
                    if requires_power_plant(&self.definitions, type_id) && !self.house_has_living_power(&house) {
                        self.reject(command_index, CommandRejectReason::InsufficientPower);
                        continue;
                    }
                    if !self.can_place_structure(x, y) {
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
                    self.pass_grid.set_passable(x, y, false);
                    self.entities.push(WorldEntity {
                        id,
                        kind: MapEntityKind::Structure,
                        owner: house,
                        type_id: Arc::<str>::from(type_id.to_ascii_uppercase()),
                        x,
                        y,
                        facing: 0,
                        turret_facing: 0,
                        sub_cell: 0,
                        health: max_health,
                        max_health,
                        speed: 0,
                        armor,
                        attack_range: 0,
                        attack_damage: 0,
                        attack_cooldown_max: 0,
                        attack_verses: full_verses(),
                        techno_kind: Some(TechnoKind::Building),
                        target_x: None,
                        target_y: None,
                        path: Vec::new(),
                        move_accum: 0,
                        hva_frame: 0,
                        attack_target: None,
                        attack_cooldown: 0,
                        ore_trip_accum: 0,
                        produce_queue: None,
                        rally_x: None,
                        rally_y: None,
                        hit_flash: 0,
                        dead: false,
                    });
                    self.bind_ecs_at(self.entities.len() - 1);
                    self.mark_entity_dirty(id);
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
                    let Some(tt) = self.definitions.techno.get(type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    };
                    if !matches!(tt.class, TechnoClass::Infantry | TechnoClass::Vehicle) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
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
                    self.entities[factory_index].produce_queue = Some((Arc::<str>::from(type_id.to_ascii_uppercase()), PRODUCE_TICKS));
                    self.mark_entity_dirty(self.entities[factory_index].id);
                }
                GameCommand::SetRallyPoint { factory, x, y } => {
                    let Some(factory_index) = self.entity_index(factory)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    if self.entities[factory_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !self.player_owns_entity(scheduled.player, factory_index) {
                        self.reject(command_index, CommandRejectReason::WrongOwner);
                        continue;
                    }
                    if !is_production_factory(&self.definitions, &self.entities[factory_index].type_id) {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if !self.pass_grid.in_bounds(x, y) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    let e = &mut self.entities[factory_index];
                    e.rally_x = Some(x);
                    e.rally_y = Some(y);
                    let id = e.id;
                    self.mark_entity_dirty(id);
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
        self.entities[entity_index].owner.as_ref() == p.house.as_ref()
    }

    pub(crate) fn reject(&mut self, command_index: usize, reason: crate::game::CommandRejectReason) {
        self.last_rejects.push(crate::game::CommandReject { command_index, reason });
    }
}
