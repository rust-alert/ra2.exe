//! 遭遇战 / 战役装载：规则 → `BattleState` → `BattleSession` → `Session`。

use std::sync::Arc;

use ra_adaptor::{ResourceChain, RulesSystem};
use ra_map::{MapInfo, MapEntityKind, apply_overlay_land_to_pass_grid, seal_pass_grid_from_tmp, skirmish_start_waypoint};
use ra_types::{AssetSource, RaResult};

use crate::{
    engine::{Engine, EngineConfig},
    game::{BattleSession, SessionBootKind},
    gameplay::starting_mcv_type_for_house,
    session::Session,
    state::BattleState,
};

/// `open_skirmish_session` / `open_campaign_session` 的成功结果。
#[derive(Debug)]
pub struct SkirmishOpenResult {
    /// 长期引擎（共享定义；本便利 API 每次新建一份默认定义）。
    pub engine: Engine,
    /// 已装载规则、地图与指纹的会话（内含一场 `BattleSession`）。
    pub session: Session,
    /// 追加了规则 / 世界统计后的 boot 注记。
    pub note: String,
}

/// 从已装载的 `RulesSystem` 与地图打开一局遭遇战会话。
///
/// - `preferred_house` 若给出，则登记到玩家表并设为本地玩家；登记后仍匹配失败则报错（禁止静默改用其它阵营）。
/// - `ensure_houses` 中的阵营一律登记进玩家表（遭遇战对手不一定出现在地图放置段）。
/// - 每个 `ensure_houses[slot]` 在地图航点 `slot` 放置该 house 的开局 MCV；航点缺失或格子非法时失败。
/// - 地图预放的机动单位（步兵 / 载具 / 飞行器）不进入仿真，仅保留建筑；避免无工厂时 AI 驱赶预放部队。
/// - `match_seed` 混入对局指纹，供后续确定性 RNG 使用。
pub fn open_skirmish_session(
    source: &dyn AssetSource,
    chain: &ResourceChain,
    rules: &RulesSystem,
    mut map: MapInfo,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
) -> RaResult<SkirmishOpenResult> {
    note = format!(
        "{note} · rules#{} · overlay_types#{} · techno_types#{} · seed={:#x}",
        rules.rules.sections.len(),
        rules.overlay_types.len(),
        rules.techno_types.len(),
        match_seed
    );

    let stripped = strip_skirmish_map_mobiles(&mut map);
    if stripped > 0 {
        note = format!("{note} · strip_mobiles#{stripped}");
    }

    open_session_common(
        source,
        chain,
        rules,
        map,
        note,
        preview_origin,
        preferred_house,
        ensure_houses,
        match_seed,
        SessionBootKind::Skirmish,
        true,
    )
}

/// 从已装载的 `RulesSystem` 与地图打开一局战役会话。
///
/// 与遭遇战的差异：
/// - **保留**地图预放步兵 / 载具 / 飞行器（不剥机动）。
/// - **不**按席位航点种开局 MCV。
/// - `BattleSession` 标记为 [`SessionBootKind::Campaign`]（胜负由触发器驱动，不用遭遇战 sole victor）。
pub fn open_campaign_session(
    source: &dyn AssetSource,
    chain: &ResourceChain,
    rules: &RulesSystem,
    map: MapInfo,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
) -> RaResult<SkirmishOpenResult> {
    note = format!(
        "{note} · campaign · rules#{} · overlay_types#{} · techno_types#{} · seed={:#x} · preplaced#{}",
        rules.rules.sections.len(),
        rules.overlay_types.len(),
        rules.techno_types.len(),
        match_seed,
        map.entities.len()
    );

    open_session_common(
        source,
        chain,
        rules,
        map,
        note,
        preview_origin,
        preferred_house,
        ensure_houses,
        match_seed,
        SessionBootKind::Campaign,
        false,
    )
}

