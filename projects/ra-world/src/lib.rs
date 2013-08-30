//! 确定性世界推进。不依赖渲染器与文件系统。

#![deny(missing_docs)]

mod command;
mod player;
mod reject;

use ra_adaptor::RulesDb;
use ra_assets::{TechnoKind, TechnoTypeRegistry, WarheadRegistry, armor_index};
use ra_map::{MapEntityKind, MapInfo, PassGrid};
use ra_types::{EntityId, GameEdition, PlayerId};

pub use command::{GameCommand, InputFrame, decode_command, decode_commands, encode_command, encode_commands};
pub use player::PlayerState;
pub use reject::{CommandReject, CommandRejectReason};

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

/// 世界中的一个已放置实体（由地图播种，后续仿真就地改）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntity {
    /// 稳定实体 ID（不随列表紧凑化改变）。
    pub id: EntityId,
    /// 地图实体种类（单位、建筑、步兵等）。
    pub kind: MapEntityKind,
    /// 所属方名称（地图放置段字符串）。
    pub owner: String,
    /// 规则类型 ID（如 `MTNK`）。
    pub type_id: String,
    /// 当前格 X。
    pub x: u16,
    /// 当前格 Y。
    pub y: u16,
    /// 车身朝向（0..=255 环）。
    pub facing: u8,
    /// 炮塔朝向（无炮塔时与 `facing` 同步）。
    pub turret_facing: u8,
    /// 子格偏移（步兵等）。
    pub sub_cell: u8,
    /// 当前生命（由放置段比例 × Strength）。
    pub health: u32,
    /// 最大生命（rules `Strength`）。
    pub max_health: u32,
    /// 移动速度（每 tick 累加到 `move_accum`）。
    pub speed: u32,
    /// 护甲名（rules `Armor`）；未知按 `none`。
    pub armor: String,
    /// 攻击射程（曼哈顿格）；优先武器 `Range`，否则 `Sight` / 预览常量。
    pub attack_range: u32,
    /// 单次基础伤害；优先武器 `Damage`，否则 `Strength/4` / 预览常量。
    pub attack_damage: u32,
    /// 开火冷却上限（tick）；优先武器 `ROF`，否则类型节 / 预览常量。
    pub attack_cooldown_max: u32,
    /// 弹头对各护甲的伤害百分比（来自 `Warhead`/`Verses`）；缺省全 100。
    pub attack_verses: [u32; 11],
    /// 对应 techno 种类；无规则绑定时为 `None`。
    pub techno_kind: Option<TechnoKind>,
    /// 简易移动目标格 X；无航点时为 `None`。
    pub target_x: Option<u16>,
    /// 简易移动目标格 Y；无航点时为 `None`。
    pub target_y: Option<u16>,
    /// 剩余路径（下一格在 `[0]`）。
    pub path: Vec<(u16, u16)>,
    /// 累积移动点（每 tick += Speed）。
    pub move_accum: u32,
    /// HVA 动画帧（移动时递增；光栅化时对 `hva.frames` 取模）。
    pub hva_frame: u16,
    /// 攻击目标实体下标。
    pub attack_target: Option<usize>,
    /// 开火冷却剩余 tick。
    pub attack_cooldown: u32,
    /// 矿场采矿行程累计 tick；非矿场保持 0。
    pub ore_trip_accum: u32,
    /// 生产队列：（类型 ID，剩余 tick）；空闲为 `None`。
    pub produce_queue: Option<(String, u32)>,
    /// 生产集结格 X。
    pub rally_x: Option<u16>,
    /// 生产集结格 Y。
    pub rally_y: Option<u16>,
    /// 受击闪白剩余 tick；大于 0 时呈现层可显示 `TakeDamage`。
    pub hit_flash: u32,
    /// 生命归零后为真；不再移动/占格。
    pub dead: bool,
}

