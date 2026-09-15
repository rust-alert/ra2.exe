//! 超武与特殊能力（闪电风暴等）。
//!
//! 当前切片：推进闪电风暴倒计时，激活／结束时切换地图 `LightingProfile`，
//! 并在激活期间按间隔对目标格邻域造成落雷伤害。粒子特效另做。

use ra_map::{LightingProfile, MapEntityKind};

use crate::state::{
    BattleState,
    components::{Health, Identity, Transform},
};

/// 结束态标记：持续时间耗尽后保留一帧 Ion，再清场并切回 Normal。
pub const ENDING_DURATION_SENTINEL: i32 = i32::MIN;

/// 激活期两次落雷之间的逻辑 tick 间隔（竖切，非原版精确表）。
pub const LIGHTNING_STRIKE_INTERVAL_TICKS: i32 = 15;

/// 落雷相对目标格的切比雪夫半径（含中心格）。
pub const LIGHTNING_STRIKE_RADIUS: i32 = 1;

/// 单次落雷对命中单位／建筑的伤害（竖切）。
pub const LIGHTNING_STRIKE_DAMAGE: u32 = 50;

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
    /// 距下一次落雷的剩余 tick（0 表示本帧应击打）。
    pub strike_cooldown: i32,
}

impl LightningStormState {
    /// 构造一场风暴。
    pub fn new(target_x: u16, target_y: u16, deferment: i32, duration: i32) -> Self {
        Self { target_x, target_y, deferment_remaining: deferment.max(0), duration_remaining: duration, strike_cooldown: 0 }
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

/// 推进一场闪电风暴一个 tick，并同步光照档与落雷伤害。
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
        // 延迟归零帧只切 Ion，不扣持续时间；立即尝试首击。
        apply_lightning_strike_if_due(world);
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
        // `-1` 无限持续：保持 Ion，不扣减，仍落雷。
        apply_lightning_strike_if_due(world);
        return;
    }

    apply_lightning_strike_if_due(world);
    let storm = world.lightning_storm.as_mut().expect("激活风暴仍应存在");
    storm.duration_remaining -= 1;
}

/// 若冷却到期，对目标邻域造成一次落雷伤害并重置间隔。
fn apply_lightning_strike_if_due(world: &mut BattleState) {
    let (cx, cy) = {
        let Some(storm) = world.lightning_storm.as_mut()
        else {
            return;
        };
        if storm.deferment_remaining > 0 || storm.duration_remaining == ENDING_DURATION_SENTINEL || storm.duration_remaining == 0 {
            return;
        }
        if storm.strike_cooldown > 0 {
            storm.strike_cooldown -= 1;
            return;
        }
        storm.strike_cooldown = LIGHTNING_STRIKE_INTERVAL_TICKS;
        (storm.target_x, storm.target_y)
    };
    apply_lightning_strike_at(world, cx, cy);
}

