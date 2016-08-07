//! 对局侧栏 chrome：按已解析 [`UiFactionChrome`] 从 `sidecNN` 解码并合成。
//!
//! 文件名与菜单壳层分离；同名 SHP 靠 `MixFileIndex` 嵌套包区分外观。
//! 战术区铺到命令条顶边；chrome 含右侧栏与底边命令条。

use ra_assets::{Palette, ShpFile, parse_pcx, shp_body_frame_count};
use ra_renderer::RgbaImage;

use crate::{
    fs_source::GameAssetSource,
    screens::page::UiAssetRef,
    skin::decode::{DecodedUiSprite, frame_to_canvas_rgba},
    skirmish_setup::UiFactionChrome,
};

/// 对局侧栏调色板。

use super::chrome::{BATTLE_HUD_PAL, BattleHudChrome, COMMAND_BUTTON_SLOTS};


fn decode_asset_ref_preferring(source: &GameAssetSource, asset: &UiAssetRef, prefer_mix: &str) -> Result<DecodedUiSprite, String> {
    let frame_idx = asset.frame.unwrap_or(0) as usize;
    let hit = source
        .resolve_preferring(&asset.name, prefer_mix)
        .ok_or_else(|| format!("{}: 不可读", asset.name))?;
    let shp = ShpFile::parse(&hit.bytes).map_err(|e| format!("{}: SHP 解析失败 · {e}", asset.name))?;
    if shp.frames.is_empty() {
        return Err(format!("{}: SHP 无帧", asset.name));
    }
    if frame_idx >= shp.frames.len() {
        return Err(format!("{}: 帧 {} 越界 · 共 {} 帧", asset.name, frame_idx, shp.frames.len()));
    }
    let pal_name = asset.palette.as_deref().ok_or_else(|| format!("{}: 未指定调色板", asset.name))?;
    // 必须与 SHP 同档案取调色板：`sidec01`/`sidec02` 各有一份，
    // 不可回退全局 `resolve`（后挂载的苏军包常抢走 `sidebar.pal`）。
    let pal_hit = source
        .resolve_preferring(pal_name, prefer_mix)
        .ok_or_else(|| format!("{pal_name}: 调色板不可读（prefer {prefer_mix}）"))?;
    let palette = Palette::parse(&pal_hit.bytes).map_err(|e| format!("{pal_name}: 解析失败 · {e}"))?;
    let frame = &shp.frames[frame_idx];
    let image = frame_to_canvas_rgba(&shp, frame, &palette).ok_or_else(|| format!("{}#{}: 画布 RGBA 构造失败", asset.name, frame_idx))?;
    Ok(DecodedUiSprite {
        label: format!("{}#{}", asset.name, frame_idx),
        image,
        origin: format!("{} · pal {}", hit.explain(), pal_hit.explain()),
        frame: frame_idx as u16,
        canvas: (shp.width, shp.height),
        frame_rect: (frame.frame_x, frame.frame_y, frame.frame_width, frame.frame_height),
    })
}