fn open_session_common(
    source: &dyn AssetSource,
    chain: &ResourceChain,
    rules: &RulesSystem,
    map: MapInfo,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
    boot_kind: SessionBootKind,
    seed_skirmish_mcv: bool,
) -> RaResult<SkirmishOpenResult> {
    let mut state = BattleState::new(chain.edition, rules, map);
    for house in ensure_houses {
        if !house.is_empty() {
            state.ensure_house(house);
        }
    }
    if let Some(house) = preferred_house {
        state.ensure_house(house);
        if !state.prefer_local_house(house) {
            let available: Vec<&str> = state.players.iter().map(|p| p.house.as_ref()).collect();
            return Err(ra_types::RaError::Msg(format!("指定阵营不可用: {house}（地图玩家: {}）", available.join(", "))));
        }
        note = format!("{note} · local_house={house}");
    }
    let land_sealed = seal_pass_grid_from_tmp(source, &state.map, &mut state.pass_grid);
    let overlay_land = apply_overlay_land_to_pass_grid(
        &state.map,
        &rules.rules,
        &|id| rules.overlay_types.name(id).map(str::to_string),
        &mut state.pass_grid,
    );
    if land_sealed > 0 || overlay_land > 0 {
        state.repath_mobiles();
    }

    if seed_skirmish_mcv {
        let starts = seed_skirmish_starts_at_waypoints(&mut state, ensure_houses)?;
        if !starts.is_empty() {
            note = format!("{note} · starts=[{starts}]");
        }
    }

    note = format!(
        "{note} · world_entities#{} bound#{} blocked#{} land#{} overlay_land#{} defs_struct#{} deploy#{}",
        state.entities.len(),
        state.bound_techno_count(),
        state.pass_grid.blocked_count(),
        land_sealed,
        overlay_land,
        state.definitions.structures.len(),
        state.definitions.deployables.len()
    );

    let rules_bytes = source
        .read(chain.rules_ini)
        .map_err(|e| ra_types::RaError::Msg(format!("无法读取规则文件 {} 以生成对局指纹: {e}", chain.rules_ini)))?;
    if rules_bytes.is_empty() {
        return Err(ra_types::RaError::Msg(format!("规则文件 {} 为空，拒绝用空字节生成对局指纹", chain.rules_ini)));
    }
    let fingerprint = BattleSession::build_skirmish_fingerprint(
        chain.edition.as_str(),
        &state.map.name,
        &rules_bytes,
        state.map.width,
        state.map.height,
        state.entities.len(),
        match_seed,
    );

    let defs_for_engine = Arc::clone(&state.definitions);
    let mut game = match boot_kind {
        SessionBootKind::Skirmish => BattleSession::open_skirmish(state, note.clone(), preview_origin, fingerprint),
        SessionBootKind::Campaign => BattleSession::open_campaign(state, note.clone(), preview_origin, fingerprint),
    };
    game.set_match_seed(match_seed);
    let engine = Engine::new(defs_for_engine, EngineConfig::default()).map_err(|e| ra_types::RaError::Msg(e.to_string()))?;
    let mut session = engine.create_session(crate::session::SessionSpec::default()).map_err(|e| ra_types::RaError::Msg(e.to_string()))?;
    session.attach_battle(game);

    Ok(SkirmishOpenResult { engine, session, note })
}

/// 遭遇战不把地图预放机动单位纳入权威世界（预览底图仍可保留叠画）。
fn strip_skirmish_map_mobiles(map: &mut MapInfo) -> usize {
    let before = map.entities.len();
    map.entities.retain(|e| e.kind == MapEntityKind::Structure);
    before.saturating_sub(map.entities.len())
}

/// 按大厅席位顺序，在地图航点放置各 house 的开局 MCV。
fn seed_skirmish_starts_at_waypoints(state: &mut BattleState, houses: &[&str]) -> RaResult<String> {
    let mut parts = Vec::new();
    for (slot, house) in houses.iter().enumerate() {
        if house.is_empty() {
            continue;
        }
        let slot = slot as u32;
        let Some(wp) = skirmish_start_waypoint(&state.map.waypoints, slot)
        else {
            return Err(ra_types::RaError::Msg(format!("开局席位 {slot} 缺少地图航点（house={house}）")));
        };
        let Some(mcv) = starting_mcv_type_for_house(&state.definitions, house).map(str::to_owned)
        else {
            return Err(ra_types::RaError::Msg(format!(
                "阵营 {house} 无可用开局 MCV（需 Vehicle 且 DeploysInto 建造场）"
            )));
        };
        let id = state.spawn_unit_at(house, &mcv, wp.x, wp.y).map_err(ra_types::RaError::Msg)?;
        parts.push(format!("{house}@{slot}:({},{})={mcv}#{:?}", wp.x, wp.y, id));
    }
    Ok(parts.join(" "))
}
