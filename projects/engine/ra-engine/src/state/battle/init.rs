use std::{collections::HashSet, sync::Arc};

use ra_map::{MapEntityKind, MapInfo, PassGrid};
use ra_types::{EntityId, GameEdition, PlayerId, RaResult, RuntimeDefinitions, TechnoClass};

use super::super::{
    components::{
        AnimationState, AttackState, CombatStats, EntitySpawnBundle, HarvesterState, Health, Identity, Locomotor, MovementState, Owner,
        ProductionQueue, Transform,
    },
    ecs_registry::EcsRegistry,
    players::PlayerState,
};
use crate::{game::InputFrame, gameplay::verses_for, presentation::DirtyEntitySet};

use super::types::{ATTACK_COOLDOWN_TICKS, BattleState};

impl BattleState {
    /// 由冻结运行时定义与地图播种新世界，并为移动单位预计算路径。
    ///
    /// 通行层与预放绑定取自 [`MapInfo::to_prepared_map`]（Foundation 骨架 + 稳定 id + occupancy 重封）。
    /// Overlay land 必须在对局装载 `seal_pass_grid_from_tmp` 之后再应用，才能重开桥面。
    ///
    /// 未知 techno / house 等引用在准备期拒绝播种。
    pub fn new(edition: GameEdition, definitions: Arc<RuntimeDefinitions>, map: MapInfo) -> RaResult<Self> {
        let prepared = map.to_prepared_map(&definitions)?;
        let pass_grid =
            PassGrid::from_prepared_pass_layers(prepared.pass_width, prepared.pass_height, &prepared.passable, &prepared.cell_heights);
        let mut next_entity_id = 1u64;
        let mut house_order: Vec<String> = Vec::new();
        let ecs = EcsRegistry::new();
        let mut seed_bundles: Vec<EntitySpawnBundle> = Vec::with_capacity(prepared.placements.len());
        for p in &prepared.placements {
            let Some(tt) = definitions.techno.get_by_id(p.definition_id)
            else {
                return Err(ra_types::RaError::Msg(format!("绑定后缺少 techno id {:?}", p.definition_id)));
            };
            let Some(house) = definitions.houses.get_by_id(p.owner)
            else {
                return Err(ra_types::RaError::Msg(format!("绑定后缺少 house id {:?}", p.owner)));
            };
            let owner_key = house.type_key.as_str();
            if !house_order.iter().any(|h| h.eq_ignore_ascii_case(owner_key)) {
                house_order.push(owner_key.to_string());
            }
            let weapon = tt.primary_id.and_then(|id| definitions.weapons.get_by_id(id));
            let max_health = tt.strength.max(1);
            let health = (u64::from(max_health) * u64::from(p.health) / 256) as u32;
            let speed = tt.speed;
            let attack_range = weapon.map(|w| if w.range > 0 { w.range } else { tt.sight.max(1) }).unwrap_or(0);
            let attack_damage = weapon.map(|w| w.damage).unwrap_or(0);
            let attack_cooldown_max = weapon.map(|w| if w.rof > 0 { w.rof } else { ATTACK_COOLDOWN_TICKS }).unwrap_or(0);
            let armor = tt.armor;
            let warhead_id = weapon.and_then(|w| w.warhead_id).or(tt.warhead_id);
            let attack_verses = verses_for(&definitions, warhead_id);
            let techno_class = Some(tt.class);
            let id = EntityId(next_entity_id);
            next_entity_id = next_entity_id.saturating_add(1);
            seed_bundles.push(EntitySpawnBundle {
                identity: Identity {
                    entity_id: id,
                    type_id: tt.id,
                    kind: match p.kind {
                        ra_types::MapPlacedEntityKind::Structure => MapEntityKind::Structure,
                        ra_types::MapPlacedEntityKind::Unit => MapEntityKind::Unit,
                        ra_types::MapPlacedEntityKind::Infantry => MapEntityKind::Infantry,
                        ra_types::MapPlacedEntityKind::Aircraft => MapEntityKind::Aircraft,
                    },
                    mission: p.mission,
                    tag: p.tag,
                },
                owner: Owner { house: house.id },
                transform: Transform { x: p.x, y: p.y, facing: p.facing, turret_facing: p.facing, sub_cell: p.sub_cell },
                health: Health { current: health, maximum: max_health, dead: false },
                locomotor: Locomotor { speed },
                movement: MovementState { destination_x: None, destination_y: None, waypoints: Vec::new(), path: Vec::new(), move_accum: 0 },
                combat: CombatStats { armor, attack_range, attack_damage, attack_cooldown_max, attack_verses, techno_class },
                attack: AttackState { target: None, cooldown: 0, infiltrate_target: None, capture_target: None, follow_target: None },
                production: ProductionQueue { item: None, ready: None, rally_x: None, rally_y: None },
                harvester: HarvesterState { ore_trip_accum: 0, cargo: 0 },
                animation: AnimationState { hva_frame: 0, hit_flash: 0, fire_flash: 0 },
            });
        }
        let default_tech = definitions.default_tech_level;
        let players: Vec<PlayerState> = house_order
            .into_iter()
            .enumerate()
            .map(|(i, house)| PlayerState::with_tech_level(PlayerId(i as u8), house, default_tech))
            .collect();
        let trigger_runtime = crate::gameplay::TriggerRuntime::from_prepared(&prepared.triggers, &prepared.events, &prepared.actions);
        let ai_trigger_runtime = crate::gameplay::AiTriggerRuntime::from_map(!prepared.ai_triggers.is_empty());
        let terrain_spawners = crate::gameplay::seed_terrain_spawners(&map, &definitions.terrain_spawners);
        let speak_delay_ticks = definitions.speak_delay_ticks;
        let mut world = Self {
            edition,
            tick: 0,
            map,
            prepared,
            overlay_types: definitions.overlays.clone(),
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
            lightning_storm: None,
            super_weapon_runtime: crate::gameplay::SuperWeaponRuntime::default(),
            trigger_runtime,
            script_team_runtime: crate::gameplay::ScriptTeamRuntime::default(),
            ai_trigger_runtime,
            terrain_spawners,
            overlay_paint_dirty: Vec::new(),
            structure_paint_dirty: Vec::new(),
            structure_buildup_dirty: Vec::new(),
            match_seed: 0,
            pending_eva_cues: Vec::new(),
            pending_battle_sfx_cues: Vec::new(),
            eva_base_under_attack: Vec::new(),
            speak_delay_ticks,
            ecs,
        };
        for bundle in seed_bundles {
            let id = bundle.identity.entity_id;
            let kind = bundle.identity.kind;
            let type_id = bundle.identity.type_id;
            let x = bundle.transform.x;
            let y = bundle.transform.y;
            let index = world.spawn_from_bundle(bundle);
            world.repath_entity_at(index);
            world.mark_entity_dirty(id);
            // 地图预放建筑也要按 Foundation 封满，不能只堵左上角一格。
            if kind == MapEntityKind::Structure {
                let foundation = world.definitions.structures.get_by_id(type_id).map(|s| s.foundation.clone()).unwrap_or_default();
                world.seal_structure_footprint(x, y, foundation.width, foundation.height);
            }
        }
        world.rehash();
        Ok(world)
    }