/// 确定性仿真世界：实体、通行格与按 tick 消费的命令。
#[derive(Debug, Clone)]
pub struct World {
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
    /// 规则 techno 表（造价、生命等查询）。
    techno_types: TechnoTypeRegistry,
    /// 弹头 `Verses` 表（攻击结算）。
    warheads: WarheadRegistry,
    /// 下一枚可分配的稳定实体 ID（从 1 起）。
    next_entity_id: u64,
    /// 待本 tick 消费的命令（先进先出）。
    pending_commands: Vec<GameCommand>,
    /// 上一 tick 实际消费的输入帧（含空帧）。
    last_input_frame: InputFrame,
    /// 上一 tick 产生的命令拒绝记录。
    last_rejects: Vec<CommandReject>,
    state_hash: u64,
}

impl World {
    /// 由规则与地图播种新世界，并为移动单位预计算路径。
    pub fn new(edition: GameEdition, rules: &RulesDb, map: MapInfo) -> Self {
        let pass_grid = PassGrid::from_map(&map);
        let mut next_entity_id = 1u64;
        let mut house_order: Vec<String> = Vec::new();
        let entities: Vec<WorldEntity> = map
            .entities
            .iter()
            .map(|e| {
                if !house_order.iter().any(|h| h == &e.owner) {
                    house_order.push(e.owner.clone());
                }
                let tt = rules.techno_types.get(&e.type_id);
                let max_health = tt.map(|t| t.strength).unwrap_or(1).max(1);
                let health = (u64::from(max_health) * u64::from(e.health) / 256) as u32;
                let speed = tt.map(|t| t.speed).unwrap_or(0);
                let attack_range = tt
                    .map(|t| if t.range > 0 { t.range } else { t.sight.max(1) })
                    .unwrap_or(DEFAULT_ATTACK_RANGE);
                let attack_damage = tt
                    .map(|t| {
                        if t.damage > 0 {
                            t.damage
                        } else {
                            (t.strength / 4).max(1)
                        }
                    })
                    .unwrap_or(DEFAULT_ATTACK_DAMAGE);
                let attack_cooldown_max =
                    tt.map(|t| if t.rof > 0 { t.rof } else { ATTACK_COOLDOWN_TICKS }).unwrap_or(ATTACK_COOLDOWN_TICKS);
                let armor = tt.map(|t| t.armor.clone()).unwrap_or_else(|| "none".into());
                let attack_verses = tt
                    .map(|t| verses_for(&rules.warheads, &t.warhead))
                    .unwrap_or_else(full_verses);
                let id = EntityId(next_entity_id);
                next_entity_id = next_entity_id.saturating_add(1);
                WorldEntity {
                    id,
                    kind: e.kind,
                    owner: e.owner.clone(),
                    type_id: e.type_id.clone(),
                    x: e.x,
                    y: e.y,
                    facing: e.facing,
                    turret_facing: e.facing,
                    sub_cell: e.sub_cell,
                    health,
                    max_health,
                    speed,
                    armor,
                    attack_range,
                    attack_damage,
                    attack_cooldown_max,
                    attack_verses,
                    techno_kind: tt.map(|t| t.kind),
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
                }
            })
            .collect();
        let players: Vec<PlayerState> = house_order
            .into_iter()
            .enumerate()
            .map(|(i, house)| PlayerState::new(PlayerId(i as u8), house))
            .collect();
        let mut world = Self {
            edition,
            tick: 0,
            map,
            pass_grid,
            entities,
            players,
            local_player: PlayerId(0),
            techno_types: rules.techno_types.clone(),
            warheads: rules.warheads.clone(),
            next_entity_id,
            pending_commands: Vec::new(),
            last_input_frame: InputFrame::empty(0),
            last_rejects: Vec::new(),
            state_hash: 0,
        };
        for i in 0..world.entities.len() {
            repath_at(&mut world.entities, i, &world.pass_grid);
        }
        world.rehash();
        world
    }

