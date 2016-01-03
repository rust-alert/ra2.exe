//! 安装探测、资源挂载与遭遇战会话打开（对局前装载，不属于 `BattleController`）。

use std::collections::HashMap;

use ra_adaptor::{ResourceChain, RulesSystem, detect_edition, load_rules_chain};
use ra_assets::{
    CountryRegistry, IniDocument, Palette, Rgba, find_battle_campaign, parse_battle_campaigns, parse_mpmodes,
};
use ra_engine::{Engine, Session, open_campaign_session, open_skirmish_session};
use ra_map::{
    MapEntity, MapEntityKind, MapInfo, MobilePaintPose, StructureAnimBank, campaign_blocking_capability_message,
    compose_boot_preview, count_skirmish_start_slots, decode_preview_from_map_bytes, find_boot_map,
    list_parseable_maps_from_missions_pkt, list_parseable_maps_from_names, map_scripting_capability_gaps,
    mount_theater_mixes, paint_mobiles_onto_preview_rgba, paint_structure_anims_onto_rgba,
};
use ra_renderer::RgbaImage;
use ra_types::{AssetSource, GameEdition, RaResult};
use ra_widgets::load_kind::LoadKind;
use ra_widgets::campaign_setup::campaign_side_battle_id;
use ra_widgets::fs_source::GameAssetSource;
use ra_widgets::skirmish_setup::{LOBBY_COLORS, SkirmishBootRequest};

use super::config::{DesktopConfig, load_desktop_config_with_diagnostics};

pub use ra_assets::{BattleCampaign, CountryDef, MpMode, SideGroup};
pub use ra_map::{BootMapCandidate, skirmish_ai_row_count};

/// 一次装载尝试的结果（成功或带说明的失败）。
///
/// `Result::Ok` 只表示装载流程跑完；是否可开战看 [`BootResult::is_ready`]。
#[derive(Debug)]
pub struct BootResult {
    /// 人类可读装载备注。
    pub note: String,
    /// 长期引擎（与会话共享定义生命周期）。
    pub engine: Option<Engine>,
    /// 已打开的会话（若装载成功）。
    pub session: Option<Session>,
    /// 可选地形预览图（含当前活动层）。
    pub preview: Option<RgbaImage>,
    /// 不含建筑活动层的预览底图（对局时钟刷新用；可含开局移动单位）。
    pub preview_base: Option<RgbaImage>,
    /// 无开局移动单位的底图（部署后重组预览用）。
    pub preview_clean: Option<RgbaImage>,
    /// 建筑活动层银行。
    pub structure_anims: StructureAnimBank,
    /// 预览画布原点（世界像素）。
    pub preview_origin: (i32, i32),
    /// 资源链 art INI 逻辑名（对局部署 Buildup 用）。
    pub art_ini: &'static str,
    /// 资源链 rules INI 逻辑名。
    pub rules_ini: &'static str,
    /// 已解析规则（房屋色调 / 部署叠画）。
    pub rules: Option<RulesSystem>,
    /// 大厅行色 → house 主色。
    pub lobby_primaries: HashMap<String, Rgba>,
}

impl BootResult {
    /// 是否已打开可玩会话（进度「完成」与进对局的唯一判据）。
    pub fn is_ready(&self) -> bool {
        self.session.as_ref().and_then(|s| s.battle()).is_some()
    }

    /// 装载失败占位。
    pub fn failed(note: impl Into<String>) -> Self {
        Self {
            note: note.into(),
            engine: None,
            session: None,
            preview: None,
            preview_base: None,
            preview_clean: None,
            structure_anims: StructureAnimBank::default(),
            preview_origin: (0, 0),
            art_ini: "art.ini",
            rules_ini: "rules.ini",
            rules: None,
            lobby_primaries: HashMap::new(),
        }
    }

    /// 由测试场景 [`super::test_boot::TestBoot`] 构造（无活动层银行）。
    #[cfg(feature = "test-harness")]
    pub fn from_test(t: super::test_boot::TestBoot) -> Self {
        Self {
            note: t.note,
            engine: Some(t.engine),
            session: Some(t.session),
            preview: t.preview,
            preview_base: None,
            preview_clean: None,
            structure_anims: StructureAnimBank::default(),
            preview_origin: (0, 0),
            art_ini: "art.ini",
            rules_ini: "rules.ini",
            rules: None,
            lobby_primaries: HashMap::new(),
        }
    }
}

