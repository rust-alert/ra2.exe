//! 对局能力快照：科技与建造/生产可用性绑定**当前存活建筑**。
//!
//! 前置不做成永久解锁。建造场、电厂、兵营、战车工厂被摧毁后，
//! 下一帧快照即失去对应条目的 `enabled`（`MissingPrerequisite` / `InsufficientPower`）。

use std::sync::Arc;

use ra_assets::TechnoKind;
use ra_types::{EntityId, TechnoClass};

use crate::{
    game::{CommandRejectReason, SnapshotProduceQueue},
    gameplay::{
        TechTreePlayer,
        build_limit_reached, deploy_into_type, is_type_eligible, living_structure_keys, requires_power_plant,
    },
    state::{
        BattleState,
        components::{Health, Identity, Owner, ProductionQueue},
    },
};

use super::BattleSession;

/// 单条可建造 / 可生产能力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityItem {
    /// 规则类型键。
    pub type_id: Arc<str>,
    /// 造价。
    pub cost: i32,
    /// 当前是否可下单（已计入存活建筑 / 资金 / 电力）。
    pub enabled: bool,
    /// 不可用时的结构化原因。
    pub disabled_reason: Option<CommandRejectReason>,
}

/// 选中实体的部署能力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployCapability {
    /// 实体。
    pub entity: EntityId,
    /// 部署目标建筑类型。
    pub into_type: Arc<str>,
    /// 当前是否可部署。
    pub enabled: bool,
    /// 不可用原因。
    pub disabled_reason: Option<CommandRejectReason>,
}

/// 本方可释放的超级武器（绑定存活挂接建筑与充能进度）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperWeaponCapabilityItem {
    /// `[SuperWeaponTypes]` 类型键。
    pub type_id: Arc<str>,
    /// `UIName=` CSF 键（可空）。
    pub ui_name: Arc<str>,
    /// `SidebarImage=`（可空）。
    pub sidebar_image: Arc<str>,
    /// `Type=` 玩法类型字面（大写）。
    pub kind: Arc<str>,
    /// 已充能 tick。
    pub charge_ticks: u32,
    /// 就绪所需 tick。
    pub required_ticks: u32,
    /// 是否充能完毕。
    pub ready: bool,
    /// 当前是否可下发释放命令（就绪且玩法已接线）。
    pub enabled: bool,
    /// 不可用原因。
    pub disabled_reason: Option<CommandRejectReason>,
}

/// 由权威世界投影的对局能力（HUD / host intent 唯一来源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleCapabilitiesSnapshot {
    /// 本地阵营。
    pub house: Arc<str>,
    /// 资金。
    pub funds: i32,
    /// 供电。
    pub power_output: i32,
    /// 耗电。
    pub power_drain: i32,
    /// 是否仍有存活建造场（建筑科技根）。
    pub has_construction_yard: bool,
    /// 是否仍有存活电厂。
    pub has_power_plant: bool,
    /// 是否仍有存活步兵工厂。
    pub has_infantry_factory: bool,
    /// 是否仍有存活载具工厂。
    pub has_vehicle_factory: bool,
    /// 是否仍有存活飞行器工厂（机场 / 停机坪）。
    pub has_aircraft_factory: bool,
    /// 是否仍有存活雷达建筑（侧栏开图；低电时 HUD 仍应关图）。
    pub has_radar: bool,
    /// 当前选中。
    pub selected: Vec<EntityId>,
    /// 首个可部署选中项（若有）。
    pub deploy: Option<DeployCapability>,
    /// 建造栏（绑定建造场存活；非 `BuildCat=Combat`）。
    pub build_items: Vec<CapabilityItem>,
    /// 防御栏（绑定建造场存活；`BuildCat=Combat`）。
    pub defense_items: Vec<CapabilityItem>,
    /// 步兵生产（绑定兵营存活）。
    pub infantry_items: Vec<CapabilityItem>,
    /// 载具生产（绑定战车工厂存活）。
    pub vehicle_items: Vec<CapabilityItem>,
    /// 飞行器生产（绑定机场 / 停机坪存活）。
    pub aircraft_items: Vec<CapabilityItem>,
    /// 超级武器栏（绑定挂接建筑存活与充能）。
    pub super_weapon_items: Vec<SuperWeaponCapabilityItem>,
    /// 生产队列摘要。
    pub queues: Vec<SnapshotProduceQueue>,
}

