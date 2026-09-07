//! 安装探测、资源挂载与遭遇战会话打开（对局前装载，不属于 `MatchController`）。

use ra_adaptor::{ResourceChain, RulesDb, detect_edition, load_rules_chain};
use ra_engine::{Engine, Session, open_skirmish_session};
use ra_map::{MapInfo, compose_boot_preview, find_first_boot_map, mount_theater_mixes};
use ra_renderer::RgbaImage;
use ra_types::{GameEdition, RaResult};

use crate::{
    config::{DesktopConfig, load_desktop_config_with_diagnostics},
    fs_source::GameAssetSource,
};

/// 一次装载尝试的结果（成功或带说明的失败）。
#[derive(Debug)]
pub struct BootResult {
    /// 人类可读装载备注。
    pub note: String,
    /// 长期引擎（与会话共享定义生命周期）。
    pub engine: Option<Engine>,
    /// 已打开的会话（若装载成功）。
    pub session: Option<Session>,
    /// 可选地形预览图。
    pub preview: Option<RgbaImage>,
}

fn load_map_terrain_preview(
    source: &GameAssetSource,
    map: &MapInfo,
    chain: &ResourceChain,
    rules: &RulesDb,
) -> Option<(String, RgbaImage, i32, i32)> {
    let preview = compose_boot_preview(
        source,
        map,
        chain.art_ini,
        &|id| rules.overlay_types.name(id).map(str::to_owned),
        &|base, owner| rules.color_schemes.palette_for_house(&rules.rules, base, owner),
    )?;
    let rgba = RgbaImage::new(preview.image.width, preview.image.height, preview.image.pixels)?;
    Some((preview.note, rgba, preview.origin_x, preview.origin_y))
}

fn load_boot_map(source: &mut GameAssetSource, edition: GameEdition, note: &mut String) -> MapInfo {
    let loaded = find_first_boot_map(edition, source);
    let theater_mounted =
        mount_theater_mixes(loaded.map.theater, &mut |mix| matches!(source.vfs.mount_nested(mix), Ok(true)));
    *note = format!("{note} · {} · 剧院mix {}", loaded.note, theater_mounted);
    loaded.map
}

/// 按桌面配置探测安装并打开一局遭遇战会话。
pub fn boot_world(cfg: &DesktopConfig) -> RaResult<BootResult> {
    let root = cfg.ra2_dir.clone();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&root, explicit)?;
    for report in &manifest.stack.unsupported {
        tracing::warn!("适配能力缺口 [{}] {}", report.code, report.message);
    }
    if !manifest.stack.extensions.is_empty() {
        let ids: Vec<_> = manifest.stack.extensions.iter().map(|e| e.as_str()).collect();
        tracing::info!("适配扩展探测: {}", ids.join("+"));
    }
    let chain = &manifest.chain;

    for line in manifest.composition.diagnostics.summary_lines() {
        tracing::info!("资源组合 {line}");
    }

    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, skipped_root) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    for line in manifest.composition.mount_plan_lines() {
        tracing::info!("资源组合 {line}");
    }
    let mounted_nested = source.mount_nested_names(chain.nested_mix_files);

    if let Some((archive, _)) = source.vfs.resolve(chain.rules_ini) {
        tracing::info!("资源组合 resolved: rules={archive}:{}", chain.rules_ini);
    }
    else {
        tracing::warn!("资源组合 resolved: rules=(missing) {}", chain.rules_ini);
    }

    let expand_n = manifest.composition.diagnostics.detected_expansions.len();
    let mut note = format!(
        "{} · 根mix {} · 嵌套 {} · 跳过 {} · 缺盘 {} · expand#{}",
        chain.edition.as_str(),
        mounted_root,
        mounted_nested,
        skipped_root,
        manifest.missing_mixes.len(),
        expand_n
    );

    let map = load_boot_map(&mut source, chain.edition, &mut note);

    let mut preview_origin = (0i32, 0i32);
    let rules = match load_rules_chain(&source, chain) {
        Ok(db) => Some(db),
        Err(e) => {
            note = format!("{note} · 规则待加载（{e}）");
            None
        }
    };

    let preview = match rules.as_ref().and_then(|rules| load_map_terrain_preview(&source, &map, chain, rules)) {
        Some((name, image, ox, oy)) => {
            note = format!("{note} · preview:{name}");
            preview_origin = (ox, oy);
            Some(image)
        }
        None => {
            note = format!("{note} · preview:无");
            None
        }
    };

    let (engine, session) =
        match rules.as_ref().map(|rules| open_skirmish_session(&source, chain, rules, map, note.clone(), preview_origin)) {
            Some(Ok(opened)) => {
                note = opened.note;
                tracing::info!(
                    "fingerprint edition={} map={} rules_hash={:#x}",
                    opened.session.expect_game().fingerprint.edition,
                    opened.session.expect_game().fingerprint.map,
                    opened.session.expect_game().fingerprint.rules_hash
                );
                (Some(opened.engine), Some(opened.session))
            }
            Some(Err(e)) => {
                note = format!("{note} · 会话未打开（{e}）");
                (None, None)
            }
            None => (None, None),
        };

    Ok(BootResult { note, engine, session, preview })
}

/// 读取 `config.toml` 并尝试装载（失败时仍返回带 note 的 `BootResult`）。
pub fn boot_from_install() -> BootResult {
    let (cfg, cfg_diags) = load_desktop_config_with_diagnostics();
    for d in &cfg_diags {
        tracing::warn!("配置诊断 {} · {}", d.source, d.message);
    }
    match (&cfg.net_url, &cfg.net_room) {
        (Some(url), room) => {
            tracing::info!("联机配置预留 url={} room={}（协议未定点，不接 socket）", url, room.as_deref().unwrap_or("—"))
        }
        (None, _) => tracing::info!("联机配置：未设 net_url"),
    }
    let boot = match boot_world(&cfg) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("启动失败: {e}");
            BootResult {
                note: format!("启动失败: {e}"),
                engine: None,
                session: None,
                preview: None,
            }
        }
    };
    tracing::info!("boot: {} · session={}", boot.note, if boot.session.is_some() { "ok" } else { "none" });
    boot
}
