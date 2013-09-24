//! 玩家/AI 注入的确定性命令（按 tick 排序消费）。

use ra_types::PlayerId;

/// 单条命令。后续扩展生产等。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    /// 将实体移动到目标格（会重算路径）。
    MoveTo {
        /// 实体在世界实体列表中的下标。
        entity_index: usize,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 指定攻击目标（进入射程后造成伤害）。
    Attack {
        /// 攻击方实体下标。
        attacker_index: usize,
        /// 被攻击方实体下标。
        target_index: usize,
    },
    /// 部署可展开实体（如 MCV → 建造场）。
    Deploy {
        /// 实体在世界实体列表中的下标。
        entity_index: usize,
    },
    /// 在目标格放置建筑（扣费、校验前置与占地）。
    PlaceBuilding {
        /// 出资并拥有该建筑的玩家。
        player: PlayerId,
        /// 规则类型 ID（如 `GAPOWR`）。
        type_id: String,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 在空闲工厂排队生产单位（立即扣费）。
    Produce {
        /// 出资并拥有产出单位的玩家。
        player: PlayerId,
        /// 规则类型 ID（如 `E1` / `MTNK`）。
        type_id: String,
    },
    /// 为工厂设置生产集结点。
    SetRallyPoint {
        /// 工厂实体下标。
        factory_index: usize,
        /// 集结格 X。
        x: u16,
        /// 集结格 Y。
        y: u16,
    },
}

/// 一个仿真 tick 的完整输入帧。
///
/// 联机锁步要求：**每个 tick 都必须有一帧**；本 tick 无操作时 `commands` 为空。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InputFrame {
    /// 本帧对应的逻辑 tick。
    pub tick: u64,
    /// 本 tick 消费的命令列表（可为空）。
    pub commands: Vec<GameCommand>,
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

/// 编码单条命令为网络 `payload` 字节。
pub fn encode_command(cmd: &GameCommand) -> Vec<u8> {
    let mut b = Vec::new();
    match *cmd {
        GameCommand::MoveTo { entity_index, x, y } => {
            b.push(1);
            b.extend_from_slice(&(entity_index as u32).to_be_bytes());
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::Attack { attacker_index, target_index } => {
            b.push(2);
            b.extend_from_slice(&(attacker_index as u32).to_be_bytes());
            b.extend_from_slice(&(target_index as u32).to_be_bytes());
        }
        GameCommand::Deploy { entity_index } => {
            b.push(3);
            b.extend_from_slice(&(entity_index as u32).to_be_bytes());
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
        GameCommand::SetRallyPoint { factory_index, x, y } => {
            b.push(6);
            b.extend_from_slice(&(factory_index as u32).to_be_bytes());
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
            if bytes.len() < 1 + 4 + 2 + 2 {
                return None;
            }
            let entity_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            let x = u16::from_be_bytes(bytes[5..7].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[7..9].try_into().ok()?);
            Some(GameCommand::MoveTo { entity_index, x, y })
        }
        2 => {
            if bytes.len() < 1 + 4 + 4 {
                return None;
            }
            let attacker_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            let target_index = u32::from_be_bytes(bytes[5..9].try_into().ok()?) as usize;
            Some(GameCommand::Attack { attacker_index, target_index })
        }
        3 => {
            if bytes.len() < 1 + 4 {
                return None;
            }
            let entity_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            Some(GameCommand::Deploy { entity_index })
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
            if bytes.len() < 1 + 4 + 2 + 2 {
                return None;
            }
            let factory_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            let x = u16::from_be_bytes(bytes[5..7].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[7..9].try_into().ok()?);
            Some(GameCommand::SetRallyPoint { factory_index, x, y })
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

/// 解码命令列表。
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

impl crate::state::MatchState {
    pub(crate) fn apply_commands(&mut self, cmds: &[GameCommand]) {
        use ra_assets::TechnoKind;
        use ra_map::MapEntityKind;

        use crate::{
            gameplay::{
                building_power_delta, deploy_into_type, full_verses, is_construction_yard, is_production_factory,
                requires_power_plant,
            },
            game::CommandRejectReason,
            spatial::{is_mobile, repath_at},
            state::{PRODUCE_TICKS, WorldEntity},
        };

        for (command_index, cmd) in cmds.iter().enumerate() {
            match *cmd {
                GameCommand::MoveTo { entity_index, x, y } => {
                    if entity_index >= self.entities.len() {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    }
                    if self.entities[entity_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
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
                GameCommand::Attack { attacker_index, target_index } => {
                    if attacker_index >= self.entities.len() || target_index >= self.entities.len() {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    }
                    if attacker_index == target_index {
                        self.reject(command_index, CommandRejectReason::InvalidTarget);
                        continue;
                    }
                    if self.entities[attacker_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
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
                    a.attack_target = Some(target_index);
                    a.target_x = Some(tx);
                    a.target_y = Some(ty);
                    a.path.clear();
                    a.move_accum = 0;
                    repath_at(&mut self.entities, attacker_index, &self.pass_grid);
                }
                GameCommand::Deploy { entity_index } => {
                    if entity_index >= self.entities.len() {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    }
                    if self.entities[entity_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    let Some(building_type) = deploy_into_type(&self.entities[entity_index].type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::CannotDeploy);
                        continue;
                    };
                    let armor = self.techno_types.get(building_type).map(|t| t.armor.clone()).unwrap_or_else(|| "none".into());
                    let e = &mut self.entities[entity_index];
                    e.kind = MapEntityKind::Structure;
                    e.type_id = building_type.to_string();
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
                    e.hva_frame = 0;
                }
                GameCommand::PlaceBuilding { player, ref type_id, x, y } => {
                    let Some(player_index) = self.players.iter().position(|p| p.id == player)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let house = self.players[player_index].house.clone();
                    let Some(tt) = self.techno_types.get(type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    };
                    if tt.kind != TechnoKind::Building {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    if is_construction_yard(type_id) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    if !self.house_has_living_yard(&house) {
                        self.reject(command_index, CommandRejectReason::MissingPrerequisite);
                        continue;
                    }
                    if requires_power_plant(type_id) && !self.house_has_living_power(&house) {
                        self.reject(command_index, CommandRejectReason::MissingPrerequisite);
                        continue;
                    }
                    if !self.can_place_structure(x, y) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    let cost = tt.cost as i32;
                    if self.players[player_index].funds < cost {
                        self.reject(command_index, CommandRejectReason::InsufficientFunds);
                        continue;
                    }
                    let power = building_power_delta(type_id);
                    let max_health = tt.strength.max(1);
                    let armor = tt.armor.clone();
                    let id = self.alloc_entity_id();
                    self.players[player_index].funds -= cost;
                    self.players[player_index].funds_spent = self.players[player_index].funds_spent.saturating_add(cost);
                    if power >= 0 {
                        self.players[player_index].power_output = self.players[player_index].power_output.saturating_add(power);
                    }
                    else {
                        self.players[player_index].power_drain = self.players[player_index].power_drain.saturating_add(-power);
                    }
                    self.pass_grid.set_passable(x, y, false);
                    self.entities.push(WorldEntity {
                        id,
                        kind: MapEntityKind::Structure,
                        owner: house,
                        type_id: type_id.to_ascii_uppercase(),
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
                }
                GameCommand::Produce { player, ref type_id } => {
                    let Some(player_index) = self.players.iter().position(|p| p.id == player)
                    else {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    };
                    let house = self.players[player_index].house.clone();
                    let Some(tt) = self.techno_types.get(type_id)
                    else {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    };
                    if !matches!(tt.kind, TechnoKind::Infantry | TechnoKind::Vehicle) {
                        self.reject(command_index, CommandRejectReason::InvalidPlacement);
                        continue;
                    }
                    let Some(factory_index) = self.find_idle_factory(&house, tt.kind)
                    else {
                        let has_busy = self.find_factory(&house, tt.kind).is_some();
                        if has_busy {
                            self.reject(command_index, CommandRejectReason::QueueFull);
                        }
                        else {
                            self.reject(command_index, CommandRejectReason::MissingPrerequisite);
                        }
                        continue;
                    };
                    let cost = tt.cost as i32;
                    if self.players[player_index].funds < cost {
                        self.reject(command_index, CommandRejectReason::InsufficientFunds);
                        continue;
                    }
                    self.players[player_index].funds -= cost;
                    self.players[player_index].funds_spent = self.players[player_index].funds_spent.saturating_add(cost);
                    self.entities[factory_index].produce_queue = Some((type_id.to_ascii_uppercase(), PRODUCE_TICKS));
                }
                GameCommand::SetRallyPoint { factory_index, x, y } => {
                    if factory_index >= self.entities.len() {
                        self.reject(command_index, CommandRejectReason::EntityNotFound);
                        continue;
                    }
                    if self.entities[factory_index].dead {
                        self.reject(command_index, CommandRejectReason::EntityDead);
                        continue;
                    }
                    if !is_production_factory(&self.entities[factory_index].type_id) {
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
                }
            }
        }
    }

    pub(crate) fn reject(&mut self, command_index: usize, reason: crate::game::CommandRejectReason) {
        self.last_rejects.push(crate::game::CommandReject { command_index, reason });
    }
}
