//! 超武与特殊能力（闪电风暴等）。
//!
//! 当前切片只推进闪电风暴倒计时，并在激活 / 结束时切换地图 `LightingProfile`；
//! 落雷伤害与粒子另做。

use ra_map::LightingProfile;

use crate::state::BattleState;

/// 结束态标记：持续时间耗尽后保留一帧 Ion，再清场并切回 Normal。
pub const ENDING_DURATION_SENTINEL: i32 = i32::MIN;

/// 全局唯一的闪电风暴状态（同时最多一场）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct LightningStormState {
    /// 风暴中心格 X。
    pub target_x: u16,
    /// 风暴中心格 Y。
    pub target_y: u16,
    /// 激活前剩余 tick（>0 时仍为 Normal）。
    pub deferment_remaining: i32,
    /// 激活后剩余持续时间 tick（`-1` 表示无限）。
    pub duration_remaining: i32,
}

impl LightningStormState {
    /// 构造一场风暴。
    pub fn new(target_x: u16, target_y: u16, deferment: i32, duration: i32) -> Self {
        Self { target_x, target_y, deferment_remaining: deferment.max(0), duration_remaining: duration }
    }
}

/// 启动或重瞄闪电风暴。
///
/// - 无进行中风暴：写入新状态；若 `deferment <= 0` 立刻切 Ion。
/// - 已在延迟中：可缩短延迟并刷新目标 / 持续时间；缩短到 0 则立刻切 Ion。
/// - 已激活：只更新目标格，不重置寿命。
pub fn start_lightning_storm(world: &mut BattleState, target_x: u16, target_y: u16, deferment: i32, duration: i32) {
    if world.lightning_storm.is_some() {
        let begin_now = {
            let storm = world.lightning_storm.as_mut().expect("刚检查过存在");
            storm.target_x = target_x;
            storm.target_y = target_y;
            if storm.deferment_remaining > 0 {
                let requested = deferment.max(0);
                if requested <= storm.deferment_remaining {
                    storm.deferment_remaining = requested;
                }
                storm.duration_remaining = duration;
                storm.deferment_remaining == 0
            }
            else {
                false
            }
        };
        if begin_now {
            begin_lightning_storm(world);
        }
        return;
    }

    let storm = LightningStormState::new(target_x, target_y, deferment, duration);
    let starts_active = storm.deferment_remaining == 0;
    world.lightning_storm = Some(storm);
    if starts_active {
        begin_lightning_storm(world);
    }
}

#[doc(hidden)]
pub fn begin_lightning_storm(world: &mut BattleState) {
    world.map.set_lighting_profile(LightingProfile::Ion);
}

#[doc(hidden)]
pub fn end_lightning_storm(world: &mut BattleState) {
    world.lightning_storm = None;
    world.map.set_lighting_profile(LightingProfile::Normal);
}

/// 推进一场闪电风暴一个 tick，并同步光照档。
pub fn tick_lightning_storm(world: &mut BattleState) {
    let activates_now = match world.lightning_storm.as_mut() {
        None => return,
        Some(storm) if storm.deferment_remaining > 0 => {
            storm.deferment_remaining -= 1;
            if storm.deferment_remaining > 0 {
                return;
            }
            true
        }
        Some(_) => false,
    };
    if activates_now {
        begin_lightning_storm(world);
        // 延迟归零帧只切 Ion，不扣持续时间。
        return;
    }

    let duration = world.lightning_storm.as_ref().expect("风暴在延迟处理后仍应存在").duration_remaining;
    if duration == ENDING_DURATION_SENTINEL {
        end_lightning_storm(world);
        return;
    }
    if duration == 0 {
        world.lightning_storm.as_mut().expect("进入结束态时风暴仍应存在").duration_remaining = ENDING_DURATION_SENTINEL;
        return;
    }
    if duration < 0 {
        // `-1` 无限持续：保持 Ion，不扣减。
        return;
    }

    let storm = world.lightning_storm.as_mut().expect("激活风暴仍应存在");
    storm.duration_remaining -= 1;
}

/// `RechargeTime` 原版分钟档 → 逻辑 tick（竖切换算；后续可接速度档）。
pub const SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT: u32 = 90;

/// 单房主一条超武充能槽。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct SuperWeaponCharge {
    /// `[SuperWeaponTypes]` 类型键。
    pub type_key: ra_types::SuperWeaponName,
    /// 已累计充能 tick。
    pub charge_ticks: u32,
    /// 就绪所需 tick（来自 `RechargeTime` × 换算）。
    pub required_ticks: u32,
}

impl SuperWeaponCharge {
    /// 是否已充能就绪。
    pub fn is_ready(&self) -> bool {
        self.required_ticks > 0 && self.charge_ticks >= self.required_ticks
    }
}

/// 局内超武充能运行时（按 house → 类型键）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[doc(hidden)]
pub struct SuperWeaponRuntime {
    /// house 名 → 该阵营可用超武充能表。
    by_house: std::collections::BTreeMap<String, Vec<SuperWeaponCharge>>,
}

