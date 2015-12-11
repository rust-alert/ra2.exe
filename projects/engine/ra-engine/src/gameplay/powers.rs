//! 超武与特殊能力（闪电风暴等）。
//!
//! 当前切片只推进闪电风暴倒计时，并在激活 / 结束时切换地图 `LightingProfile`；
//! 落雷伤害与粒子另做。

use ra_map::LightingProfile;

use crate::state::BattleState;

/// 结束态标记：持续时间耗尽后保留一帧 Ion，再清场并切回 Normal。
const ENDING_DURATION_SENTINEL: i32 = i32::MIN;

/// 全局唯一的闪电风暴状态（同时最多一场）。
#[derive(Debug, Clone, PartialEq, Eq)]
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
        Self {
            target_x,
            target_y,
            deferment_remaining: deferment.max(0),
            duration_remaining: duration,
        }
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
            } else {
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

fn begin_lightning_storm(world: &mut BattleState) {
    world.map.set_lighting_profile(LightingProfile::Ion);
}

fn end_lightning_storm(world: &mut BattleState) {
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

    let duration = world
        .lightning_storm
        .as_ref()
        .expect("风暴在延迟处理后仍应存在")
        .duration_remaining;
    if duration == ENDING_DURATION_SENTINEL {
        end_lightning_storm(world);
        return;
    }
    if duration == 0 {
        world
            .lightning_storm
            .as_mut()
            .expect("进入结束态时风暴仍应存在")
            .duration_remaining = ENDING_DURATION_SENTINEL;
        return;
    }
    if duration < 0 {
        // `-1` 无限持续：保持 Ion，不扣减。
        return;
    }

    let storm = world.lightning_storm.as_mut().expect("激活风暴仍应存在");
    storm.duration_remaining -= 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_adaptor::RulesSystem;
    use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
    use ra_map::{LightingConfig, LightingProfile};
    use ra_types::GameEdition;

    fn empty_rules() -> RulesSystem {
        RulesSystem {
            edition: GameEdition::Ra2,
            rules: IniDocument::default(),
            art: IniDocument::default(),
            overlay_types: OverlayTypeRegistry::default(),
            color_schemes: ColorSchemes::default(),
            countries: CountryRegistry::default(),
            techno_types: TechnoTypeRegistry::default(),
            warheads: WarheadRegistry::default(),
        }
    }

    fn world_with_ion_keys() -> BattleState {
        let bytes = b"[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n[Lighting]\nAmbient=1.0\nGround=0.0\nLevel=0.0\n\
IonAmbient=0.5\nIonRed=0.25\nIonGreen=0.25\nIonBlue=1.0\nIonGround=0.0\nIonLevel=0.0\n";
        let mut map = ra_map::MapInfo::parse_ini(GameEdition::Ra2, "storm.map", bytes).expect("map");
        // 无压暗便于断言。
        map.lighting = LightingConfig {
            ambient: 1.0,
            ground: 0.0,
            level: 0.0,
            ..LightingConfig::identity()
        };
        BattleState::new(GameEdition::Ra2, &empty_rules(), map)
    }

    #[test]
    fn deferment_then_duration_switches_ion_and_back() {
        let mut world = world_with_ion_keys();
        assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
        start_lightning_storm(&mut world, 5, 5, 1, 2);
        assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
        tick_lightning_storm(&mut world);
        assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
        let ion_tint = world.map.tint_at(0, 0, 0);
        tick_lightning_storm(&mut world); // duration 2→1
        assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
        tick_lightning_storm(&mut world); // duration 1→0
        assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
        tick_lightning_storm(&mut world); // → ending sentinel
        assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
        assert!(world.lightning_storm.is_some());
        tick_lightning_storm(&mut world); // cleanup
        assert!(world.lightning_storm.is_none());
        assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
        let normal_tint = world.map.tint_at(0, 0, 0);
        assert!(ion_tint[0] < normal_tint[0], "ion={ion_tint:?} normal={normal_tint:?}");
        assert!(ion_tint[2] > ion_tint[0]);
    }

    #[test]
    fn zero_deferment_starts_on_ion() {
        let mut world = world_with_ion_keys();
        start_lightning_storm(&mut world, 1, 1, 0, 1);
        assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    }
}