fn load_map_terrain_preview(
    source: &GameAssetSource,
    map: &MapInfo,
    chain: &ResourceChain,
    rules: &RulesSystem,
    lobby_primaries: Option<&HashMap<String, Rgba>>,
) -> Option<(String, RgbaImage, RgbaImage, StructureAnimBank, i32, i32)> {
    let preview = compose_boot_preview(
        source,
        map,
        chain.art_ini,
        chain.rules_ini,
        &|id| rules.overlay_types.name(id).map(str::to_owned),
        &|id| {
            rules.overlay_types.name(id).is_some_and(|n| {
                rules
                    .rules
                    .get(n, "Tiberium")
                    .is_some_and(|v| v.eq_ignore_ascii_case("yes"))
                    || rules
                        .rules
                        .get(n, "SpawnsTiberium")
                        .is_some_and(|v| v.eq_ignore_ascii_case("yes"))
            })
        },
        &|base, owner| remap_owner_palette(rules, lobby_primaries, base, owner),
    )?;
    let rgba = preview.image.image;
    Some((
        preview.note,
        rgba,
        preview.base_without_anims,
        preview.anim_bank,
        preview.origin_x,
        preview.origin_y,
    ))
}

/// 大厅行色 → house 主色（遭遇战阵营色以大厅为准，不用国家默认 `Color=Gold`）。
fn lobby_house_primaries(request: &SkirmishBootRequest) -> HashMap<String, Rgba> {
    let mut out = HashMap::new();
    if request.sides.is_empty() {
        return out;
    }
    for (row, &side_i) in request.row_sides.iter().enumerate() {
        let Some(side) = request.sides.get(usize::from(side_i) % request.sides.len())
        else {
            continue;
        };
        let ci = usize::from(request.row_colors[row]) % LOBBY_COLORS.len();
        let [r, g, b] = LOBBY_COLORS[ci];
        out.insert(side.to_ascii_uppercase(), Rgba::rgb(r, g, b));
    }
    let ci = usize::from(request.color_index) % LOBBY_COLORS.len();
    let [r, g, b] = LOBBY_COLORS[ci];
    out.insert(request.side.to_ascii_uppercase(), Rgba::rgb(r, g, b));
    out
}

/// 房屋色调色板（大厅行色优先，否则国家默认配色）。
pub(crate) fn remap_owner_palette(
    rules: &RulesSystem,
    lobby_primaries: Option<&HashMap<String, Rgba>>,
    base: &Palette,
    owner: &str,
) -> Palette {
    let up = owner.to_ascii_uppercase();
    if matches!(up.as_str(), "NEUTRAL" | "SPECIAL" | "CIVILIAN") {
        return rules.color_schemes.palette_for_house(&rules.rules, base, owner);
    }
    if let Some(primary) = lobby_primaries.and_then(|m| m.get(&up)) {
        return base.with_house_remap(*primary);
    }
    rules.color_schemes.palette_for_house(&rules.rules, base, owner)
}

/// 将会话里已有的移动单位（含航点播种 MCV）叠画到启动预览底图。
fn paint_session_mobiles_onto_preview(
    source: &GameAssetSource,
    chain: &ResourceChain,
    rules: &RulesSystem,
    session: &Session,
    image: &mut RgbaImage,
    origin: (i32, i32),
    lobby_primaries: &HashMap<String, Rgba>,
) -> usize {
    let Some(game) = session.battle()
    else {
        return 0;
    };
    let mut paint_map = game.world.map.clone();
    paint_map.entities.clear();
    for id in game.world.entity_ids() {
        let Some((type_id, kind)) = game.world.ecs_identity(id)
        else {
            continue;
        };
        if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
            continue;
        }
        let Some(owner) = game.world.ecs_owner(id)
        else {
            continue;
        };
        let Some((x, y, facing)) = game.world.ecs_transform(id)
        else {
            continue;
        };
        if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
            continue;
        }
        paint_map.entities.push(MapEntity {
            kind,
            owner: owner.to_string(),
            type_id: type_id.to_string(),
            health: 256,
            x,
            y,
            facing,
            sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
        });
    }
    if paint_map.entities.is_empty() {
        return 0;
    }
    paint_mobiles_onto_preview_rgba(
        source,
        &paint_map,
        image,
        origin.0,
        origin.1,
        chain.art_ini,
        chain.rules_ini,
        &|base, owner| remap_owner_palette(rules, Some(lobby_primaries), base, owner),
        &|_| MobilePaintPose::default(),
    )
}