    /// 用当前 `pass_grid` 回写 `prepared` 通行层（TMP 封格 / overlay land 之后）。
    pub fn sync_prepared_pass_layers(&mut self) {
        let (pass_width, pass_height, passable, cell_heights) = self.pass_grid.to_prepared_pass_layers();
        self.prepared.pass_width = pass_width;
        self.prepared.pass_height = pass_height;
        self.prepared.passable = passable;
        self.prepared.cell_heights = cell_heights;
    }

    /// 若存在同名 house（大小写不敏感），将 `local_player` 切到该玩家；否则保持原值并返回 `false`。
    pub fn prefer_local_house(&mut self, house: &str) -> bool {
        if let Some(p) = self.players.iter().find(|p| p.house.eq_ignore_ascii_case(house)) {
            self.local_player = p.id;
            true
        }
        else {
            false
        }
    }

    /// 确保玩家表含有该 house（遭遇战大厅阵营）。
    ///
    /// 多人图实体多为 `Neutral` 平民，大厅所选国家不会出现在放置段里，需要显式登记。
    /// 已有同名（大小写不敏感）则不重复添加。
    pub fn ensure_house(&mut self, house: &str) {
        if house.is_empty() {
            return;
        }
        if self.players.iter().any(|p| p.house.eq_ignore_ascii_case(house)) {
            return;
        }
        let id = PlayerId(self.players.len() as u8);
        self.players.push(PlayerState::new(id, house));
    }

