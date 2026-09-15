//! 遭遇战地图包装载／准备／能力缺口分析（供 `ra2 analyze maps`）。
//!
//! 不打开 GUI：枚举 `missions.pkt`（空则回退扫描），对每张图做解析、剧本缺口与
//! [`ra_engine::validate_map_for_battle`]。结果供验收清单三态统计，禁止把「能列出来」写成可玩。

use std::path::PathBuf;

use ra_adaptor::{build_runtime_definitions, detect_edition, load_rules_chain};
use ra_engine::validate_map_for_battle;
use ra_map::{
    BootMapCandidate, MapInfo, is_campaign_blocking_action_gap, list_parseable_maps_from_missions_pkt, list_parseable_maps_from_names,
    map_scripting_capability_gaps, mount_theater_mixes,
};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};
use ra_widgets::fs_source::GameAssetSource;

/// 诊断请求。
#[derive(Debug, Clone)]
pub struct DiagnoseMapsRequest {
    /// 游戏安装根目录。
    pub ra2_dir: PathBuf,
    /// 可选版本；`None` 则自动探测（合集盘仍应显式 `ra2`）。
    pub edition: Option<String>,
    /// 最多诊断前 N 张；`None` 表示全表。
    pub limit: Option<usize>,
}

/// 单张地图诊断行。
#[derive(Debug, Clone)]
pub struct MapDiagnoseRow {
    /// 地图文件名（如 `mp02t2.map`）。
    pub file_name: String,
    /// CSF 显示名键（可能为空）。
    pub name_csf: String,
    /// 是否成功解析为 [`MapInfo`]。
    pub parse_ok: bool,
    /// 解析失败说明。
    pub parse_error: Option<String>,
    /// `validate_map_for_battle` 是否通过。
    pub prepare_ok: bool,
    /// 准备失败说明。
    pub prepare_error: Option<String>,
    /// 战役硬拒类缺口码（`map.action.*` / `map.event.*` unsupported）。
    pub blocking_gaps: Vec<String>,
    /// 呈现占位 stub 码。
    pub stub_gaps: Vec<String>,
    /// 已装载未接线 deferred 码。
    pub deferred_gaps: Vec<String>,
    /// 其它缺口码（未知节等）。
    pub other_gaps: Vec<String>,
    /// 三态建议：`success` / `reject` / `missing`（见 [`classify_skirmish_tri_state`]）。
    pub tri_state: String,
}

/// 整次诊断报告。
#[derive(Debug, Clone)]
pub struct DiagnoseMapsReport {
    /// 探测版本。
    pub edition: String,
    /// 地图来源说明（`missions.pkt` 或 `scan`）。
    pub source: String,
    /// 候选总数（应用 limit 前）。
    pub candidate_count: usize,
    /// 规则层能力缺口（全表共用，含「超武有定义无执行器」等）。
    pub rules_capability_gaps: Vec<String>,
    /// 诊断行。
    pub maps: Vec<MapDiagnoseRow>,
    /// 三态计数。
    pub success: usize,
    pub reject: usize,
    pub missing: usize,
}

/// 按遭遇战装载口径建议三态（非整局可玩验收）。
///
/// - 解析或准备失败 → `reject`（明确拒绝）
/// - 通过但有硬拒类剧本缺口 → `missing`（功能缺失风险；遭遇战 boot 当前不硬拒动作，但仍记入）
/// - 通过且无硬拒缺口 → `success`（仅表示装载／准备成功）
pub fn classify_skirmish_tri_state(parse_ok: bool, prepare_ok: bool, blocking_gaps: &[String]) -> &'static str {
    if !parse_ok || !prepare_ok {
        return "reject";
    }
    if !blocking_gaps.is_empty() {
        return "missing";
    }
    "success"
}