    /// 入队命令；在下一次 `advance_tick` 开头按序应用。
    pub fn push_command(&mut self, cmd: GameCommand) {
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

    /// 按 house 名称设置资金（启动与测试播种用）。
    pub fn set_house_funds(&mut self, house: &str, funds: i32) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house == house) {
            player.funds = funds;
            self.rehash();
            true
        } else {
            false
        }
    }

    /// 按 house 名称读取资金。
    pub fn house_funds(&self, house: &str) -> Option<i32> {
        self.players.iter().find(|p| p.house == house).map(|p| p.funds)
    }

    /// 查询规则造价；未知类型为 `None`。
    pub fn techno_cost(&self, type_id: &str) -> Option<u32> {
        self.techno_types.get(type_id).map(|t| t.cost)
    }

    /// 推进一个逻辑 tick：消费命令、移动、战斗与炮塔转向。
    pub fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let commands = std::mem::take(&mut self.pending_commands);
        self.last_input_frame = InputFrame { tick: self.tick, commands: commands.clone() };
        self.last_rejects.clear();
        self.apply_commands(&commands);
        self.advance_movement();
        self.tick_hit_flash();
        self.resolve_combat();
        self.advance_turrets();
        self.advance_refinery_income();
        self.advance_production();
        self.rehash();
    }

    fn advance_movement(&mut self) {
        let n = self.entities.len();
        for i in 0..n {
            if self.entities[i].dead || !is_mobile(self.entities[i].kind) || self.entities[i].speed == 0 {
                continue;
            }
            // 攻击中且已在射程内：停步开火，不继续挤占目标格。
            if let Some(ti) = self.entities[i].attack_target {
                if ti < n
                    && !self.entities[ti].dead
                    && manhattan(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y)
                        <= self.entities[i].attack_range
                {
                    self.entities[i].path.clear();
                    continue;
                }
            }
            let (Some(tx), Some(ty)) = (self.entities[i].target_x, self.entities[i].target_y)
            else {
                continue;
            };
            if self.entities[i].x == tx && self.entities[i].y == ty {
                self.entities[i].path.clear();
                continue;
            }
            self.entities[i].move_accum = self.entities[i].move_accum.saturating_add(self.entities[i].speed);
            while self.entities[i].move_accum >= CELL_MOVE_COST {
                self.entities[i].move_accum -= CELL_MOVE_COST;
                if self.entities[i].path.is_empty() {
                    repath_at(&mut self.entities, i, &self.pass_grid);
                    if self.entities[i].path.is_empty() {
                        break;
                    }
                }
                let Some((nx, ny)) = self.entities[i].path.first().copied()
                else {
                    break;
                };
                if cell_occupied_by_other(&self.entities, i, nx, ny) {
                    self.entities[i].path.clear();
                    repath_at(&mut self.entities, i, &self.pass_grid);
                    let Some((nx2, ny2)) = self.entities[i].path.first().copied()
                    else {
                        break;
                    };
                    if cell_occupied_by_other(&self.entities, i, nx2, ny2) {
                        break;
                    }
                }
                if !step_along_path(&mut self.entities[i]) {
                    break;
                }
                self.entities[i].hva_frame = self.entities[i].hva_frame.wrapping_add(1);
            }
        }
    }

    fn resolve_combat(&mut self) {
        let n = self.entities.len();
        let mut damage_events: Vec<(usize, u32)> = Vec::new();
        for i in 0..n {
            if self.entities[i].dead || !is_mobile(self.entities[i].kind) {
                continue;
            }
            let Some(ti) = self.entities[i].attack_target
            else {
                continue;
            };
            if ti >= n || self.entities[ti].dead || ti == i {
                self.entities[i].attack_target = None;
                continue;
            }
            // 追击：把移动目标钉在敌人当前格。
            self.entities[i].target_x = Some(self.entities[ti].x);
            self.entities[i].target_y = Some(self.entities[ti].y);

            if self.entities[i].attack_cooldown > 0 {
                self.entities[i].attack_cooldown -= 1;
                continue;
            }
            let dist = manhattan(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y);
            if dist <= self.entities[i].attack_range {
                let base = self.entities[i].attack_damage;
                let verses = self.entities[i].attack_verses;
                let armor = self.entities[ti].armor.as_str();
                let dmg = scale_damage(base, &verses, armor);
                damage_events.push((ti, dmg));
                self.entities[i].attack_cooldown = self.entities[i].attack_cooldown_max;
            }
        }
        for (ti, dmg) in damage_events {
            self.apply_damage(ti, dmg);
        }
    }

    fn tick_hit_flash(&mut self) {
        for e in &mut self.entities {
            if e.hit_flash > 0 {
                e.hit_flash -= 1;
            }
        }
    }

    fn apply_damage(&mut self, index: usize, amount: u32) {
        if index >= self.entities.len() || self.entities[index].dead || amount == 0 {
            return;
        }
        let house = self.entities[index].owner.clone();
        let kind = self.entities[index].kind;
        let type_id = self.entities[index].type_id.clone();
        let (x, y) = (self.entities[index].x, self.entities[index].y);
        {
            let e = &mut self.entities[index];
            e.health = e.health.saturating_sub(amount);
            e.hit_flash = HIT_FLASH_TICKS;
            if e.health > 0 {
                return;
            }
            e.dead = true;
            e.speed = 0;
            e.path.clear();
            e.target_x = None;
            e.target_y = None;
            e.attack_target = None;
            e.move_accum = 0;
        }
        let dead_i = index;
        for o in self.entities.iter_mut() {
            if o.attack_target == Some(dead_i) {
                o.attack_target = None;
            }
        }
        if kind == MapEntityKind::Structure {
            self.pass_grid.set_passable(x, y, true);
            self.revoke_structure_power(&house, &type_id);
        }
    }

    fn revoke_structure_power(&mut self, house: &str, type_id: &str) {
        let Some(player) = self.players.iter_mut().find(|p| p.house == house)
        else {
            return;
        };
        let power = building_power_delta(type_id);
        if power >= 0 {
            player.power_output = player.power_output.saturating_sub(power);
        }
        else {
            player.power_drain = player.power_drain.saturating_sub(-power);
        }
    }

    fn advance_turrets(&mut self) {
        let n = self.entities.len();
        for i in 0..n {
            if self.entities[i].dead || !is_mobile(self.entities[i].kind) {
                continue;
            }
            let desired = if let Some(ti) = self.entities[i].attack_target {
                if ti < n && !self.entities[ti].dead {
                    facing_toward(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y)
                }
                else {
                    self.entities[i].facing
                }
            }
            else {
                self.entities[i].facing
            };
            turn_facing_toward(&mut self.entities[i].turret_facing, desired, TURRET_TURN_STEP);
        }
    }

    fn apply_commands(&mut self, cmds: &[GameCommand]) {
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
                    let armor = self
                        .techno_types
                        .get(building_type)
                        .map(|t| t.armor.clone())
                        .unwrap_or_else(|| "none".into());
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
                    self.players[player_index].funds_spent =
                        self.players[player_index].funds_spent.saturating_add(cost);
                    if power >= 0 {
                        self.players[player_index].power_output =
                            self.players[player_index].power_output.saturating_add(power);
                    } else {
                        self.players[player_index].power_drain =
                            self.players[player_index].power_drain.saturating_add(-power);
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
                        } else {
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
                    self.players[player_index].funds_spent =
                        self.players[player_index].funds_spent.saturating_add(cost);
                    self.entities[factory_index].produce_queue =
                        Some((type_id.to_ascii_uppercase(), PRODUCE_TICKS));
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

    fn reject(&mut self, command_index: usize, reason: CommandRejectReason) {
        self.last_rejects.push(CommandReject { command_index, reason });
    }

    fn advance_refinery_income(&mut self) {
        let mut credits: Vec<(String, i32)> = Vec::new();
        for e in &mut self.entities {
            if e.dead || !is_refinery(&e.type_id) {
                continue;
            }
            e.ore_trip_accum = e.ore_trip_accum.saturating_add(1);
            if e.ore_trip_accum >= ORE_TRIP_TICKS {
                e.ore_trip_accum = 0;
                credits.push((e.owner.clone(), ORE_INCOME_PER_TRIP as i32));
            }
        }
        for (house, amount) in credits {
            if let Some(player) = self.players.iter_mut().find(|p| p.house == house) {
                player.funds = player.funds.saturating_add(amount);
            }
        }
    }

    fn advance_production(&mut self) {
        let mut spawns: Vec<(usize, String)> = Vec::new();
        for (index, e) in self.entities.iter_mut().enumerate() {
            if e.dead {
                continue;
            }
            let Some((type_id, remaining)) = e.produce_queue.as_mut()
            else {
                continue;
            };
            if *remaining > 1 {
                *remaining -= 1;
                continue;
            }
            let type_id = type_id.clone();
            e.produce_queue = None;
            spawns.push((index, type_id));
        }
        for (factory_index, type_id) in spawns {
            self.spawn_produced_unit(factory_index, &type_id);
        }
    }

    fn spawn_produced_unit(&mut self, factory_index: usize, type_id: &str) {
        let Some(tt) = self.techno_types.get(type_id).cloned()
        else {
            return;
        };
        let factory = &self.entities[factory_index];
        let owner = factory.owner.clone();
        let fx = factory.x;
        let fy = factory.y;
        let rally = match (factory.rally_x, factory.rally_y) {
            (Some(rx), Some(ry)) => Some((rx, ry)),
            _ => None,
        };
        let Some((x, y)) = self.find_spawn_cell(fx, fy)
        else {
            return;
        };
        let kind = match tt.kind {
            TechnoKind::Infantry => MapEntityKind::Infantry,
            TechnoKind::Vehicle => MapEntityKind::Unit,
            TechnoKind::Aircraft => MapEntityKind::Aircraft,
            TechnoKind::Building => return,
        };
        let max_health = tt.strength.max(1);
        let id = self.alloc_entity_id();
        let unit_index = self.entities.len();
        self.entities.push(WorldEntity {
            id,
            kind,
            owner,
            type_id: type_id.to_ascii_uppercase(),
            x,
            y,
            facing: 0,
            turret_facing: 0,
            sub_cell: 0,
            health: max_health,
            max_health,
            speed: tt.speed,
            armor: tt.armor.clone(),
            attack_range: if tt.range > 0 { tt.range } else { tt.sight.max(1) },
            attack_damage: if tt.damage > 0 { tt.damage } else { (tt.strength / 4).max(1) },
            attack_cooldown_max: if tt.rof > 0 { tt.rof } else { ATTACK_COOLDOWN_TICKS },
            attack_verses: verses_for(&self.warheads, &tt.warhead),
            techno_kind: Some(tt.kind),
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
        if let Some((rx, ry)) = rally {
            let e = &mut self.entities[unit_index];
            e.target_x = Some(rx);
            e.target_y = Some(ry);
            e.path.clear();
            e.move_accum = 0;
            repath_at(&mut self.entities, unit_index, &self.pass_grid);
        }
    }

    fn find_spawn_cell(&self, fx: u16, fy: u16) -> Option<(u16, u16)> {
        const DELTAS: [(i32, i32); 8] =
            [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (-1, -1), (1, -1)];
        for (dx, dy) in DELTAS {
            let x = i32::from(fx) + dx;
            let y = i32::from(fy) + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as u16, y as u16);
            if self.can_place_structure(x, y) {
                return Some((x, y));
            }
        }
        None
    }

    fn find_factory(&self, house: &str, kind: TechnoKind) -> Option<usize> {
        self.entities.iter().position(|e| {
            !e.dead
                && e.owner == house
                && e.kind == MapEntityKind::Structure
                && factory_matches_unit(&e.type_id, kind)
        })
    }

    fn find_idle_factory(&self, house: &str, kind: TechnoKind) -> Option<usize> {
        self.entities.iter().position(|e| {
            !e.dead
                && e.owner == house
                && e.kind == MapEntityKind::Structure
                && e.produce_queue.is_none()
                && factory_matches_unit(&e.type_id, kind)
        })
    }

    fn house_has_living_yard(&self, house: &str) -> bool {
        self.entities.iter().any(|e| {
            !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_construction_yard(&e.type_id)
        })
    }

    fn house_has_living_power(&self, house: &str) -> bool {
        self.entities.iter().any(|e| {
            !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_power_plant(&e.type_id)
        })
    }

    /// 目标格是否可放置单格建筑（界内、可通行、无占用实体）。
    pub fn can_place_structure(&self, x: u16, y: u16) -> bool {
        if !self.pass_grid.in_bounds(x, y) {
            return false;
        }
        if !self.pass_grid.is_passable(x, y) {
            return false;
        }
        !self.entities.iter().any(|e| !e.dead && e.x == x && e.y == y)
    }

    /// 当前确定性状态哈希（锁步校验用）。
    pub fn state_hash(&self) -> u64 {
        self.state_hash
    }

    /// 已绑定 techno 规则的实体数量。
    pub fn bound_techno_count(&self) -> usize {
        self.entities.iter().filter(|e| e.techno_kind.is_some()).count()
    }

    /// 通行表变更后，为全部移动单位重算路径。
    pub fn repath_mobiles(&mut self) {
        for i in 0..self.entities.len() {
            if is_mobile(self.entities[i].kind) {
                repath_at(&mut self.entities, i, &self.pass_grid);
            }
        }
        self.rehash();
    }

    fn rehash(&mut self) {
        let mut h = self.tick;
        h = h.wrapping_mul(1099511628211).wrapping_add(self.edition.as_str().len() as u64);
        h = h.wrapping_mul(1099511628211).wrapping_add(self.entities.len() as u64);
        h = h
            .wrapping_mul(1099511628211)
            .wrapping_add(self.last_input_frame.tick)
            .wrapping_add(self.last_input_frame.commands.len() as u64);
        for cmd in &self.last_input_frame.commands {
            h = hash_command(h, cmd);
        }
        for e in &self.entities {
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(e.id.0)
                .wrapping_add(e.x as u64)
                .wrapping_add((e.y as u64) << 16)
                .wrapping_add((e.facing as u64) << 32)
                .wrapping_add((e.turret_facing as u64) << 40)
                .wrapping_add(u64::from(e.health))
                .wrapping_add(u64::from(e.max_health).wrapping_shl(1))
                .wrapping_add(u64::from(e.speed).wrapping_shl(2))
                .wrapping_add(u64::from(e.attack_range).wrapping_shl(3))
                .wrapping_add(u64::from(e.attack_damage).wrapping_shl(4))
                .wrapping_add(u64::from(e.attack_cooldown_max).wrapping_shl(5))
                .wrapping_add(u64::from(e.dead))
                .wrapping_add(u64::from(e.hva_frame) << 8)
                .wrapping_add(u64::from(e.attack_cooldown) << 24)
                .wrapping_add(u64::from(e.ore_trip_accum) << 8)
                .wrapping_add(u64::from(e.hit_flash) << 16)
                .wrapping_add(e.attack_target.map(|i| i as u64 + 1).unwrap_or(0) << 32);
            for b in e.armor.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
            for v in e.attack_verses {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(v));
            }
            if let Some((ref qid, rem)) = e.produce_queue {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(rem));
                for b in qid.as_bytes() {
                    h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
                }
            }
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(e.rally_x.map(u64::from).unwrap_or(0))
                .wrapping_add(e.rally_y.map(|v| u64::from(v) << 16).unwrap_or(0));
            for b in e.type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
            for b in e.owner.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        for p in &self.players {
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(u64::from(p.id.0))
                .wrapping_add(p.funds as u64)
                .wrapping_add(p.funds_spent as u64)
                .wrapping_add((p.power_output as u64) << 16)
                .wrapping_add((p.power_drain as u64) << 32);
            for b in p.house.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        self.state_hash = h;
    }
}

fn hash_command(mut h: u64, cmd: &GameCommand) -> u64 {
    match *cmd {
        GameCommand::MoveTo { entity_index, x, y } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(1);
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(entity_index as u64)
                .wrapping_add((x as u64) << 16)
                .wrapping_add((y as u64) << 32);
        }
        GameCommand::Attack { attacker_index, target_index } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(2);
            h = h.wrapping_mul(1099511628211).wrapping_add(attacker_index as u64).wrapping_add((target_index as u64) << 16);
        }
        GameCommand::Deploy { entity_index } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(3);
            h = h.wrapping_mul(1099511628211).wrapping_add(entity_index as u64);
        }
        GameCommand::PlaceBuilding { player, ref type_id, x, y } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(4);
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(u64::from(player.0))
                .wrapping_add((x as u64) << 8)
                .wrapping_add((y as u64) << 24);
            for b in type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        GameCommand::Produce { player, ref type_id } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(5);
            h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(player.0));
            for b in type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        GameCommand::SetRallyPoint { factory_index, x, y } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(6);
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(factory_index as u64)
                .wrapping_add((x as u64) << 16)
                .wrapping_add((y as u64) << 32);
        }
    }
    h
}

