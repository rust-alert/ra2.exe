//! 遭遇战 / 战役装载：冻结定义 → `BattleState` → `BattleSession` → `Session`。

use std::sync::Arc;

use ra_map::{MapEntityKind, MapInfo};
use ra_types::{AssetSource, GameEdition, PreparedMap, RaResult, RuntimeDefinitions};

use crate::{
    engine::{Engine, EngineConfig},
    game::{BattleSession, SessionBootKind},
    gameplay::{
        compose_starting_unit_ids, format_type_keys, starting_deploy_clearance, starting_mcv_id_for_house, starting_unit_pools,
    },
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

/// 用与 [`BattleState::new`] 相同的 [`MapInfo::to_prepared_map`] 路径准备并校验地图引用。
///
/// 产品 boot 应在预览 / 打开会话之前调用，使非法 techno / house / 脚本引用，以及放置
/// 越界 / 结构足迹重叠，在装载前半段失败，而不是先画出预览再在开会话时才拒绝。
///
/// 成功时返回可交给 [`BattleState::from_prepared`] / [`open_campaign_session_prepared`] 的
/// [`PreparedMap`]，避免战役路径二次绑定。
pub fn validate_map_for_battle(map: &MapInfo, definitions: &RuntimeDefinitions) -> RaResult<PreparedMap> {
    map.to_prepared_map(definitions)
}

/// 从冻结定义与地图打开一局遭遇战会话。
///
/// - `preferred_house` 若给出，则登记到玩家表并设为本地玩家；登记后仍匹配失败则报错（禁止静默改用其它阵营）。
/// - `ensure_houses` 中的阵营一律登记进玩家表（遭遇战对手不一定出现在地图放置段）。
/// - 每个 `ensure_houses[slot]` 在地图航点 `slot` 放置该 house 的开局 MCV；航点缺失或格子非法时失败。
/// - 地图预放的机动单位（步兵 / 载具 / 飞行器）不进入仿真，仅保留建筑；避免无工厂时 AI 驱赶预放部队。
/// - `match_seed` 混入对局指纹，供后续确定性 RNG 使用。
/// - `rules_ini` 为资源链中规则文件逻辑路径，仅用于读取对局指纹字节。
pub fn open_skirmish_session(
    source: &dyn AssetSource,
    edition: GameEdition,
    rules_ini: &str,
    definitions: Arc<RuntimeDefinitions>,
    mut map: MapInfo,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
) -> RaResult<SkirmishOpenResult> {
    let stripped = strip_skirmish_map_mobiles(&mut map);
    if stripped > 0 {
        note = format!("{note} · strip_mobiles#{stripped}");
    }
    // 剥机动后实体集已变，必须按剥后地图重新准备（不可复用预览前的全图 `PreparedMap`）。
    let prepared = validate_map_for_battle(&map, definitions.as_ref())?;
    open_skirmish_session_prepared(
        source,
        edition,
        rules_ini,
        definitions,
        map,
        prepared,
        note,
        preview_origin,
        preferred_house,
        ensure_houses,
        match_seed,
    )
}

/// 遭遇战开局：复用已按**剥机动后**地图通过 [`validate_map_for_battle`] 的 [`PreparedMap`]。
///
/// 调用方须先 [`strip_skirmish_map_mobiles`]（或保证 `map.entities` 仅含建筑），且 `prepared` 与该地图一致。
pub fn open_skirmish_session_prepared(
    source: &dyn AssetSource,
    edition: GameEdition,
    rules_ini: &str,
    definitions: Arc<RuntimeDefinitions>,
    map: MapInfo,
    prepared: PreparedMap,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
) -> RaResult<SkirmishOpenResult> {
    note = format!(
        "{note} · overlays#{} · techno#{} · seed={:#x} · prepared#{}",
        definitions.overlays.len(),
        definitions.techno.len(),
        match_seed,
        prepared.placements.len()
    );

    open_session_common(
        source,
        edition,
        rules_ini,
        definitions,
        map,
        prepared,
        note,
        preview_origin,
        preferred_house,
        ensure_houses,
        match_seed,
        SessionBootKind::Skirmish,
        true,
    )
}

/// 从冻结定义与地图打开一局战役会话。
///
/// 与遭遇战的差异：
/// - **保留**地图预放步兵 / 载具 / 飞行器（不剥机动）。
/// - **不**按席位航点种开局 MCV。
/// - `BattleSession` 标记为 [`SessionBootKind::Campaign`]（胜负由触发器驱动，不用遭遇战 sole victor）。
pub fn open_campaign_session(
    source: &dyn AssetSource,
    edition: GameEdition,
    rules_ini: &str,
    definitions: Arc<RuntimeDefinitions>,
    map: MapInfo,
    note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
) -> RaResult<SkirmishOpenResult> {
    let prepared = validate_map_for_battle(&map, definitions.as_ref())?;
    open_campaign_session_prepared(
        source,
        edition,
        rules_ini,
        definitions,
        map,
        prepared,
        note,
        preview_origin,
        preferred_house,
        ensure_houses,
        match_seed,
    )
}

/// 战役开局：复用已通过 [`validate_map_for_battle`] 的 [`PreparedMap`]，避免二次绑定。
pub fn open_campaign_session_prepared(
    source: &dyn AssetSource,
    edition: GameEdition,
    rules_ini: &str,
    definitions: Arc<RuntimeDefinitions>,
    map: MapInfo,
    prepared: PreparedMap,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
) -> RaResult<SkirmishOpenResult> {
    note = format!(
        "{note} · campaign · overlays#{} · techno#{} · seed={:#x} · preplaced#{} · prepared#{}",
        definitions.overlays.len(),
        definitions.techno.len(),
        match_seed,
        map.entities.len(),
        prepared.placements.len()
    );

    open_session_common(
        source,
        edition,
        rules_ini,
        definitions,
        map,
        prepared,
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
    edition: GameEdition,
    rules_ini: &str,
    definitions: Arc<RuntimeDefinitions>,
    map: MapInfo,
    prepared: PreparedMap,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
    ensure_houses: &[&str],
    match_seed: u64,
    boot_kind: SessionBootKind,
    seed_skirmish_mcv: bool,
) -> RaResult<SkirmishOpenResult> {
    let mut state = BattleState::from_prepared(edition, definitions, map, prepared)?;
    note = format!("{note} · placements#{}", state.prepared.placements.len());
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
    if boot_kind == SessionBootKind::Campaign {
        let applied = apply_campaign_map_houses(&mut state);
        if applied > 0 {
            note = format!("{note} · map_houses#{applied}");
        }
        if apply_basic_starting_credits(&mut state) {
            note = format!("{note} · starting_credits={}", state.map.starting_credits);
        }
    }
    // 装载序：Foundation 种子（`from_prepared`）→ `finalize_pass_from_assets`（TMP → overlay land → 回写 prepared）。
    let (land_sealed, overlay_land) = state.finalize_pass_from_assets(source);

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

    let rules_bytes =
        source.read(rules_ini).map_err(|e| ra_types::RaError::Msg(format!("无法读取规则文件 {rules_ini} 以生成对局指纹: {e}")))?;
    if rules_bytes.is_empty() {
        return Err(ra_types::RaError::Msg(format!("规则文件 {rules_ini} 为空，拒绝用空字节生成对局指纹")));
    }
    let fingerprint = BattleSession::build_skirmish_fingerprint(
        edition.as_str(),
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
///
/// 返回被剥离的实体数。产品 boot 在剥后应再 [`validate_map_for_battle`]，再开
/// [`open_skirmish_session_prepared`]。
pub fn strip_skirmish_map_mobiles(map: &mut MapInfo) -> usize {
    let before = map.entities.len();
    map.entities.retain(|e| e.kind == MapEntityKind::Structure);
    before.saturating_sub(map.entities.len())
}

/// 战役：登记地图 `[Houses]`，并把 `Credits`（百计）、`TechLevel` 与 `Allies` 写入对应 house。
///
/// 优先消费 [`BattleState::prepared`] 中已绑定的 [`ra_types::PreparedHouse`]（稳定 `HouseId`）。
/// house 键用 `Country=` 对应规则房屋名，同时登记节名（部分地图放置 owner 用节名）。
fn apply_campaign_map_houses(state: &mut BattleState) -> usize {
    let houses = state.prepared.houses.clone();
    if houses.is_empty() {
        return 0;
    }
    let mut applied = 0usize;
    for h in &houses {
        let Some(primary) = state.definitions.houses.get_by_id(h.country).map(|c| c.type_key.as_str().to_string())
        else {
            continue;
        };
        let section = h.name.trim().to_string();
        state.ensure_house(&primary);
        if !section.is_empty() && !section.eq_ignore_ascii_case(&primary) {
            state.ensure_house(&section);
        }
        if h.credits > 0 {
            let funds = h.credits.saturating_mul(100);
            let _ = state.set_house_funds(&primary, funds);
            if !section.is_empty() && !section.eq_ignore_ascii_case(&primary) {
                let _ = state.set_house_funds(&section, funds);
            }
        }
        if h.tech_level > 0 {
            let _ = state.set_house_tech_level(&primary, h.tech_level);
            if !section.is_empty() && !section.eq_ignore_ascii_case(&primary) {
                let _ = state.set_house_tech_level(&section, h.tech_level);
            }
        }
        if !h.allies.is_empty() {
            let allies: Vec<String> =
                h.allies.iter().filter_map(|id| state.definitions.houses.get_by_id(*id).map(|d| d.type_key.as_str().to_string())).collect();
            let _ = state.set_house_allies(&primary, allies.clone());
            if !section.is_empty() && !section.eq_ignore_ascii_case(&primary) {
                let _ = state.set_house_allies(&section, allies);
            }
        }
        if h.player_control {
            let _ = state.prefer_local_house(&primary);
        }
        applied = applied.saturating_add(1);
    }
    applied
}

/// 战役：对仍为 0 资金的 house 套用 `[Basic] StartingCredits`（不覆盖 Houses `Credits`）。
fn apply_basic_starting_credits(state: &mut BattleState) -> bool {
    let credits = state.map.starting_credits;
    if credits <= 0 {
        return false;
    }
    let needy: Vec<std::sync::Arc<str>> = state.players.iter().filter(|p| p.funds <= 0).map(|p| p.house.clone()).collect();
    if needy.is_empty() {
        return false;
    }
    for house in needy {
        let _ = state.set_house_funds(house.as_ref(), credits);
    }
    true
}

/// 按大厅席位顺序，在地图航点放置各 house 的开局 MCV。
fn seed_skirmish_starts_at_waypoints(state: &mut BattleState, houses: &[&str]) -> RaResult<String> {
    let mut parts = Vec::new();
    for (slot, house) in houses.iter().enumerate() {
        if house.is_empty() {
            continue;
        }
        let slot = slot as u32;
        let Some((x, y)) = state.prepared.definition.waypoints.iter().find(|w| w.index == slot).map(|w| (w.x, w.y))
        else {
            return Err(ra_types::RaError::Msg(format!("开局席位 {slot} 缺少地图航点（house={house}）")));
        };
        let Some(mcv_id) = starting_mcv_id_for_house(&state.definitions, house)
        else {
            return Err(ra_types::RaError::Msg(format!("阵营 {house} 无可用开局 MCV（需 Vehicle 且 DeploysInto 建造场）")));
        };
        let Some(house_id) = crate::gameplay::house_id_of(&state.definitions, house)
        else {
            return Err(ra_types::RaError::Msg(format!("未知开局阵营: {house}")));
        };
        let mcv_key = crate::gameplay::type_key_of(&state.definitions, mcv_id).to_string();
        let id = state.spawn_unit_at_ids(house_id, mcv_id, x, y).map_err(ra_types::RaError::Msg)?;
        parts.push(format!("{house}@{slot}:({x},{y})={mcv_key}#{id:?}"));
    }
    Ok(parts.join(" "))
}

/// 遭遇战大厅 `Unit Count`：在各席位航点周围按阵营种植开局部队（不含 MCV）。
///
/// - 候选：地面步兵 / 载具，且 `AllowedToStartInMultiplayer`（缺省是）、Owner / 科技允许，排除 BaseUnit。
/// - 交替取最便宜步兵与最便宜载具，占位在 MCV 展开 Foundation **之外**的扩环空格。
/// - 应在写入大厅 `tech_level` 之后调用，以便科技上限生效。
pub fn seed_skirmish_starting_units(state: &mut BattleState, houses: &[&str], unit_count: i32) -> RaResult<String> {
    let unit_count = unit_count.clamp(0, 20);
    if unit_count <= 0 {
        return Ok(String::new());
    }
    let mut parts = Vec::new();
    for (slot, house) in houses.iter().enumerate() {
        if house.is_empty() {
            continue;
        }
        let slot = slot as u32;
        let Some((ox, oy)) = state.prepared.definition.waypoints.iter().find(|w| w.index == slot).map(|w| (w.x, w.y))
        else {
            return Err(ra_types::RaError::Msg(format!("开局部队席位 {slot} 缺少地图航点（house={house}）")));
        };
        let Some(house_id) = crate::gameplay::house_id_of(&state.definitions, house)
        else {
            return Err(ra_types::RaError::Msg(format!("未知开局部队阵营: {house}")));
        };
        let tech_level = state
            .players
            .iter()
            .find(|p| p.house.eq_ignore_ascii_case(house))
            .map(|p| p.tech_level)
            .unwrap_or(state.definitions.default_tech_level);
        let (infantry, vehicles) = starting_unit_pools(&state.definitions, house, tech_level);
        if infantry.is_empty() && vehicles.is_empty() {
            return Err(ra_types::RaError::Msg(format!(
                "阵营 {house} 无可用开局部队（需至少一种 AllowedToStartInMultiplayer 地面步兵或载具）"
            )));
        }
        if infantry.is_empty() || vehicles.is_empty() {
            return Err(ra_types::RaError::Msg(format!(
                "阵营 {house} 开局部队不完整（须同时具备 AllowedToStartInMultiplayer 步兵与载具）"
            )));
        }
        let ids = compose_starting_unit_ids(&infantry, &vehicles, unit_count);
        let plan = format_type_keys(&state.definitions, &ids);
        let (clear_w, clear_h) = starting_deploy_clearance(&state.definitions, house);
        let mut blocked: Vec<(u16, u16)> = Vec::new();
        let mut placed = 0usize;
        for type_id in &ids {
            let Some((x, y)) = find_starting_unit_cell(state, ox, oy, clear_w, clear_h, &blocked)
            else {
                break;
            };
            match state.spawn_unit_at_ids(house_id, *type_id, x, y) {
                Ok(_) => placed = placed.saturating_add(1),
                Err(_) => {
                    // 与 `can_place` 不一致时跳过该格，避免死循环同一格。
                    blocked.push((x, y));
                }
            }
        }
        parts.push(format!("{house}@{slot}:n{placed}/{}[{plan}]", unit_count));
    }
    Ok(parts.join(" "))
}

/// 航点外扩环搜可放格：跳过 MCV 展开 Foundation 矩形，并避开 `blocked`。
fn find_starting_unit_cell(
    state: &BattleState,
    ox: u16,
    oy: u16,
    clear_w: u16,
    clear_h: u16,
    blocked: &[(u16, u16)],
) -> Option<(u16, u16)> {
    let clear_w = clear_w.max(1);
    let clear_h = clear_h.max(1);
    let min_r = i32::from(clear_w.max(clear_h));
    let max_r = min_r.saturating_add(24);
    for r in min_r..=max_r {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let x = i32::from(ox) + dx;
                let y = i32::from(oy) + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let (x, y) = (x as u16, y as u16);
                if x >= ox && y >= oy && x < ox.saturating_add(clear_w) && y < oy.saturating_add(clear_h) {
                    continue;
                }
                if blocked.iter().any(|&(bx, by)| bx == x && by == y) {
                    continue;
                }
                if state.can_place_structure(x, y) {
                    return Some((x, y));
                }
            }
        }
    }
    None
}