/// 枚举并诊断遭遇战地图包。
pub fn diagnose_skirmish_maps(req: &DiagnoseMapsRequest) -> RaResult<DiagnoseMapsReport> {
    if req.ra2_dir.as_os_str().is_empty() {
        return Err(RaError::Msg("analyze maps: --path must not be empty".into()));
    }
    let explicit = match req.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&req.ra2_dir, explicit)?;
    let chain = &manifest.chain;
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    let (candidates, list_source) = if let Ok(pkt) = source.read(manifest.chain.missions_pkt) {
        let from_pkt = list_parseable_maps_from_missions_pkt(chain.edition, &source, &pkt);
        if !from_pkt.is_empty() {
            (from_pkt, manifest.chain.missions_pkt.to_string())
        }
        else {
            let names = source.discover_skirmish_map_names();
            (list_parseable_maps_from_names(chain.edition, &source, names), "scan".into())
        }
    }
    else {
        let names = source.discover_skirmish_map_names();
        (list_parseable_maps_from_names(chain.edition, &source, names), "scan".into())
    };

    let candidate_count = candidates.len();
    let take_n = req.limit.unwrap_or(candidate_count).min(candidate_count);

    let rules = load_rules_chain(&source, chain)?;
    let definitions = build_runtime_definitions(&rules)?;
    let rules_capability_gaps: Vec<String> = definitions.capability_gaps.iter().map(|g| g.code.clone()).collect();
    for gap in &definitions.capability_gaps {
        tracing::warn!("规则能力缺口 [{}] {}", gap.code, gap.message);
    }

    let mut maps = Vec::with_capacity(take_n);
    let mut success = 0usize;
    let mut reject = 0usize;
    let mut missing = 0usize;

    for cand in candidates.into_iter().take(take_n) {
        let row = diagnose_one(&mut source, chain.edition, &definitions, cand);
        match row.tri_state.as_str() {
            "success" => success += 1,
            "reject" => reject += 1,
            _ => missing += 1,
        }
        maps.push(row);
    }

    Ok(DiagnoseMapsReport {
        edition: chain.edition.as_str().to_string(),
        source: list_source,
        candidate_count,
        rules_capability_gaps,
        maps,
        success,
        reject,
        missing,
    })
}

fn diagnose_one(
    source: &mut GameAssetSource,
    edition: GameEdition,
    definitions: &ra_types::RuntimeDefinitions,
    cand: BootMapCandidate,
) -> MapDiagnoseRow {
    let file_name = cand.file_name;
    let name_csf = cand.name_csf;
    let bytes = match source.read(&file_name) {
        Ok(b) => b,
        Err(e) => {
            return MapDiagnoseRow {
                file_name,
                name_csf,
                parse_ok: false,
                parse_error: Some(format!("{e}")),
                prepare_ok: false,
                prepare_error: None,
                blocking_gaps: Vec::new(),
                stub_gaps: Vec::new(),
                deferred_gaps: Vec::new(),
                other_gaps: Vec::new(),
                tri_state: "reject".into(),
            };
        }
    };

    let map = match MapInfo::parse_ini(edition, &file_name, &bytes) {
        Ok(m) => m,
        Err(e) => {
            return MapDiagnoseRow {
                file_name,
                name_csf,
                parse_ok: false,
                parse_error: Some(format!("{e}")),
                prepare_ok: false,
                prepare_error: None,
                blocking_gaps: Vec::new(),
                stub_gaps: Vec::new(),
                deferred_gaps: Vec::new(),
                other_gaps: Vec::new(),
                tri_state: "reject".into(),
            };
        }
    };

    let _ = mount_theater_mixes(map.theater, &mut |mix| matches!(source.vfs.mount_nested_all_from_parents(mix), Ok(n) if n > 0));

    let mut blocking_gaps = Vec::new();
    let mut stub_gaps = Vec::new();
    let mut deferred_gaps = Vec::new();
    let mut other_gaps = Vec::new();
    for gap in map_scripting_capability_gaps(&map) {
        if is_campaign_blocking_action_gap(&gap.code) {
            blocking_gaps.push(gap.code);
        }
        else if gap.code.ends_with(" stub") {
            stub_gaps.push(gap.code);
        }
        else if gap.code.ends_with(" deferred") {
            deferred_gaps.push(gap.code);
        }
        else {
            other_gaps.push(gap.code);
        }
    }

    let (prepare_ok, prepare_error) = match validate_map_for_battle(&map, definitions) {
        Ok(_) => (true, None),
        Err(e) => (false, Some(format!("{e}"))),
    };

    let tri_state = classify_skirmish_tri_state(true, prepare_ok, &blocking_gaps).to_string();
    MapDiagnoseRow {
        file_name,
        name_csf,
        parse_ok: true,
        parse_error: None,
        prepare_ok,
        prepare_error,
        blocking_gaps,
        stub_gaps,
        deferred_gaps,
        other_gaps,
        tri_state,
    }
}

#[cfg(test)]
mod tests {
    use super::classify_skirmish_tri_state;

    #[test]
    fn tri_state_reject_on_parse_or_prepare_failure() {
        assert_eq!(classify_skirmish_tri_state(false, false, &[]), "reject");
        assert_eq!(classify_skirmish_tri_state(true, false, &[]), "reject");
    }

    #[test]
    fn tri_state_missing_when_blocking_gaps_present() {
        let gaps = vec!["map.event.2 unsupported".into()];
        assert_eq!(classify_skirmish_tri_state(true, true, &gaps), "missing");
    }

    #[test]
    fn tri_state_success_when_prepare_clean() {
        assert_eq!(classify_skirmish_tri_state(true, true, &[]), "success");
    }
}