/// 对中心格切比雪夫半径内的存活实体造成 [`LIGHTNING_STRIKE_DAMAGE`]。
fn apply_lightning_strike_at(world: &mut BattleState, cx: u16, cy: u16) {
    let radius = LIGHTNING_STRIKE_RADIUS;
    let mut hit_indices = Vec::new();
    for (index, entity) in world.entities.iter().enumerate() {
        let id = entity.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            continue;
        };
        let is_structure = world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false);
        let covers = if is_structure {
            let type_id = world.ecs_get::<Identity>(id).map(|i| i.type_id);
            let foundation =
                type_id.and_then(|tid| world.definitions.structures.get_by_id(tid)).map(|s| s.foundation.clone()).unwrap_or_default();
            let fw = foundation.width.max(1) as i32;
            let fh = foundation.height.max(1) as i32;
            // 建筑占地与风暴圆盘是否相交（格中心切比雪夫距离）。
            let left = xf.x as i32;
            let top = xf.y as i32;
            let right = left + fw - 1;
            let bottom = top + fh - 1;
            let nearest_x = (cx as i32).clamp(left, right);
            let nearest_y = (cy as i32).clamp(top, bottom);
            (nearest_x - cx as i32).abs().max((nearest_y - cy as i32).abs()) <= radius
        }
        else {
            let dx = (xf.x as i32 - cx as i32).abs();
            let dy = (xf.y as i32 - cy as i32).abs();
            dx.max(dy) <= radius
        };
        if covers {
            hit_indices.push(index);
        }
    }
    for index in hit_indices {
        world.apply_damage(index, LIGHTNING_STRIKE_DAMAGE);
    }
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
        self.charges_for_house(house).and_then(|list| list.iter().find(|c| &c.type_key == type_key))
    }

    /// 某房主全部超武充能槽（大小写不敏感匹配 house）。
    pub fn charges_for_house(&self, house: &str) -> Option<&[SuperWeaponCharge]> {
        self.by_house.iter().find(|(k, _)| k.eq_ignore_ascii_case(house)).map(|(_, list)| list.as_slice())
    }

    /// 覆盖写入充能进度（测试 / 诊断；正式 tick 由 `tick_super_weapons` 推进）。
    pub fn set_charge_for_test(&mut self, house: &str, type_key: ra_types::SuperWeaponName, charge_ticks: u32, required_ticks: u32) {
        let required_ticks = required_ticks.max(1);
        self.ensure_slot(house, type_key.clone(), required_ticks);
        if let Some(slot) = self.charge_mut(house, &type_key) {
            slot.charge_ticks = charge_ticks.min(required_ticks);
            slot.required_ticks = required_ticks;
        }
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
        let Some(structure) = defs.structures.get_by_id(identity.type_id)
        else {
            continue;
        };
        let Some(sw_def) = structure.super_weapon_id.and_then(|id| defs.super_weapons.get_by_id(id))
        else {
            continue;
        };
        let Some(owner) = world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_key_of(&world.definitions, o.house).to_string())
        else {
            continue;
        };
        // 低电 / 断电时超武建筑停止充能（与雷达、周期产钱一致）。
        if world.house_is_low_power(&owner) {
            continue;
        }
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
pub fn try_fire_super_weapon(
    world: &mut BattleState,
    house: &str,
    type_id: ra_types::TypeId,
    x: u16,
    y: u16,
) -> Result<(), FireSuperWeaponError> {
    use crate::state::components::{Health, Identity, Owner};
    use ra_map::MapEntityKind;

    let Some(sw_def) = world.definitions.super_weapons.get_by_id(type_id)
    else {
        return Err(FireSuperWeaponError::UnknownType);
    };
    let type_key = sw_def.type_key.clone();
    let has_provider = world.entities.iter().any(|e| {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return false;
        }
        if world.ecs_get::<Owner>(id).is_none_or(|o| crate::gameplay::house_id_of(&world.definitions, house) != Some(o.house)) {
            return false;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            return false;
        };
        if identity.kind != MapEntityKind::Structure {
            return false;
        }
        world.definitions.structures.get_by_id(identity.type_id).is_some_and(|s| s.super_weapon_id == Some(sw_def.id))
    });
    if !has_provider {
        return Err(FireSuperWeaponError::NoProvider);
    }
    let ready = world.super_weapon_runtime.charge(house, &type_key).is_some_and(SuperWeaponCharge::is_ready);
    if !ready {
        return Err(FireSuperWeaponError::NotReady);
    }

    if !sw_def.has_registered_executor() {
        return Err(FireSuperWeaponError::UnsupportedKind);
    }
    // 已注册执行器分发：登记表在 `super_weapon_kind_has_executor`；此处按 kind 调用实现。
    match sw_def.kind.as_str() {
        "LIGHTNINGSTORM" => {
            let rules = &world.definitions.lightning_storm;
            start_lightning_storm(world, x, y, rules.deferment_ticks as i32, rules.duration_ticks as i32);
        }
        "MULTIMISSILE" | "NUKE" => {
            let Some(weapon_id) = sw_def.weapon_id
            else {
                return Err(FireSuperWeaponError::UnsupportedKind);
            };
            super::combat::apply_weapon_strike_at(world, x, y, weapon_id, Some(house));
        }
        "IRONCURTAIN" => {
            super::effects::apply_iron_curtain_at(world, house, x, y, false);
        }
        "FORCESHIELD" => {
            super::effects::apply_iron_curtain_at(world, house, x, y, true);
        }
        "PARADROP" | "AMERPARADROP" => {
            super::effects::apply_paradrop_at(world, house, x, y);
        }
        "REVEAL" | "PSYCHICREVEAL" => {
            super::effects::apply_reveal_at(world, house, x, y);
        }
        "CHRONOSPHERE" => {
            let house_key = house.trim().to_ascii_uppercase();
            if let Some(&(sx, sy)) = world.chronosphere_arms.get(&house_key) {
                super::effects::apply_chronosphere_warp(world, house, sx, sy, x, y);
                world.chronosphere_arms.remove(&house_key);
            }
            else {
                // 第一次点击只装订源点，不消耗充能。
                world.chronosphere_arms.insert(house_key, (x, y));
                return Ok(());
            }
        }
        _ => return Err(FireSuperWeaponError::UnsupportedKind),
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