    /// 查询格上可采 overlay 的密度字节；无可采矿则 `None`。
    pub fn harvestable_ore_at(&self, x: u16, y: u16) -> Option<u8> {
        self.map.overlays.iter().find_map(|cell| {
            if cell.x == x && cell.y == y && self.overlay_types.is_harvestable(cell.overlay_id) { Some(cell.data) } else { None }
        })
    }

    /// 在指定格生成单位（遭遇战开局等）。格子越界、不可走或已被存活实体占用时失败。
    pub fn spawn_unit_at(&mut self, house: &str, type_id: &str, x: u16, y: u16) -> Result<EntityId, String> {
        if !self.pass_grid.in_bounds(x, y) {
            return Err(format!("生成格越界: ({x},{y}) type={type_id} house={house}"));
        }
        if !self.pass_grid.is_passable(x, y) {
            return Err(format!("生成格不可走: ({x},{y}) type={type_id} house={house}"));
        }
        if self.living_at_cell(x, y) {
            return Err(format!("生成格已被占用: ({x},{y}) type={type_id} house={house}"));
        }
        let type_key = type_id.to_ascii_uppercase();
        let Some(tt) = self.definitions.techno.get(&type_key)
        else {
            return Err(format!("未知单位类型: {type_key}"));
        };
        if tt.class == TechnoClass::Building {
            return Err(format!("spawn_unit_at 不接受建筑类型: {type_key}"));
        }
        let def_id = tt.id;
        let max_health = tt.strength.max(1);
        let speed = tt.speed;
        let armor = tt.armor;
        let weapon = tt.primary_id.and_then(|id| self.definitions.weapons.get_by_id(id));
        let class = tt.class;
        let sight = tt.sight;
        let warhead_fallback = tt.warhead_id;
        let attack_range = weapon.map(|w| if w.range > 0 { w.range } else { sight.max(1) }).unwrap_or(0);
        // 无 Primary / Damage=0 保持 0，禁止用 Strength 发明伤害。
        let attack_damage = weapon.map(|w| w.damage).unwrap_or(0);
        let attack_cooldown_max = weapon.map(|w| if w.rof > 0 { w.rof } else { ATTACK_COOLDOWN_TICKS }).unwrap_or(0);
        let warhead_id = weapon.and_then(|w| w.warhead_id).or(warhead_fallback);
        let attack_verses = verses_for(&self.definitions, warhead_id);
        let id = self.alloc_entity_id();
        let kind = match class {
            TechnoClass::Infantry => MapEntityKind::Infantry,
            TechnoClass::Vehicle => MapEntityKind::Unit,
            TechnoClass::Aircraft => MapEntityKind::Aircraft,
            TechnoClass::Building => MapEntityKind::Structure,
        };
        self.spawn_from_bundle(EntitySpawnBundle {
            identity: Identity { entity_id: id, type_id: def_id, kind, mission: None, tag: None },
            owner: Owner { house: self.definitions.houses.get(house).map(|h| h.id).ok_or_else(|| format!("未知房主: {house}"))? },
            transform: Transform { x, y, facing: 0, turret_facing: 0, sub_cell: 0 },
            health: Health { current: max_health, maximum: max_health, dead: false },
            locomotor: Locomotor { speed },
            movement: MovementState { destination_x: None, destination_y: None, waypoints: Vec::new(), path: Vec::new(), move_accum: 0 },
            combat: CombatStats { armor, attack_range, attack_damage, attack_cooldown_max, attack_verses, techno_class: Some(class) },
            attack: AttackState { target: None, cooldown: 0, infiltrate_target: None, capture_target: None, follow_target: None },
            production: ProductionQueue { item: None, ready: None, rally_x: None, rally_y: None },
            harvester: HarvesterState { ore_trip_accum: 0, cargo: 0 },
            animation: AnimationState { hva_frame: 0, hit_flash: 0, fire_flash: 0 },
        });
        self.mark_entity_dirty(id);
        if let Some(index) = self.entity_index(id) {
            self.repath_entity_at(index);
        }
        self.rehash();
        Ok(id)
    }
}