/// 冻结竖切内 MCV → 建造场映射（后续可由 adaptor 定义表替换）。
fn deploy_into_type(type_id: &str) -> Option<&'static str> {
    match type_id {
        "AMCV" => Some("GACNST"),
        "SMCV" => Some("NACNST"),
        _ => None,
    }
}

fn is_construction_yard(type_id: &str) -> bool {
    matches!(type_id, "GACNST" | "NACNST")
}

fn is_power_plant(type_id: &str) -> bool {
    matches!(type_id, "GAPOWR" | "NAPOWR")
}

fn requires_power_plant(type_id: &str) -> bool {
    matches!(type_id, "GAPILE" | "NAHAND" | "GAWEAP" | "NAWEAP" | "GAREFN" | "NAREFN")
}

fn is_refinery(type_id: &str) -> bool {
    matches!(type_id, "GAREFN" | "NAREFN")
}

fn factory_matches_unit(factory_type: &str, kind: TechnoKind) -> bool {
    match kind {
        TechnoKind::Infantry => matches!(factory_type, "GAPILE" | "NAHAND"),
        TechnoKind::Vehicle => matches!(factory_type, "GAWEAP" | "NAWEAP"),
        TechnoKind::Aircraft | TechnoKind::Building => false,
    }
}

fn is_production_factory(type_id: &str) -> bool {
    matches!(type_id, "GAPILE" | "NAHAND" | "GAWEAP" | "NAWEAP")
}