fn load_boot_map(
    source: &mut GameAssetSource,
    edition: GameEdition,
    note: &mut String,
    preferred_map: Option<&str>,
) -> Result<MapInfo, String> {
    let loaded = find_boot_map(edition, source, preferred_map)?;
    let theater_mounted =
        mount_theater_mixes(loaded.map.theater, &mut |mix| matches!(source.vfs.mount_nested_all_from_parents(mix), Ok(n) if n > 0));
    *note = format!("{note} · {} · 剧院mix {}", loaded.note, theater_mounted);
    Ok(loaded.map)
}

/// 列出安装资源中可解析的遭遇战地图（供大厅选图）。
///
/// 优先按资源链 `missions_pkt` 的 `[MultiMaps]` 源序；缺表或空表时回退到松散/`mp*.map` 扫描。
pub fn list_install_boot_maps() -> Vec<BootMapCandidate> {
    let (cfg, _) = load_desktop_config_with_diagnostics();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => GameEdition::parse(s).ok(),
        None => None,
    };
    let Ok(manifest) = detect_edition(&cfg.ra2_dir, explicit)
    else {
        return Vec::new();
    };
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);
    if let Ok(pkt) = source.read(manifest.chain.missions_pkt) {
        let from_pkt = list_parseable_maps_from_missions_pkt(manifest.chain.edition, &source, &pkt);
        if !from_pkt.is_empty() {
            return from_pkt;
        }
        tracing::warn!(
            file = %manifest.chain.missions_pkt,
            "遭遇战选图表可读但未产出可解析行，回退扫描"
        );
    } else {
        tracing::warn!(file = %manifest.chain.missions_pkt, "遭遇战选图表不可读，回退扫描");
    }
    let names = source.discover_skirmish_map_names();
    list_parseable_maps_from_names(manifest.chain.edition, &source, names)
}

/// 列出安装资源链中离线遭遇战可选多人模式（来自 `mpmodes.ini` / `mpmodesmd.ini`）。
///
/// 失败或缺文件时返回空表；调用方应回退到空列表 UI，勿写死模式名。
pub fn list_install_skirmish_modes() -> Vec<MpMode> {
    let (cfg, _) = load_desktop_config_with_diagnostics();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => GameEdition::parse(s).ok(),
        None => None,
    };
    let Ok(manifest) = detect_edition(&cfg.ra2_dir, explicit)
    else {
        return Vec::new();
    };
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);
    let Some(bytes) = source.vfs.read(manifest.chain.mpmodes_ini)
    else {
        tracing::warn!(file = %manifest.chain.mpmodes_ini, "多人模式表不可读");
        return Vec::new();
    };
    match parse_mpmodes(&bytes) {
        Ok(modes) => modes.into_iter().filter(|m| m.visible_in_offline_skirmish()).collect(),
        Err(e) => {
            tracing::warn!(file = %manifest.chain.mpmodes_ini, error = %e, "多人模式表解析失败");
            Vec::new()
        }
    }
}

/// 列出安装资源链中遭遇战可选国家 / 势力（来自 `rules.ini` 的 `[Countries]` / `[Sides]`）。
///
/// 失败或缺文件时返回空表；调用方应回退到空列表 UI，勿写死国家名。
pub fn list_install_skirmish_countries() -> (Vec<CountryDef>, Vec<SideGroup>) {
    let (cfg, _) = load_desktop_config_with_diagnostics();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => GameEdition::parse(s).ok(),
        None => None,
    };
    let Ok(manifest) = detect_edition(&cfg.ra2_dir, explicit)
    else {
        return (Vec::new(), Vec::new());
    };
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);
    let Some(bytes) = source.vfs.read(manifest.chain.rules_ini)
    else {
        tracing::warn!(file = %manifest.chain.rules_ini, "规则表不可读，无法列出国家");
        return (Vec::new(), Vec::new());
    };
    let Ok(doc) = IniDocument::parse(&bytes)
    else {
        tracing::warn!(file = %manifest.chain.rules_ini, "规则表解析失败，无法列出国家");
        return (Vec::new(), Vec::new());
    };
    let registry = CountryRegistry::from_rules(&doc);
    let countries: Vec<CountryDef> = registry.skirmish_countries().into_iter().cloned().collect();
    let sides = registry.sides().to_vec();
    (countries, sides)
}

