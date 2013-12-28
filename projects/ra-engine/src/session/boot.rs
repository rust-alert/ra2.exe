//! 遭遇战装载：规则 → MatchState → Game → Session。

use std::sync::Arc;

use ra_adaptor::{ResourceChain, RulesDb};
use ra_map::{MapInfo, seal_pass_grid_from_tmp};
use ra_types::{AssetSource, RaResult};

use crate::engine::{Engine, EngineConfig};
use crate::game::Game;
use crate::session::Session;
use crate::state::MatchState;

/// `open_skirmish_session` 的成功结果。
#[derive(Debug)]
pub struct SkirmishOpenResult {
    /// 长期引擎（共享定义；本便利 API 每次新建一份默认定义）。
    pub engine: Engine,
    /// 已装载规则、地图与指纹的遭遇战会话（内含一局 Game）。
    pub session: Session,
    /// 追加了规则 / 世界统计后的 boot 注记。
    pub note: String,
}

/// 从已装载的 `RulesDb` 与地图打开一局遭遇战会话。
///
/// `preferred_house` 若能在世界玩家表中匹配，则设为本地玩家。
pub fn open_skirmish_session(
    source: &dyn AssetSource,
    chain: &ResourceChain,
    rules: &RulesDb,
    map: MapInfo,
    mut note: String,
    preview_origin: (i32, i32),
    preferred_house: Option<&str>,
) -> RaResult<SkirmishOpenResult> {
    note = format!(
        "{note} · rules#{} · overlay_types#{} · techno_types#{}",
        rules.rules.sections.len(),
        rules.overlay_types.len(),
        rules.techno_types.len()
    );

    let mut state = MatchState::new(chain.edition, rules, map);
    if let Some(house) = preferred_house {
        if state.prefer_local_house(house) {
            note = format!("{note} · local_house={house}");
        }
        else {
            note = format!("{note} · local_house=(fallback) want={house}");
        }
    }
    let land_sealed = seal_pass_grid_from_tmp(source, &state.map, &mut state.pass_grid);
    if land_sealed > 0 {
        state.repath_mobiles();
    }
    note = format!(
        "{note} · world_entities#{} bound#{} blocked#{} land#{} defs_struct#{} deploy#{}",
        state.entities.len(),
        state.bound_techno_count(),
        state.pass_grid.blocked_count(),
        land_sealed,
        state.definitions.structures.len(),
        state.definitions.deployables.len()
    );

    let rules_bytes = source.read(chain.rules_ini).unwrap_or_default();
    let fingerprint = Game::build_skirmish_fingerprint(
        chain.edition.as_str(),
        &state.map.name,
        &rules_bytes,
        state.map.width,
        state.map.height,
        state.entities.len(),
    );

    let defs_for_engine = Arc::clone(&state.definitions);
    let game = Game::open_skirmish(state, note.clone(), preview_origin, fingerprint);
    let engine = Engine::new(defs_for_engine, EngineConfig::default())
        .map_err(|e| ra_types::RaError::Msg(e.to_string()))?;
    let mut session = engine
        .create_session(crate::session::SessionSpec::default())
        .map_err(|e| ra_types::RaError::Msg(e.to_string()))?;
    session.attach_game(game);

    Ok(SkirmishOpenResult { engine, session, note })
}
