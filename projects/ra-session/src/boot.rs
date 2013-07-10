//! 遭遇战装载：规则 → 世界 → 指纹 → 会话。

use ra_adaptor::{ResourceChain, RulesDb};
use ra_map::{MapInfo, seal_pass_grid_from_tmp};
use ra_types::{AssetSource, RaResult};
use ra_world::World;

use crate::Session;

/// `open_skirmish_session` 的成功结果。
#[derive(Debug)]
pub struct SkirmishOpenResult {
    /// 已装载规则、地图与指纹的遭遇战会话。
    pub session: Session,
    /// 追加了规则 / 世界统计后的 boot 注记。
    pub note: String,
}

/// 从已装载的 `RulesDb` 与地图打开一局遭遇战会话。
pub fn open_skirmish_session(
    source: &dyn AssetSource,
    chain: &ResourceChain,
    rules: &RulesDb,
    map: MapInfo,
    mut note: String,
    preview_origin: (i32, i32),
) -> RaResult<SkirmishOpenResult> {
    note = format!(
        "{note} · rules#{} · overlay_types#{} · techno_types#{}",
        rules.rules.sections.len(),
        rules.overlay_types.len(),
        rules.techno_types.len()
    );

    let mut world = World::new(chain.edition, rules, map);
    let land_sealed = seal_pass_grid_from_tmp(source, &world.map, &mut world.pass_grid);
    if land_sealed > 0 {
        world.repath_mobiles();
    }
    note = format!(
        "{note} · world_entities#{} bound#{} blocked#{} land#{}",
        world.entities.len(),
        world.bound_techno_count(),
        world.pass_grid.blocked_count(),
        land_sealed
    );

    let rules_bytes = source.read(chain.rules_ini).unwrap_or_default();
    let fingerprint = Session::build_skirmish_fingerprint(
        chain.edition.as_str(),
        &world.map.name,
        &rules_bytes,
        world.map.width,
        world.map.height,
        world.entities.len(),
    );

    let session = Session::open_skirmish(world, note.clone(), preview_origin, fingerprint);
    Ok(SkirmishOpenResult { session, note })
}