/// 冻结竖切建筑的电力增量（正=供电，负=耗电）。后续由 adaptor 定义替换。
fn building_power_delta(type_id: &str) -> i32 {
    match type_id {
        "GAPOWR" | "NAPOWR" => 200,
        "GAPILE" | "NAHAND" => -20,
        "GAWEAP" | "NAWEAP" => -30,
        "GAREFN" | "NAREFN" => -50,
        _ => 0,
    }
}

fn is_mobile(kind: MapEntityKind) -> bool {
    matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
}

fn turn_facing_toward(current: &mut u8, desired: u8, step: u8) {
    if *current == desired || step == 0 {
        return;
    }
    let cur = i16::from(*current);
    let want = i16::from(desired);
    let mut delta = (want - cur).rem_euclid(256);
    if delta > 128 {
        delta -= 256;
    }
    let step = i16::from(step);
    let moved = if delta > 0 { delta.min(step) } else { delta.max(-step) };
    *current = (cur + moved).rem_euclid(256) as u8;
}

fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    (i32::from(ax) - i32::from(bx)).unsigned_abs() + (i32::from(ay) - i32::from(by)).unsigned_abs()
}

/// 粗 8 向朝向（与迈格 facing 桶对齐）。
fn facing_toward(from_x: u16, from_y: u16, to_x: u16, to_y: u16) -> u8 {
    let dx = (i32::from(to_x) - i32::from(from_x)).signum();
    let dy = (i32::from(to_y) - i32::from(from_y)).signum();
    match (dx, dy) {
        (1, 0) => 0,
        (1, 1) => 32,
        (0, 1) => 64,
        (-1, 1) => 96,
        (-1, 0) => 128,
        (-1, -1) => 160,
        (0, -1) => 192,
        (1, -1) => 224,
        _ => 0,
    }
}

