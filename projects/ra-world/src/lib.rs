//! 确定性世界推进。不依赖渲染器与文件系统。

#![deny(missing_docs)]

mod combat;
mod command;
mod economy;
mod entity;
mod navigation;
mod player;
mod production;
mod reject;
mod rules;
mod state_hash;

use ra_adaptor::RulesDb;
use ra_assets::{TechnoTypeRegistry, WarheadRegistry};
use ra_map::{MapInfo, PassGrid};
use ra_types::{EntityId, GameEdition, PlayerId};

pub use command::{GameCommand, InputFrame, decode_command, decode_commands, encode_command, encode_commands};
pub use entity::WorldEntity;
use navigation::{is_mobile, repath_at};
pub use player::PlayerState;
pub use reject::{CommandReject, CommandRejectReason};
use rules::{full_verses, verses_for};

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
                let attack_range =
                    tt.map(|t| if t.range > 0 { t.range } else { t.sight.max(1) }).unwrap_or(DEFAULT_ATTACK_RANGE);
                let attack_damage =
                    tt.map(|t| if t.damage > 0 { t.damage } else { (t.strength / 4).max(1) }).unwrap_or(DEFAULT_ATTACK_DAMAGE);
                let attack_cooldown_max =
                    tt.map(|t| if t.rof > 0 { t.rof } else { ATTACK_COOLDOWN_TICKS }).unwrap_or(ATTACK_COOLDOWN_TICKS);
                let armor = tt.map(|t| t.armor.clone()).unwrap_or_else(|| "none".into());
                let attack_verses = tt.map(|t| verses_for(&rules.warheads, &t.warhead)).unwrap_or_else(full_verses);
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
        let players: Vec<PlayerState> =
            house_order.into_iter().enumerate().map(|(i, house)| PlayerState::new(PlayerId(i as u8), house)).collect();
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
        }
        else {
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
}