/// 列出安装资源链中的战役表（来自 `battle.ini` / `battlemd.ini`）。
///
/// 失败或缺文件时返回空表。
pub fn list_install_battle_campaigns() -> Vec<BattleCampaign> {
    let (cfg, _) = load_desktop_config_with_diagnostics();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => GameEdition::parse(s).ok(),
        None => None,
    };
    let Ok(manifest) = detect_edition(&cfg.ra2_dir, explicit)
    else {
        return Vec::new();
    };
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);
    let Some(bytes) = source.vfs.read(manifest.chain.battle_ini)
    else {
        tracing::warn!(file = %manifest.chain.battle_ini, "战役表不可读");
        return Vec::new();
    };
    match parse_battle_campaigns(&bytes) {
        Ok(camps) => camps,
        Err(e) => {
            tracing::warn!(file = %manifest.chain.battle_ini, error = %e, "战役表解析失败");
            Vec::new()
        }
    }
}

/// 按选边入口（`allied` / `tutorial` / `soviet`）解析首关战役定义。
pub fn resolve_install_campaign_for_side(side: &str) -> Option<BattleCampaign> {
    let battle_id = campaign_side_battle_id(side)?;
    let camps = list_install_battle_campaigns();
    find_battle_campaign(&camps, battle_id).cloned()
}

/// 为遭遇战大厅生成指定地图的烘焙缩略图（`[PreviewPack]`，未缩小）。
///
/// 与原版大厅一致：只读地图内预烘焙预览，不做等距地形合成。
/// 失败时返回 `None`（缺图、无 `[Preview]` / `[PreviewPack]` 或解码失败）。
pub fn preview_install_boot_map(map_name: &str) -> Option<(String, RgbaImage)> {
    let (cfg, _) = load_desktop_config_with_diagnostics();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => GameEdition::parse(s).ok(),
        None => None,
    };
    let manifest = detect_edition(&cfg.ra2_dir, explicit).ok()?;
    let mut source = GameAssetSource::new(manifest.root.clone());
    let _ = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let _ = source.mount_nested_plan(&manifest.composition.nested_mount_plan);
    let bytes = source.read(map_name).ok()?;
    let preview = match decode_preview_from_map_bytes(&bytes) {
        Ok(Some(img)) => img,
        Ok(None) => {
            tracing::warn!(map = %map_name, "地图无 [PreviewPack]");
            return None;
        }
        Err(e) => {
            tracing::warn!(map = %map_name, error = %e, "PreviewPack 解码失败");
            return None;
        }
    };
    let width = preview.width;
    let height = preview.height;
    let image = preview.into_rgba_image()?;
    Some((format!("map:{map_name} · PreviewPack {width}x{height}"), image))
}

/// 按桌面配置探测安装并打开一局遭遇战会话。
pub fn boot_world(cfg: &DesktopConfig, request: &ra_widgets::skirmish_setup::SkirmishBootRequest) -> RaResult<BootResult> {
    boot_world_with_progress(cfg, request, |_, _| {})
}