impl BattleSession {
    /// 按选中与本地阵营投影能力。前置一律查**当前存活建筑**。
    pub fn snapshot_capabilities(&self, selected: &[EntityId]) -> BattleCapabilitiesSnapshot {
        let local = self
            .world
            .players
            .iter()
            .find(|p| p.id == self.world.local_player);
        let house = local
            .map(|p| p.house.clone())
            .unwrap_or_else(|| Arc::<str>::from(""));
        let funds = local.map(|p| p.funds).unwrap_or(0);
        let power_output = local.map(|p| p.power_output).unwrap_or(0);
        let power_drain = local.map(|p| p.power_drain).unwrap_or(0);
        let tech_player = local.map(TechTreePlayer::from_player).unwrap_or(TechTreePlayer {
            house: house.as_ref(),
            tech_level: 10,
            stolen_allied_tech: false,
            stolen_soviet_tech: false,
            stolen_third_tech: false,
        });

        let has_construction_yard = self.world.house_has_living_yard(house.as_ref());
        let has_power_plant = self.world.house_has_living_power(house.as_ref());
        let has_infantry_factory = self.world.find_factory(house.as_ref(), TechnoKind::Infantry).is_some();
        let has_vehicle_factory = self.world.find_factory(house.as_ref(), TechnoKind::Vehicle).is_some();
        let has_aircraft_factory = self.world.find_factory(house.as_ref(), TechnoKind::Aircraft).is_some();
        let has_radar = self.world.house_has_living_radar(house.as_ref());
        let infantry_idle = self.world.find_idle_factory(house.as_ref(), TechnoKind::Infantry).is_some();
        let vehicle_idle = self.world.find_idle_factory(house.as_ref(), TechnoKind::Vehicle).is_some();
        let aircraft_idle = self.world.find_idle_factory(house.as_ref(), TechnoKind::Aircraft).is_some();
        let living = living_structure_keys(&self.world, house.as_ref());

        let deploy = selected.iter().find_map(|&id| self.project_deploy_cap(id));
        let build_all = project_build_items(
            &self.world,
            tech_player,
            &living,
            funds,
            has_construction_yard,
            has_power_plant,
        );
        let mut build_items = Vec::new();
        let mut defense_items = Vec::new();
        for item in build_all {
            let defense = self
                .world
                .definitions
                .structures
                .get(item.type_id.as_ref())
                .is_some_and(|s| s.build_cat.is_defense_tab());
            if defense {
                defense_items.push(item);
            } else {
                build_items.push(item);
            }
        }
        let infantry_items = project_produce_items(
            &self.world,
            tech_player,
            &living,
            TechnoClass::Infantry,
            funds,
            has_infantry_factory,
            infantry_idle,
        );
        let vehicle_items = project_produce_items(
            &self.world,
            tech_player,
            &living,
            TechnoClass::Vehicle,
            funds,
            has_vehicle_factory,
            vehicle_idle,
        );
        let aircraft_items = project_produce_items(
            &self.world,
            tech_player,
            &living,
            TechnoClass::Aircraft,
            funds,
            has_aircraft_factory,
            aircraft_idle,
        );
        let super_weapon_items = project_super_weapon_items(&self.world, house.as_ref());
        let queues = self
            .world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if self.world.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != house.as_ref()) {
                    return None;
                }
                let queue = self.world.ecs_get::<ProductionQueue>(id)?;
                if let Some((type_id, remaining_ticks)) = queue.item.as_ref() {
                    let total_ticks = self
                        .world
                        .definitions
                        .techno
                        .get(type_id.as_ref())
                        .map(crate::gameplay::produce_ticks_for)
                        .unwrap_or(0);
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
                let total_ticks = self
                    .world
                    .definitions
                    .techno
                    .get(ready.as_ref())
                    .map(crate::gameplay::produce_ticks_for)
                    .unwrap_or(0);
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

        BattleCapabilitiesSnapshot {
            house,
            funds,
            power_output,
            power_drain,
            has_construction_yard,
            has_power_plant,
            has_infantry_factory,
            has_vehicle_factory,
            has_aircraft_factory,
            has_radar,
            selected: selected.to_vec(),
            deploy,
            build_items,
            defense_items,
            infantry_items,
            vehicle_items,
            aircraft_items,
            super_weapon_items,
            queues,
        }
    }

    fn project_deploy_cap(&self, id: EntityId) -> Option<DeployCapability> {
        if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return None;
        }
        let into = self.deploy_target_of(id)?;
        Some(DeployCapability {
            entity: id,
            into_type: Arc::<str>::from(into),
            enabled: true,
            disabled_reason: None,
        })
    }
}