fn full_verses() -> [u32; 11] {
    [100; 11]
}

fn verses_for(warheads: &WarheadRegistry, warhead: &str) -> [u32; 11] {
    warheads.get(warhead).map(|w| w.verses).unwrap_or_else(full_verses)
}

fn scale_damage(base: u32, verses: &[u32; 11], armor: &str) -> u32 {
    let pct = verses[armor_index(armor)];
    ((u64::from(base) * u64::from(pct)) / 100) as u32
}

fn cell_occupied_by_other(entities: &[WorldEntity], self_i: usize, x: u16, y: u16) -> bool {
    entities.iter().enumerate().any(|(j, o)| j != self_i && !o.dead && is_mobile(o.kind) && o.x == x && o.y == y)
}

/// 寻路时把其它移动单位占格封死；目标被占则改停邻格。
fn repath_at(entities: &mut [WorldEntity], i: usize, grid: &PassGrid) {
    entities[i].path.clear();
    let (Some(tx), Some(ty)) = (entities[i].target_x, entities[i].target_y)
    else {
        return;
    };
    let (sx, sy) = (entities[i].x, entities[i].y);
    let mut g = grid.clone();
    for (j, o) in entities.iter().enumerate() {
        if j != i && !o.dead && is_mobile(o.kind) {
            g.set_passable(o.x, o.y, false);
        }
    }
    // 允许离开当前格。
    g.set_passable(sx, sy, true);
    let (gx, gy) = nearest_free_goal(&g, sx, sy, tx, ty);
    g.set_passable(gx, gy, true);
    let Some(mut path) = g.find_path_diag(sx, sy, gx, gy)
    else {
        return;
    };
    if path.first() == Some(&(sx, sy)) {
        path.remove(0);
    }
    entities[i].path = path;
}