/// 与 [`boot_world`] 相同，并按装载阶段回调进度（`ratio` 为 0..1）。
pub fn boot_world_with_progress(
    cfg: &DesktopConfig,
    request: &ra_widgets::skirmish_setup::SkirmishBootRequest,
    mut report: impl FnMut(f32, &str),
) -> RaResult<BootResult> {
    report(0.08, "探测安装");
    let root = cfg.ra2_dir.clone();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&root, explicit)?;
    for item in &manifest.stack.unsupported {
        tracing::warn!("适配能力缺口 [{}] {}", item.code, item.message);
    }
    if !manifest.stack.extensions.is_empty() {
        let ids: Vec<_> = manifest.stack.extensions.iter().map(|e| e.as_str()).collect();
        tracing::info!("适配扩展探测: {}", ids.join("+"));
    }
    let chain = &manifest.chain;

    for line in manifest.composition.diagnostics.summary_lines() {
        tracing::info!("资源组合 {line}");
    }

    report(0.22, "挂载资源");
    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, skipped_root) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    for line in manifest.composition.mount_plan_lines() {
        tracing::info!("资源组合 {line}");
    }
    let (mounted_nested, skipped_nested) = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    if let Some(hit) = source.resolve(chain.rules_ini) {
        tracing::info!("资源组合 resolved: rules={} · {}", chain.rules_ini, hit.explain());
    }
    else {
        tracing::warn!("资源组合 resolved: rules=(missing) {}", chain.rules_ini);
    }

    let expand_n = manifest.composition.diagnostics.detected_expansions.len();
    let skipped_total = skipped_root.saturating_add(skipped_nested);
    let mut note = format!(
        "{} · 根mix {} · 嵌套 {} · 跳过 {} · 缺盘 {} · expand#{} · {}",
        chain.edition.as_str(),
        mounted_root,
        mounted_nested,
        skipped_total,
        manifest.missing_mixes.len(),
        expand_n,
        request.note_fragment()
    );

    report(0.40, "装载地图");
    let map = match load_boot_map(&mut source, chain.edition, &mut note, request.preferred_map.as_deref()) {
        Ok(map) => map,
        Err(e) => {
            note = format!("{note} · {e}");
            report(1.0, "地图失败");
            return Ok(BootResult::failed(note));
        }
    };

    report(0.55, "解析规则");
    let mut preview_origin = (0i32, 0i32);
    let rules = match load_rules_chain(&source, chain) {
        Ok(db) => Some(db),
        Err(e) => {
            // 规则是开战硬前置：解析失败不得静默成 session=none。
            note = format!("{note} · 规则解析失败（{e}）");
            tracing::error!(error = %e, rules = %chain.rules_ini, art = %chain.art_ini, "规则装载失败");
            None
        }
    };

    report(0.70, "地形预览");
    let lobby_primaries = lobby_house_primaries(request);
    let mut preview_base: Option<RgbaImage> = None;
    let mut preview_clean: Option<RgbaImage> = None;
    let mut structure_anims = StructureAnimBank::default();
    let mut preview = match rules
        .as_ref()
        .and_then(|rules| load_map_terrain_preview(&source, &map, chain, rules, Some(&lobby_primaries)))
    {
        Some((name, image, base, bank, ox, oy)) => {
            note = format!("{note} · preview:{name}");
            preview_origin = (ox, oy);
            preview_base = Some(base);
            structure_anims = bank;
            Some(image)
        }
        None => {
            note = format!("{note} · preview:无");
            None
        }
    };

    report(0.88, "打开会话");
    for gap in map_scripting_capability_gaps(&map) {
        tracing::warn!("地图能力缺口 [{}] {}", gap.code, gap.message);
        note = format!("{note} · gap:{}", gap.code);
    }
    if request.boot_kind == LoadKind::Campaign {
        if let Some(msg) = campaign_blocking_capability_message(&map) {
            note = format!("{note} · {msg}");
            tracing::error!(%msg, "战役装载因剧本缺口拒绝");
            report(1.0, "剧本缺口");
            return Ok(BootResult::failed(note));
        }
    }
    let preferred_house = Some(request.side.as_str());
    let ai_rows = skirmish_ai_row_count(count_skirmish_start_slots(&map.waypoints, &map.name));
    let ensure_houses = request.houses_to_ensure(ai_rows);
    let ensure_refs: Vec<&str> = ensure_houses.iter().map(String::as_str).collect();
    let (engine, session) = match rules.as_ref().map(|rules| match request.boot_kind {
        LoadKind::Campaign => open_campaign_session(
            &source,
            chain,
            rules,
            map,
            note.clone(),
            preview_origin,
            preferred_house,
            &ensure_refs,
            request.match_seed,
        ),
        LoadKind::Skirmish => open_skirmish_session(
            &source,
            chain,
            rules,
            map,
            note.clone(),
            preview_origin,
            preferred_house,
            &ensure_refs,
            request.match_seed,
        ),
    }) {
        Some(Ok(mut opened)) => {
            note = opened.note;
            note = format!(
                "{note} · player={} · difficulty={} · credits={} · tech={} · seed={:#x} · houses={}",
                request.player_name,
                request.difficulty,
                request.credits,
                request.tech_level,
                request.match_seed,
                ensure_houses.join("+")
            );
            let game = opened.session.expect_battle_mut();
            game.set_difficulty(request.difficulty.clone());
            // 战役资金以地图 Houses.Credits 为准；遭遇战仍用大厅 credits。
            if request.boot_kind != LoadKind::Campaign {
                game.world.set_all_players_funds(request.credits);
            }
            game.world.set_all_players_tech_level(request.tech_level);
            tracing::info!(
                "fingerprint edition={} map={} rules_hash={:#x} seed={:#x}",
                opened.session.expect_battle().fingerprint.edition,
                opened.session.expect_battle().fingerprint.map,
                opened.session.expect_battle().fingerprint.rules_hash,
                opened.session.expect_battle().match_seed
            );
            // 航点播种的 MCV 不在地图放置段：保留无 mobile 底图，再叠到对局底图。
            if let (Some(base), Some(rules)) = (preview_base.as_mut(), rules.as_ref()) {
                preview_clean = Some(base.clone());
                let painted =
                    paint_session_mobiles_onto_preview(&source, chain, rules, &opened.session, base, preview_origin, &lobby_primaries);
                if painted > 0 {
                    note = format!("{note} · start_mobile_shp#{painted}");
                }
                else {
                    tracing::warn!("开局移动单位未能叠画到预览（VXL/SHP 可能未解析）");
                }
                let mut composed = base.clone();
                paint_structure_anims_onto_rgba(&mut composed, preview_origin.0, preview_origin.1, &structure_anims, 0);
                preview = Some(composed);
            }
            (Some(opened.engine), Some(opened.session))
        }
        Some(Err(e)) => {
            note = format!("{note} · 会话未打开（{e}）");
            (None, None)
        }
        None => (None, None),
    };

    if session.as_ref().and_then(|s| s.battle()).is_some() {
        report(1.0, "完成");
    }
    else {
        report(1.0, "装载失败");
    }
    Ok(BootResult {
        note,
        engine,
        session,
        preview,
        preview_base,
        preview_clean,
        structure_anims,
        preview_origin,
        art_ini: chain.art_ini,
        rules_ini: chain.rules_ini,
        rules,
        lobby_primaries,
    })
}