/// 建造场没了 → 全部建筑科技掉级；需电建筑在无电厂时不可用；BuildLimit 满则灰掉。
pub fn evaluate_build_availability(
    has_construction_yard: bool,
    has_power_plant: bool,
    funds: i32,
    cost: i32,
    requires_power: bool,
    build_limit_hit: bool,
) -> (bool, Option<CommandRejectReason>) {
    if !has_construction_yard {
        return (false, Some(CommandRejectReason::MissingPrerequisite));
    }
    if requires_power && !has_power_plant {
        return (false, Some(CommandRejectReason::InsufficientPower));
    }
    if build_limit_hit {
        return (false, Some(CommandRejectReason::QueueFull));
    }
    if funds < cost {
        return (false, Some(CommandRejectReason::InsufficientFunds));
    }
    (true, None)
}

/// 对应工厂没了 → 单位科技掉级；工厂忙碌 / BuildLimit → 队列满；资金不足单独标出。
pub fn evaluate_produce_availability(
    has_factory: bool,
    factory_idle: bool,
    funds: i32,
    cost: i32,
    build_limit_hit: bool,
) -> (bool, Option<CommandRejectReason>) {
    if !has_factory {
        return (false, Some(CommandRejectReason::MissingPrerequisite));
    }
    if !factory_idle || build_limit_hit {
        return (false, Some(CommandRejectReason::QueueFull));
    }
    if funds < cost {
        return (false, Some(CommandRejectReason::InsufficientFunds));
    }
    (true, None)
}

fn project_build_items(
    world: &BattleState,
    player: TechTreePlayer<'_>,
    living: &std::collections::HashSet<String>,
    funds: i32,
    has_yard: bool,
    has_power: bool,
) -> Vec<CapabilityItem> {
    let ready = world.house_ready_building(player.house);
    let yard_idle = world
        .find_idle_factory(player.house, ra_assets::TechnoKind::Building)
        .is_some();
    let mut items: Vec<CapabilityItem> = world
        .definitions
        .structures
        .iter()
        .filter(|s| is_type_eligible(&world.definitions, player, living, &s.type_key))
        .map(|s| {
            let techno = world.definitions.techno.get(&s.type_key);
            let cost = if s.cost > 0 {
                s.cost
            } else {
                techno.map(|t| t.cost).unwrap_or(0)
            };
            let requires_power = requires_power_plant(&world.definitions, &s.type_key);
            let limit_hit = techno.is_some_and(|t| build_limit_reached(world, player.house, t));
            let key = s.type_key.as_str();
            let (enabled, disabled_reason) = if ready.as_ref().is_some_and(|r| r.as_ref().eq_ignore_ascii_case(key)) {
                // 已完工：可点选落位，不再检查资金。
                (true, None)
            } else if world.entities.iter().any(|e| {
                let id = e.id;
                !world.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != player.house)
                    && world
                    .ecs_get::<ProductionQueue>(id)
                    .and_then(|q| q.item.as_ref())
                    .is_some_and(|(queued, _)| queued.as_ref().eq_ignore_ascii_case(key))
            }) {
                // 建造中：侧栏可点以取消。
                (true, None)
            } else if has_yard && !yard_idle {
                (false, Some(CommandRejectReason::QueueFull))
            } else {
                evaluate_build_availability(has_yard, has_power, funds, cost, requires_power, limit_hit)
            };
            CapabilityItem {
                type_id: Arc::<str>::from(s.type_key.as_str()),
                cost,
                enabled,
                disabled_reason,
            }
        })
        .collect();
    items.sort_by(|a, b| a.type_id.as_ref().cmp(b.type_id.as_ref()));
    items
}

fn project_produce_items(
    world: &BattleState,
    player: TechTreePlayer<'_>,
    living: &std::collections::HashSet<String>,
    class: TechnoClass,
    funds: i32,
    has_factory: bool,
    factory_idle: bool,
) -> Vec<CapabilityItem> {
    let mut items: Vec<CapabilityItem> = world
        .definitions
        .techno
        .iter()
        .filter(|t| t.class == class)
        .filter(|t| is_type_eligible(&world.definitions, player, living, &t.type_key))
        // 可部署载具（MCV）不进常规生产栏。
        .filter(|t| deploy_into_type(&world.definitions, &t.type_key).is_none())
        .map(|t| {
            let limit_hit = build_limit_reached(world, player.house, t);
            let (enabled, disabled_reason) =
                evaluate_produce_availability(has_factory, factory_idle, funds, t.cost, limit_hit);
            CapabilityItem {
                type_id: Arc::<str>::from(t.type_key.as_str()),
                cost: t.cost,
                enabled,
                disabled_reason,
            }
        })
        .collect();
    items.sort_by(|a, b| a.type_id.as_ref().cmp(b.type_id.as_ref()));
    items
}