impl SuperWeaponRuntime {
    /// 查询某 house 某超武充能进度。
    pub fn charge(&self, house: &str, type_key: &ra_types::SuperWeaponName) -> Option<&SuperWeaponCharge> {
        self.by_house.get(house).and_then(|list| list.iter().find(|c| &c.type_key == type_key))
    }

    fn charge_mut(&mut self, house: &str, type_key: &ra_types::SuperWeaponName) -> Option<&mut SuperWeaponCharge> {
        self.by_house.get_mut(house).and_then(|list| list.iter_mut().find(|c| &c.type_key == type_key))
    }

    fn ensure_slot(&mut self, house: &str, type_key: ra_types::SuperWeaponName, required_ticks: u32) {
        let house_key = house.to_string();
        let list = self.by_house.entry(house_key).or_default();
        if let Some(slot) = list.iter_mut().find(|c| c.type_key == type_key) {
            slot.required_ticks = required_ticks.max(1);
            return;
        }
        list.push(SuperWeaponCharge { type_key, charge_ticks: 0, required_ticks: required_ticks.max(1) });
    }

    /// 释放成功后清零充能。
    pub fn reset_charge(&mut self, house: &str, type_key: &ra_types::SuperWeaponName) {
        if let Some(slot) = self.charge_mut(house, type_key) {
            slot.charge_ticks = 0;
        }
    }
}

#[doc(hidden)]
pub fn required_ticks_for_sw(def: &ra_types::SuperWeaponDefinition) -> u32 {
    let units = def.recharge_time.max(1) as u32;
    units.saturating_mul(SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT)
}

/// 每个逻辑 tick：有挂接超武的存活建筑时推进对应 house 充能。
pub fn tick_super_weapon_charges(world: &mut BattleState) {
    use crate::state::components::{Health, Identity, Owner};
    use ra_map::MapEntityKind;

    let defs = std::sync::Arc::clone(&world.definitions);
    let mut active: Vec<(String, ra_types::SuperWeaponName, u32)> = Vec::new();
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if identity.kind != MapEntityKind::Structure {
            continue;
        }
        let Some(structure) = defs.structures.get(identity.type_id.as_ref())
        else {
            continue;
        };
        let Some(sw_def) = structure
            .super_weapon_id
            .and_then(|id| defs.super_weapons.get_by_id(id))
            .or_else(|| structure.super_weapon.as_ref().and_then(|k| defs.super_weapons.get_name(k)))
        else {
            continue;
        };
        let Some(owner) = world.ecs_get::<Owner>(id).map(|o| o.house.as_ref().to_string())
        else {
            continue;
        };
        active.push((owner, sw_def.type_key.clone(), required_ticks_for_sw(sw_def)));
    }

    for (house, sw_key, required) in active {
        world.super_weapon_runtime.ensure_slot(&house, sw_key.clone(), required);
        if let Some(slot) = world.super_weapon_runtime.charge_mut(&house, &sw_key) {
            if slot.charge_ticks < slot.required_ticks {
                slot.charge_ticks = slot.charge_ticks.saturating_add(1);
            }
        }
    }
}

/// 尝试释放超武：成功则清零充能并按 `Type=` 触发效果（当前仅 `LightningStorm` 切光照）。
pub fn try_fire_super_weapon(world: &mut BattleState, house: &str, type_key: &str, x: u16, y: u16) -> Result<(), FireSuperWeaponError> {
    use crate::state::components::{Health, Identity, Owner};
    use ra_map::MapEntityKind;

    let type_key = ra_types::SuperWeaponName::parse(type_key);
    let Some(sw_def) = world.definitions.super_weapons.get_name(&type_key)
    else {
        return Err(FireSuperWeaponError::UnknownType);
    };
    let has_provider = world.entities.iter().any(|e| {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return false;
        }
        if world.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != house) {
            return false;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            return false;
        };
        if identity.kind != MapEntityKind::Structure {
            return false;
        }
        world
            .definitions
            .structures
            .get(identity.type_id.as_ref())
            .is_some_and(|s| s.super_weapon_id == Some(sw_def.id) || s.super_weapon.as_ref() == Some(&type_key))
    });
    if !has_provider {
        return Err(FireSuperWeaponError::NoProvider);
    }
    let ready = world.super_weapon_runtime.charge(house, &type_key).is_some_and(SuperWeaponCharge::is_ready);
    if !ready {
        return Err(FireSuperWeaponError::NotReady);
    }

    match sw_def.kind.as_str() {
        "LIGHTNINGSTORM" => {
            // 竖切：立即激活，持续 90 tick。
            start_lightning_storm(world, x, y, 0, 90);
        }
        _ => {
            // 未接线类型：仍消耗充能并记成功，效果后置（避免静默半可玩用 CapabilityGap 另报）。
            // 当前拒绝未知玩法类型，迫使后续接线。
            return Err(FireSuperWeaponError::UnsupportedKind);
        }
    }
    world.super_weapon_runtime.reset_charge(house, &type_key);
    Ok(())
}

/// 释放超武失败原因（映射到命令拒绝）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum FireSuperWeaponError {
    /// 定义表无此类型。
    UnknownType,
    /// 本方无挂接该超武的存活建筑。
    NoProvider,
    /// 充能未满。
    NotReady,
    /// `Type=` 玩法尚未接线。
    UnsupportedKind,
}