/// 读取 `RustAlert.toml`（可选）并尝试装载（失败时仍返回带 note 的 `BootResult`）。
pub fn boot_from_install() -> BootResult {
    boot_from_install_with_request(ra_widgets::skirmish_setup::SkirmishBootRequest::default_lobby())
}

/// 指定地图文件名后装载（找不到则失败，不换图）。
#[allow(dead_code)]
pub fn boot_from_install_with_map(preferred_map: Option<String>) -> BootResult {
    let mut req = ra_widgets::skirmish_setup::SkirmishBootRequest::default_lobby();
    req.preferred_map = preferred_map;
    boot_from_install_with_request(req)
}

/// 按大厅遭遇战请求装载。
pub fn boot_from_install_with_request(request: ra_widgets::skirmish_setup::SkirmishBootRequest) -> BootResult {
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
    let boot = match boot_world(&cfg, &request) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("启动失败: {e}");
            BootResult::failed(format!("启动失败: {e}"))
        }
    };
    tracing::info!("boot: {} · session={}", boot.note, if boot.session.is_some() { "ok" } else { "none" });
    boot
}

/// 按大厅遭遇战请求装载，并向回调报告阶段进度。
pub fn boot_from_install_with_progress(
    request: ra_widgets::skirmish_setup::SkirmishBootRequest,
    mut report: impl FnMut(f32, &str),
) -> BootResult {
    report(0.04, "读取配置");
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
    let boot = match boot_world_with_progress(&cfg, &request, &mut report) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("启动失败: {e}");
            BootResult::failed(format!("启动失败: {e}"))
        }
    };
    tracing::info!("boot: {} · session={}", boot.note, if boot.session.is_some() { "ok" } else { "none" });
    boot
}