/// 诊断：某 house 当前存活的结构类型键（科技绑定用）。
pub fn living_structure_type_keys(world: &BattleState, house: &str) -> Vec<Arc<str>> {
    let mut keys: Vec<Arc<str>> = world
        .entities
        .iter()
        .filter_map(|e| {
            let id = e.id;
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return None;
            }
            if world.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != house) {
                return None;
            }
            let identity = world.ecs_get::<Identity>(id)?;
            if identity.kind != ra_map::MapEntityKind::Structure {
                return None;
            }
            Some(identity.type_id.clone())
        })
        .collect();
    keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    keys.dedup();
    keys
}

fn project_super_weapon_items(world: &BattleState, house: &str) -> Vec<SuperWeaponCapabilityItem> {
    use ra_map::MapEntityKind;

    let mut keys: Vec<String> = Vec::new();
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if world.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != house) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id) else {
            continue;
        };
        if identity.kind != MapEntityKind::Structure {
            continue;
        }
        let Some(sw_key) = world
            .definitions
            .structures
            .get(identity.type_id.as_ref())
            .and_then(|s| s.super_weapon.as_ref())
        else {
            continue;
        };
        let key = sw_key.to_ascii_uppercase();
        if !keys.iter().any(|k| k == &key) {
            keys.push(key);
        }
    }
    keys.sort();
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let Some(def) = world.definitions.super_weapons.get(&key) else {
            continue;
        };
        let charge = world.super_weapon_runtime.charge(house, &key);
        let required_ticks = charge.map(|c| c.required_ticks).unwrap_or_else(|| {
            let units = def.recharge_time.max(1) as u32;
            units.saturating_mul(crate::gameplay::SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT)
        });
        let charge_ticks = charge.map(|c| c.charge_ticks).unwrap_or(0);
        let ready = charge.is_some_and(|c| c.is_ready());
        let supported = def.kind.eq_ignore_ascii_case("LightningStorm");
        let (enabled, disabled_reason) = if !ready {
            (false, Some(CommandRejectReason::SuperWeaponNotReady))
        } else if !supported {
            // 已就绪但玩法未接线：侧栏可见但不可下发。
            (false, Some(CommandRejectReason::MissingPrerequisite))
        } else {
            (true, None)
        };
        out.push(SuperWeaponCapabilityItem {
            type_id: Arc::<str>::from(key),
            ui_name: Arc::<str>::from(def.ui_name.as_str()),
            sidebar_image: Arc::<str>::from(def.sidebar_image.as_str()),
            kind: Arc::<str>::from(def.kind.as_str()),
            charge_ticks,
            required_ticks,
            ready,
            enabled,
            disabled_reason,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn losing_construction_yard_disables_all_build_tech() {
        let (ok, reason) = evaluate_build_availability(true, true, 5000, 800, false, false);
        assert!(ok);
        assert_eq!(reason, None);

        let (ok, reason) = evaluate_build_availability(false, true, 5000, 800, false, false);
        assert!(!ok);
        assert_eq!(reason, Some(CommandRejectReason::MissingPrerequisite));
    }

    #[test]
    fn losing_power_plant_blocks_power_gated_buildings_only() {
        let (ok, _) = evaluate_build_availability(true, false, 5000, 800, false, false);
        assert!(ok);
        let (ok, reason) = evaluate_build_availability(true, false, 5000, 800, true, false);
        assert!(!ok);
        assert_eq!(reason, Some(CommandRejectReason::InsufficientPower));
    }

    #[test]
    fn losing_factory_disables_produce_tech() {
        let (ok, reason) = evaluate_produce_availability(false, true, 500, 200, false);
        assert!(!ok);
        assert_eq!(reason, Some(CommandRejectReason::MissingPrerequisite));

        let (ok, reason) = evaluate_produce_availability(true, false, 500, 200, false);
        assert!(!ok);
        assert_eq!(reason, Some(CommandRejectReason::QueueFull));
    }
}