/// 目标可走则用之；否则在半径内找最近可走格（含自身起点）。
fn nearest_free_goal(grid: &PassGrid, sx: u16, sy: u16, tx: u16, ty: u16) -> (u16, u16) {
    if grid.is_passable(tx, ty) {
        return (tx, ty);
    }
    let mut best: Option<(u32, u16, u16)> = None;
    for r in 1i32..=8 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let nx = i32::from(tx) + dx;
                let ny = i32::from(ty) + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nx = nx as u16;
                let ny = ny as u16;
                if !grid.is_passable(nx, ny) {
                    continue;
                }
                // 允许停在自己脚下（已到邻格排队）。
                let dist = (i32::from(nx) - i32::from(sx)).pow(2) + (i32::from(ny) - i32::from(sy)).pow(2);
                let key = (dist as u32, nx, ny);
                if best.map(|b| key < b).unwrap_or(true) {
                    best = Some(key);
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, x, y)| (x, y)).unwrap_or((tx, ty))
}

/// 沿 `path` 迈一格；无路则返回 `false`。
fn step_along_path(e: &mut WorldEntity) -> bool {
    let Some((nx, ny)) = e.path.first().copied()
    else {
        return false;
    };
    e.path.remove(0);
    let dx = i32::from(nx) - i32::from(e.x);
    let dy = i32::from(ny) - i32::from(e.y);
    e.facing = match (dx.signum(), dy.signum()) {
        (1, 0) => 0,
        (1, 1) => 32,
        (0, 1) => 64,
        (-1, 1) => 96,
        (-1, 0) => 128,
        (-1, -1) => 160,
        (0, -1) => 192,
        (1, -1) => 224,
        _ => e.facing,
    };
    e.x = nx;
    e.y = ny;
    true
}