/// 按候选嵌套包依次 `resolve_preferring`；皆无则失败。
fn decode_asset_ref_candidates(
    source: &GameAssetSource,
    asset: &UiAssetRef,
    prefer_mixes: &[&str],
) -> Result<DecodedUiSprite, String> {
    let mut last_err = format!("{}: 不可读", asset.name);
    for mix in prefer_mixes {
        match decode_asset_ref_preferring(source, asset, mix) {
            Ok(sprite) => return Ok(sprite),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}


fn try_decode(source: &GameAssetSource, mixes: &[&str], name: &str, pal: &str, frame: u16, errors: &mut Vec<String>) -> Option<DecodedUiSprite> {
    let asset = UiAssetRef::with_palette_frame(name, pal, frame);
    match decode_asset_ref_candidates(source, &asset, mixes) {
        Ok(s) => Some(s),
        Err(e) => {
            errors.push(e);
            None
        }
    }
}

/// 雷达 SHP 色帧中「开图」动画的下标区间（不含首帧徽与末帧关屏）。
///
/// `body_count` 为 [`shp_body_frame_count`]（不含落影半幅）。
pub fn radar_open_frame_range(body_count: usize) -> std::ops::Range<usize> {
    if body_count <= 1 {
        1..1
    } else if body_count == 2 {
        1..2
    } else {
        1..(body_count - 1)
    }
}

/// 开图动画帧推进间隔（逻辑 tick）。
pub const RADAR_OPEN_FRAME_TICKS: u64 = 2;


fn decode_radar_bundle(
    source: &GameAssetSource,
    mixes: &[&str],
    shp_name: &str,
    pal_name: &str,
) -> Result<(DecodedUiSprite, Vec<DecodedUiSprite>), String> {
    let mut last_err = format!("{shp_name}: 不可读");
    for mix in mixes {
        let hit = match source.resolve_preferring(shp_name, mix) {
            Some(h) => h,
            None => {
                last_err = format!("{shp_name}: 不可读（prefer {mix}）");
                continue;
            }
        };
        let shp = match ShpFile::parse(&hit.bytes) {
            Ok(s) => s,
            Err(e) => {
                last_err = format!("{shp_name}: SHP 解析失败 · {e}");
                continue;
            }
        };
        if shp.frames.is_empty() {
            last_err = format!("{shp_name}: SHP 无帧");
            continue;
        }
        let pal_hit = match source.resolve_preferring(pal_name, mix) {
            Some(h) => h,
            None => {
                last_err = format!("{pal_name}: 调色板不可读（prefer {mix}）");
                continue;
            }
        };
        let palette = match Palette::parse(&pal_hit.bytes) {
            Ok(p) => p,
            Err(e) => {
                last_err = format!("{pal_name}: 解析失败 · {e}");
                continue;
            }
        };
        let body = shp_body_frame_count(&shp.frames).max(1).min(shp.frames.len());
        let closed_frame = &shp.frames[0];
        let closed_image = match frame_to_canvas_rgba(&shp, closed_frame, &palette) {
            Some(img) => img,
            None => {
                last_err = format!("{shp_name}#0: 画布 RGBA 构造失败");
                continue;
            }
        };
        let closed = DecodedUiSprite {
            label: format!("{shp_name}#0"),
            image: closed_image,
            origin: format!("{} · pal {}", hit.explain(), pal_hit.explain()),
            frame: 0,
            canvas: (shp.width, shp.height),
            frame_rect: (
                closed_frame.frame_x,
                closed_frame.frame_y,
                closed_frame.frame_width,
                closed_frame.frame_height,
            ),
        };
        let mut open = Vec::new();
        for idx in radar_open_frame_range(body) {
            let frame = &shp.frames[idx];
            let Some(image) = frame_to_canvas_rgba(&shp, frame, &palette)
            else {
                continue;
            };
            open.push(DecodedUiSprite {
                label: format!("{shp_name}#{idx}"),
                image,
                origin: format!("{} · pal {}", hit.explain(), pal_hit.explain()),
                frame: idx as u16,
                canvas: (shp.width, shp.height),
                frame_rect: (frame.frame_x, frame.frame_y, frame.frame_width, frame.frame_height),
            });
        }
        return Ok((closed, open));
    }
    Err(last_err)
}

/// 按本地阵营解码对局 HUD chrome。
///
/// `faction_id` 为 rules `Side=`（如 `ThirdSide`）；模组未知国名时靠它选 UI 族。
pub fn decode_battle_hud_chrome(source: &GameAssetSource, side: &str) -> BattleHudChrome {
    decode_battle_hud_chrome_resolved(source, side, None)
}

/// 同 [`decode_battle_hud_chrome`]，可带 `Side=` 与可选 Side chrome。
pub fn decode_battle_hud_chrome_resolved(
    source: &GameAssetSource,
    side: &str,
    faction_id: Option<&str>,
) -> BattleHudChrome {
    decode_battle_hud_chrome_with(source, side, faction_id, None)
}

/// 同 [`decode_battle_hud_chrome_resolved`]，可注入已解析的 [`UiFactionChrome`]。
pub fn decode_battle_hud_chrome_with(
    source: &GameAssetSource,
    side: &str,
    _faction_id: Option<&str>,
    side_chrome: Option<&UiFactionChrome>,
) -> BattleHudChrome {
    let Some(chrome) = UiFactionChrome::resolve(side_chrome) else {
        return BattleHudChrome {
            side: side.to_string(),
            mix: String::new(),
            credits: None,
            top: None,
            radar: None,
            radar_open: Vec::new(),
            side1: None,
            side2: None,
            side3: None,
            addon: None,
            repair: None,
            repair_pressed: None,
            sell: None,
            sell_pressed: None,
            powerp: None,
            tabs: [None, None, None, None],
            tabs_pressed: [None, None, None, None],
            optbtn: None,
            diplobtn: None,
            lendcap: None,
            rendcap: None,
            lspacer: None,
            command_buttons: std::array::from_fn(|_| None),
            command_buttons_pressed: std::array::from_fn(|_| None),
            errors: vec!["缺少 Side chrome（无 MixFileIndex）".into()],
        };
    };
    let mixes_owned = chrome.sidebar_mix_candidates();
    let mixes: Vec<&str> = mixes_owned.iter().map(String::as_str).collect();
    let mix = chrome.sidebar_mix();
    let mut errors = Vec::new();
    let mut tabs = [None, None, None, None];
    let mut tabs_pressed = [None, None, None, None];
    for i in 0..4 {
        let name = format!("tab{i:02}.shp");
        tabs[i] = try_decode(source, &mixes, &name, BATTLE_HUD_PAL, 0, &mut errors);
        let asset1 = UiAssetRef::with_palette_frame(&name, BATTLE_HUD_PAL, 1);
        tabs_pressed[i] = decode_asset_ref_candidates(source, &asset1, &mixes).ok();
    }
    let mut command_buttons = std::array::from_fn(|_| None);
    let mut command_buttons_pressed = std::array::from_fn(|_| None);
    for i in 0..COMMAND_BUTTON_SLOTS {
        let name = format!("button{i:02}.shp");
        // 缺钮不记入 errors：零售包常只有 button00…11。
        let asset0 = UiAssetRef::with_palette_frame(&name, BATTLE_HUD_PAL, 0);
        if let Ok(s) = decode_asset_ref_candidates(source, &asset0, &mixes) {
            command_buttons[i] = Some(s);
        }
        let asset1 = UiAssetRef::with_palette_frame(&name, BATTLE_HUD_PAL, 1);
        if let Ok(s) = decode_asset_ref_candidates(source, &asset1, &mixes) {
            command_buttons_pressed[i] = Some(s);
        }
    }
    // 按 `YuriFileNames` 优先，再试另一套雷达文件名（模组 sidec 包常与 flags 不一致）。
    let mut radar = None;
    let mut radar_open = Vec::new();
    let mut radar_errors = Vec::new();
    for (radar_shp, radar_pal) in chrome.radar_shp_pal_candidates() {
        match decode_radar_bundle(source, &mixes, radar_shp, radar_pal) {
            Ok((closed, open)) => {
                radar = Some(closed);
                radar_open = open;
                break;
            }
            Err(e) => radar_errors.push(e),
        }
    }
    if radar.is_none() {
        errors.extend(radar_errors);
    }
    BattleHudChrome {
        side: side.to_string(),
        mix: mix.clone(),
        credits: try_decode(source, &mixes, "credits.shp", BATTLE_HUD_PAL, 0, &mut errors),
        top: try_decode(source, &mixes, "top.shp", BATTLE_HUD_PAL, 0, &mut errors),
        radar,
        radar_open,
        side1: try_decode(source, &mixes, "side1.shp", BATTLE_HUD_PAL, 0, &mut errors),
        side2: try_decode(source, &mixes, "side2.shp", BATTLE_HUD_PAL, 0, &mut errors),
        side3: try_decode(source, &mixes, "side3.shp", BATTLE_HUD_PAL, 0, &mut errors),
        addon: try_decode(source, &mixes, "addon.shp", BATTLE_HUD_PAL, 0, &mut errors),
        repair: try_decode(source, &mixes, "repair.shp", BATTLE_HUD_PAL, 0, &mut errors),
        // 缺按下帧不记 errors：回退常态即可。
        repair_pressed: {
            let asset = UiAssetRef::with_palette_frame("repair.shp", BATTLE_HUD_PAL, 1);
            decode_asset_ref_candidates(source, &asset, &mixes).ok()
        },
        sell: try_decode(source, &mixes, "sell.shp", BATTLE_HUD_PAL, 0, &mut errors),
        sell_pressed: {
            let asset = UiAssetRef::with_palette_frame("sell.shp", BATTLE_HUD_PAL, 1);
            decode_asset_ref_candidates(source, &asset, &mixes).ok()
        },
        powerp: try_decode(source, &mixes, "powerp.shp", BATTLE_HUD_PAL, 0, &mut errors),
        tabs,
        tabs_pressed,
        optbtn: try_decode(source, &mixes, "optbtn.shp", BATTLE_HUD_PAL, 0, &mut errors),
        diplobtn: try_decode(source, &mixes, "diplobtn.shp", BATTLE_HUD_PAL, 0, &mut errors),
        lendcap: try_decode(source, &mixes, "lendcap.shp", BATTLE_HUD_PAL, 0, &mut errors),
        rendcap: try_decode(source, &mixes, "rendcap.shp", BATTLE_HUD_PAL, 0, &mut errors),
        lspacer: try_decode(source, &mixes, "lspacer.shp", BATTLE_HUD_PAL, 0, &mut errors),
        command_buttons,
        command_buttons_pressed,
        errors,
    }
}

/// 按 `art.ini` 的 `CameoPCX=` / `Cameo=`（及回退名）解码建造栏图标。
///
/// 心灵终结等模组几乎只用 `CameoPCX=`；仍兼容原版 `Cameo=` SHP。
pub fn decode_cameo_sprite(
    source: &GameAssetSource,
    art: Option<&ra_assets::IniDocument>,
    type_id: &str,
) -> Option<DecodedUiSprite> {
    let mut pcx_names = Vec::new();
    let mut shp_names = Vec::new();
    if let Some(art) = art {
        push_cameo_pcx_name(&mut pcx_names, art.get(type_id, "CameoPCX"));
        push_cameo_shp_name(&mut shp_names, art.get(type_id, "Cameo"));
        let image_key = art.get(type_id, "Image").unwrap_or(type_id);
        if !image_key.eq_ignore_ascii_case(type_id) {
            push_cameo_pcx_name(&mut pcx_names, art.get(image_key, "CameoPCX"));
            push_cameo_shp_name(&mut shp_names, art.get(image_key, "Cameo"));
        }
        push_cameo_shp_name(&mut shp_names, art.get(type_id, "AltCameo"));
    }
    shp_names.push(format!("{type_id}icon.shp"));
    shp_names.push(format!("{type_id}.shp"));

    let mut last_err = None;
    for name in pcx_names {
        match decode_cameo_pcx(source, &name) {
            Ok(s) => return Some(s),
            Err(e) => last_err = Some(e),
        }
    }
    for name in shp_names {
        match decode_cameo_named(source, &name) {
            Ok(s) => return Some(s),
            Err(e) => last_err = Some(e),
        }
    }
    let _ = last_err;
    None
}


fn push_cameo_shp_name(out: &mut Vec<String>, raw: Option<&str>) {
    let Some(c) = raw.map(str::trim).filter(|s| !s.is_empty())
    else {
        return;
    };
    if c.to_ascii_lowercase().ends_with(".shp") {
        out.push(c.to_string());
    } else {
        out.push(format!("{c}.shp"));
    }
}


fn push_cameo_pcx_name(out: &mut Vec<String>, raw: Option<&str>) {
    let Some(c) = raw.map(str::trim).filter(|s| !s.is_empty())
    else {
        return;
    };
    if c.to_ascii_lowercase().ends_with(".pcx") {
        out.push(c.to_string());
    } else {
        out.push(format!("{c}.pcx"));
    }
}


fn decode_cameo_pcx(source: &GameAssetSource, name: &str) -> Result<DecodedUiSprite, String> {
    let hit = source
        .resolve(name)
        .ok_or_else(|| format!("{name}: 不可读"))?;
    let pcx = parse_pcx(&hit.bytes).map_err(|e| format!("{name}: PCX 解析失败 · {e}"))?;
    let mut rgba = pcx.rgba;
    // 与壳层旗标一致：品红作色键透明。
    for px in rgba.chunks_exact_mut(4) {
        if px[0] == 255 && px[1] == 0 && px[2] == 255 {
            px[3] = 0;
        }
    }
    let image = RgbaImage::from_raw(pcx.width, pcx.height, rgba)
        .ok_or_else(|| format!("{name}: 画布 RGBA 构造失败"))?;
    let w = pcx.width.min(u32::from(u16::MAX)) as u16;
    let h = pcx.height.min(u32::from(u16::MAX)) as u16;
    Ok(DecodedUiSprite {
        label: format!("{name}#0"),
        image,
        origin: hit.explain(),
        frame: 0,
        canvas: (w, h),
        frame_rect: (0, 0, w, h),
    })
}


fn decode_cameo_named(source: &GameAssetSource, name: &str) -> Result<DecodedUiSprite, String> {
    // MD 优先：模组图标多在 `cameomd`；基座 `cameo.mix` 次之。
    const CAMEO_MIXES: &[&str] = &["cameomd.mix", "cameo.mix"];
    let mut hit = None;
    let mut hit_mix: Option<String> = None;
    for mix in CAMEO_MIXES {
        if let Some(h) = source.resolve_preferring(name, mix) {
            hit = Some(h);
            hit_mix = Some((*mix).to_string());
            break;
        }
    }
    let hit = match hit {
        Some(h) => h,
        None => {
            let h = source.resolve(name).ok_or_else(|| format!("{name}: 不可读"))?;
            // 扩展包图标：与 SHP 同档取 `cameo.pal`，避免基座板粉噪。
            if let crate::fs_source::AssetOrigin::Mix { archive, .. } = &h.origin {
                hit_mix = Some(archive.clone());
            }
            h
        }
    };
    let shp = ShpFile::parse(&hit.bytes).map_err(|e| format!("{name}: SHP 解析失败 · {e}"))?;
    if shp.frames.is_empty() {
        return Err(format!("{name}: SHP 无帧"));
    }
    let pal_hit = resolve_cameo_palette(source, hit_mix.as_deref())
        .ok_or_else(|| "cameo.pal: 调色板不可读".to_string())?;
    let palette = Palette::parse(&pal_hit.bytes).map_err(|e| format!("cameo.pal: 解析失败 · {e}"))?;
    let frame = &shp.frames[0];
    let image = frame_to_canvas_rgba(&shp, frame, &palette).ok_or_else(|| format!("{name}#0: 画布 RGBA 构造失败"))?;
    Ok(DecodedUiSprite {
        label: format!("{name}#0"),
        image,
        origin: format!("{} · pal {}", hit.explain(), pal_hit.explain()),
        frame: 0,
        canvas: (shp.width, shp.height),
        frame_rect: (frame.frame_x, frame.frame_y, frame.frame_width, frame.frame_height),
    })
}

/// 解析 cameo 调色板：优先与图标同档 / 标准 cameo·cache 包，**禁止**裸全局解析。
///
/// 扩展包偶发覆盖同名 `cameo.pal`（内容并非建造栏板），全局胜出后图标会粉噪。
fn resolve_cameo_palette<'a>(
    source: &'a GameAssetSource,
    shp_mix: Option<&str>,
) -> Option<crate::skin::fs_source::AssetHit> {
    let mut tried = Vec::<String>::new();
    let mut try_mix = |mix: &str| -> Option<crate::skin::fs_source::AssetHit> {
        if tried.iter().any(|t| t.eq_ignore_ascii_case(mix)) {
            return None;
        }
        tried.push(mix.to_string());
        source.resolve_preferring("cameo.pal", mix)
    };
    if let Some(mix) = shp_mix {
        if let Some(h) = try_mix(mix) {
            return Some(h);
        }
    }
    for mix in ["cameomd.mix", "cameo.mix", "cachemd.mix", "cache.mix"] {
        if let Some(h) = try_mix(mix) {
            return Some(h);
        }
    }
    // 最后才用侧栏板，仍不裸 resolve `cameo.pal`。
    source
        .resolve_preferring("sidebar.pal", shp_mix.unwrap_or("cache.mix"))
        .or_else(|| source.resolve_preferring("sidebar.pal", "cache.mix"))
}
