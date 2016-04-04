//! 对局侧栏 chrome：按已解析 [`UiFactionChrome`] 从 `sidecNN` 解码并合成。
//!
//! 文件名与菜单壳层分离；同名 SHP 靠 `MixFileIndex` 嵌套包区分外观。
//! 战术区铺到命令条顶边；chrome 含右侧栏与底边命令条。

use ra_assets::{Palette, ShpFile};
use ra_layout::{
    rect_px_from_snapshot, solve_battle_hud_with_metrics, BattleHudChromeMetrics, LayoutSnapshot,
    Point2, RectPx, COMMAND_BAR_BUTTON_IDS, COMMAND_BAR_BUTTON_COUNT, SIDEBAR_TAB_COUNT,
    cameo_slot_rect, hit_cameo_slot,
};
use ra_renderer::RgbaImage;

use crate::{
    fs_source::GameAssetSource,
    screens::page::UiAssetRef,
    skin::decode::{DecodedUiSprite, frame_to_canvas_rgba},
    skin::text::{SKIRMISH_COMMAND_BAR, command_bar_shp_index},
    skirmish_setup::UiFactionChrome,
};

/// 对局侧栏调色板。
pub const BATTLE_HUD_PAL: &str = "sidebar.pal";

/// 命令条按钮槽位数（`button00`…`button11`；更高编号在零售包中常缺）。
pub const COMMAND_BUTTON_SLOTS: usize = 12;

/// 已解码的对局 HUD chrome（右侧栏 + 底边命令条）。
#[derive(Debug, Clone)]
pub struct BattleHudChrome {
    /// 阵营短名（如 `Americans` / `Russians`）。
    pub side: String,
    /// 实际优先读取的嵌套包名。
    pub mix: String,
    /// `credits.shp`。
    pub credits: Option<DecodedUiSprite>,
    /// `top.shp`。
    pub top: Option<DecodedUiSprite>,
    /// `radar.shp`（雷达未开时用末帧）。
    pub radar: Option<DecodedUiSprite>,
    /// `side1.shp`。
    pub side1: Option<DecodedUiSprite>,
    /// `side2.shp`（平铺）。
    pub side2: Option<DecodedUiSprite>,
    /// `side3.shp`。
    pub side3: Option<DecodedUiSprite>,
    /// `addon.shp`。
    pub addon: Option<DecodedUiSprite>,
    /// `repair.shp`。
    pub repair: Option<DecodedUiSprite>,
    /// `sell.shp`。
    pub sell: Option<DecodedUiSprite>,
    /// `powerp.shp`（电表）。
    pub powerp: Option<DecodedUiSprite>,
    /// `tab00`…`tab03`。
    pub tabs: [Option<DecodedUiSprite>; 4],
    /// `optbtn.shp`。
    pub optbtn: Option<DecodedUiSprite>,
    /// `diplobtn.shp`。
    pub diplobtn: Option<DecodedUiSprite>,
    /// `lendcap.shp`（命令条左端盖）。
    pub lendcap: Option<DecodedUiSprite>,
    /// `rendcap.shp`（命令条右端盖）。
    pub rendcap: Option<DecodedUiSprite>,
    /// `lspacer.shp`（命令条中段金属轨，按视口横向拉伸）。
    pub lspacer: Option<DecodedUiSprite>,
    /// `button00`…`button11` 常态帧（frame 0）。
    pub command_buttons: [Option<DecodedUiSprite>; COMMAND_BUTTON_SLOTS],
    /// `button00`…`button11` 按下高亮帧（frame 1；缺帧时回退常态）。
    pub command_buttons_pressed: [Option<DecodedUiSprite>; COMMAND_BUTTON_SLOTS],
    /// 解码失败说明。
    pub errors: Vec<String>,
}

impl BattleHudChrome {
    /// 是否至少解出右栏主体。
    pub fn has_sidebar_body(&self) -> bool {
        self.side1.is_some() || self.credits.is_some() || self.radar.is_some() || self.side2.is_some()
    }
}

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

fn radar_frame_index(_source: &GameAssetSource, _mix: &str) -> u16 {
    // 未建雷达时原版显示阵营徽（盟军鹰 / 苏军镰锤 / 尤里 Y），在雷达 SHP 首帧。
    // 末帧多为关屏黑块，不能当默认态。
    0
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
            side1: None,
            side2: None,
            side3: None,
            addon: None,
            repair: None,
            sell: None,
            powerp: None,
            tabs: [None, None, None, None],
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
    let radar_frame = radar_frame_index(source, &mix);
    let radar_shp = chrome.radar_shp();
    let radar_pal = chrome.radar_pal();
    let mut tabs = [None, None, None, None];
    for (i, slot) in tabs.iter_mut().enumerate() {
        let name = format!("tab{i:02}.shp");
        *slot = try_decode(source, &mixes, &name, BATTLE_HUD_PAL, 0, &mut errors);
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
    let radar = try_decode(source, &mixes, radar_shp, radar_pal, radar_frame, &mut errors);
    BattleHudChrome {
        side: side.to_string(),
        mix: mix.clone(),
        credits: try_decode(source, &mixes, "credits.shp", BATTLE_HUD_PAL, 0, &mut errors),
        top: try_decode(source, &mixes, "top.shp", BATTLE_HUD_PAL, 0, &mut errors),
        radar,
        side1: try_decode(source, &mixes, "side1.shp", BATTLE_HUD_PAL, 0, &mut errors),
        side2: try_decode(source, &mixes, "side2.shp", BATTLE_HUD_PAL, 0, &mut errors),
        side3: try_decode(source, &mixes, "side3.shp", BATTLE_HUD_PAL, 0, &mut errors),
        addon: try_decode(source, &mixes, "addon.shp", BATTLE_HUD_PAL, 0, &mut errors),
        repair: try_decode(source, &mixes, "repair.shp", BATTLE_HUD_PAL, 0, &mut errors),
        sell: try_decode(source, &mixes, "sell.shp", BATTLE_HUD_PAL, 0, &mut errors),
        powerp: try_decode(source, &mixes, "powerp.shp", BATTLE_HUD_PAL, 0, &mut errors),
        tabs,
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

fn blit_rgba(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    if src.width() == 0 || src.height() == 0 || dst.width() == 0 || dst.height() == 0 {
        return;
    }
    for row in 0..src.height() {
        let dy = y + row as i32;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..src.width() {
            let dx = x + col as i32;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let si = ((row * src.width() + col) * 4) as usize;
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            let sa = src.as_raw()[si + 3];
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
                continue;
            }
            let inv = 255u32 - sa as u32;
            for c in 0..3 {
                let s = src.as_raw()[si + c] as u32;
                let d = dst.as_mut()[di + c] as u32;
                dst.as_mut()[di + c] = ((s * sa as u32 + d * inv) / 255) as u8;
            }
            dst.as_mut()[di + 3] = 255;
        }
    }
}

fn blit_stretched(dst: &mut RgbaImage, src: &RgbaImage, rect: RectPx) {
    if rect.w <= 0 || rect.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    for row in 0..rect.h as u32 {
        let sy = row * src.height() / rect.h as u32;
        for col in 0..rect.w as u32 {
            let sx = col * src.width() / rect.w as u32;
            let si = ((sy * src.width() + sx) * 4) as usize;
            let dx = rect.x + col as i32;
            let dy = rect.y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width() || dy as u32 >= dst.height() {
                continue;
            }
            let raw = src.as_raw();
            if raw[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 从源图矩形拷到目标（宽度 1:1；高度按 `dst_h` 对齐，同高则不拉伸）。
fn blit_src_cols(
    dst: &mut RgbaImage,
    src: &RgbaImage,
    src_x: u32,
    src_w: u32,
    dst_x: i32,
    dst_y: i32,
    dst_h: i32,
) {
    if src_w == 0 || dst_h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    let src_x = src_x.min(src.width().saturating_sub(1));
    let src_w = src_w.min(src.width().saturating_sub(src_x));
    let sh = src.height();
    for row in 0..dst_h as u32 {
        let sy = if dst_h as u32 == sh {
            row
        } else {
            row * sh / dst_h as u32
        };
        let dy = dst_y + row as i32;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..src_w {
            let dx = dst_x + col as i32;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let si = ((sy * src.width() + (src_x + col)) * 4) as usize;
            let raw = src.as_raw();
            if raw[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 命令条钮右侧空轨：用 `lspacer` 轨身（跳过左接头）裁切/密铺成深色凹槽，禁止整图拉伸。
///
/// 零售 1024 宽时空隙小于轨身，表现为一段黑灰金属槽；更宽分辨率则循环中段，保留上下细轨。
fn blit_lspacer_gap(dst: &mut RgbaImage, src: &RgbaImage, gap: RectPx) {
    if gap.w <= 0 || gap.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    // 左端约 0..24 为接头装饰，轨身从其后开始。
    const BODY_X: u32 = 24;
    let body_x = BODY_X.min(src.width().saturating_sub(1));
    let body_w = src.width().saturating_sub(body_x).max(1);
    let first = (gap.w as u32).min(body_w);
    blit_src_cols(dst, src, body_x, first, gap.x, gap.y, gap.h);
    let mut written = first as i32;
    if written >= gap.w {
        return;
    }
    // 超出轨身时循环中段（避免回到左接头）。
    let tile_w = body_w.min(96).max(1);
    let tile_x = body_x + (body_w - tile_w) / 2;
    while written < gap.w {
        let chunk = ((gap.w - written) as u32).min(tile_w);
        blit_src_cols(
            dst,
            src,
            tile_x,
            chunk,
            gap.x + written,
            gap.y,
            gap.h,
        );
        written += chunk as i32;
    }
}

/// 侧栏主 chrome：与槽同尺寸则 1:1，否则最近邻铺满。
fn blit_chrome_slot(dst: &mut RgbaImage, src: &RgbaImage, slot: RectPx) {
    if slot.w <= 0 || slot.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    if src.width() as i32 == slot.w && src.height() as i32 == slot.h {
        blit_rgba(dst, src, slot.x, slot.y);
    } else {
        blit_stretched(dst, src, slot);
    }
}

/// 钮面优先按 SHP 画布原尺寸居中贴入命中格；仅当源图大于格时才拉伸，避免变形。
fn blit_button_in_cell(dst: &mut RgbaImage, src: &RgbaImage, cell: RectPx) {
    if cell.w <= 0 || cell.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    if sw <= cell.w && sh <= cell.h {
        let x = cell.x + (cell.w - sw) / 2;
        let y = cell.y + (cell.h - sh) / 2;
        blit_rgba(dst, src, x, y);
    } else {
        blit_stretched(dst, src, cell);
    }
}

fn fill_rect(dst: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    for y in rect.y..rect.y + rect.h {
        if y < 0 {
            continue;
        }
        let y = y as u32;
        if y >= dst.height() {
            break;
        }
        for x in rect.x..rect.x + rect.w {
            if x < 0 {
                continue;
            }
            let x = x as u32;
            if x >= dst.width() {
                break;
            }
            let di = ((y * dst.width() + x) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&rgba);
        }
    }
}

fn sample_opaque_rgb(img: &RgbaImage) -> Option<[u8; 4]> {
    let raw = img.as_raw();
    let mut sr = 0u64;
    let mut sg = 0u64;
    let mut sb = 0u64;
    let mut n = 0u64;
    let mut i = 0usize;
    while i + 4 <= raw.len() {
        if raw[i + 3] > 200 {
            sr += raw[i] as u64;
            sg += raw[i + 1] as u64;
            sb += raw[i + 2] as u64;
            n += 1;
        }
        i += 4;
    }
    if n == 0 {
        return None;
    }
    Some([(sr / n) as u8, (sg / n) as u8, (sb / n) as u8, 255])
}

/// 把已解码 chrome 画进透明页（左战术区保持透明，供地图透出）。
pub fn blit_battle_hud_chrome(
    page: &mut RgbaImage,
    chrome: &BattleHudChrome,
    snap: &LayoutSnapshot,
    power_meter_w: i32,
) {
    blit_battle_hud_chrome_with_state(page, chrome, snap, power_meter_w, None);
}

/// 带命令条按下态绘制。
pub fn blit_battle_hud_chrome_with_state(
    page: &mut RgbaImage,
    chrome: &BattleHudChrome,
    snap: &LayoutSnapshot,
    power_meter_w: i32,
    command_pressed: Option<usize>,
) {
    blit_battle_hud_chrome_ex(page, chrome, snap, power_meter_w, command_pressed, false);
}

/// `pause_menu == true`：不画修理/出售/页签/选项外交/命令钮，底边只留端盖+`lspacer` 轨。
pub fn blit_battle_hud_chrome_ex(
    page: &mut RgbaImage,
    chrome: &BattleHudChrome,
    snap: &LayoutSnapshot,
    power_meter_w: i32,
    command_pressed: Option<usize>,
    pause_menu: bool,
) {
    let sidebar = rect_px_from_snapshot(snap, "sidebar");
    let credits = rect_px_from_snapshot(snap, "credits");
    let top = rect_px_from_snapshot(snap, "top");
    let radar = rect_px_from_snapshot(snap, "radar");
    let side1 = rect_px_from_snapshot(snap, "side1");
    let cameo_band = rect_px_from_snapshot(snap, "cameo_band");
    let side3 = rect_px_from_snapshot(snap, "side3");
    let addon = rect_px_from_snapshot(snap, "addon");
    let repair = rect_px_from_snapshot(snap, "repair");
    let sell = rect_px_from_snapshot(snap, "sell");
    let bottom_strip = rect_px_from_snapshot(snap, "bottom_strip");
    let opt_btn = rect_px_from_snapshot(snap, "opt_btn");
    let diplo_btn = rect_px_from_snapshot(snap, "diplo_btn");
    let tabs = [
        rect_px_from_snapshot(snap, "tab00"),
        rect_px_from_snapshot(snap, "tab01"),
        rect_px_from_snapshot(snap, "tab02"),
        rect_px_from_snapshot(snap, "tab03"),
    ];

    let sidebar_fill = chrome
        .side2
        .as_ref()
        .or(chrome.side1.as_ref())
        .or(chrome.top.as_ref())
        .and_then(|s| sample_opaque_rgb(&s.image))
        .unwrap_or([40, 44, 52, 255]);
    fill_rect(page, sidebar, sidebar_fill);

    // chrome 主件与画布同尺寸时 1:1 贴，禁止无故拉伸把金属高光揉糊。
    if let Some(s) = &chrome.credits {
        blit_chrome_slot(page, &s.image, credits);
    }
    if let Some(s) = &chrome.top {
        blit_chrome_slot(page, &s.image, top);
    }
    if let Some(s) = &chrome.radar {
        blit_chrome_slot(page, &s.image, radar);
    }
    if let Some(s) = &chrome.side1 {
        blit_chrome_slot(page, &s.image, side1);
    }
    if let Some(tile) = &chrome.side2 {
        let th = tile.image.height().max(1) as i32;
        let mut y = cameo_band.y;
        while y < cameo_band.y + cameo_band.h {
            let remain = cameo_band.y + cameo_band.h - y;
            let h = remain.min(th);
            blit_stretched(page, &tile.image, RectPx::new(cameo_band.x, y, cameo_band.w, h));
            y += th;
        }
    }
    // 底脚只在右栏内铺色，禁止横贯战术区。
    let bottom_fill = chrome
        .addon
        .as_ref()
        .or(chrome.side3.as_ref())
        .and_then(|s| sample_opaque_rgb(&s.image))
        .unwrap_or(sidebar_fill);
    fill_rect(page, bottom_strip, bottom_fill);
    if let Some(s) = &chrome.side3 {
        blit_chrome_slot(page, &s.image, side3);
    }
    if let Some(s) = &chrome.addon {
        blit_chrome_slot(page, &s.image, addon);
    }
    if !pause_menu {
        if let Some(s) = &chrome.repair {
            blit_button_in_cell(page, &s.image, repair);
        }
        if let Some(s) = &chrome.sell {
            blit_button_in_cell(page, &s.image, sell);
        }
    }
    if let Some(s) = &chrome.powerp {
        // `powerp.shp` 为窄条带，沿 cameo 左缘纵向平铺成电表，勿整帧拉高。
        let meter_w = power_meter_w.min(sidebar.w).max(1);
        let strip_h = s.image.height().max(1) as i32;
        let mut y = cameo_band.y;
        let bottom = cameo_band.y + cameo_band.h;
        while y < bottom {
            let h = (bottom - y).min(strip_h);
            blit_stretched(
                page,
                &s.image,
                RectPx::new(sidebar.x, y, meter_w, h),
            );
            y += strip_h;
        }
    }
    if !pause_menu {
        // 四分类页签贴入布局槽位，勿压住修理/出售拱钮。
        for (i, tab) in chrome.tabs.iter().enumerate() {
            if let Some(tab) = tab {
                blit_button_in_cell(page, &tab.image, tabs[i]);
            }
        }
        // 顶栏双钮：贴在 `top.shp` 凹槽（资金条与雷达之间）。
        if let Some(s) = &chrome.diplobtn {
            blit_button_in_cell(page, &s.image, diplo_btn);
        }
        if let Some(s) = &chrome.optbtn {
            blit_button_in_cell(page, &s.image, opt_btn);
        }
        blit_command_bar(page, chrome, snap, command_pressed);
    } else {
        // 暂停：整段命令轨保留金属细节，不露编队/部署钮。
        blit_command_bar_track(page, chrome, snap, /*with_buttons*/ false, None);
    }
}

fn command_bar_shp_index_for_visual(visual_slot: usize) -> Option<usize> {
    let name = *SKIRMISH_COMMAND_BAR.get(visual_slot)?;
    command_bar_shp_index(name)
}

fn blit_command_bar(
    page: &mut RgbaImage,
    chrome: &BattleHudChrome,
    snap: &LayoutSnapshot,
    pressed_slot: Option<usize>,
) {
    blit_command_bar_track(page, chrome, snap, /*with_buttons*/ true, pressed_slot);
}

/// 底边命令条：端盖 + `lspacer` 轨身（白顶/红底细线）。
///
/// `with_buttons == false` 时整段钮槽铺轨身，供暂停态藏起编队/部署等命令钮，且不毁掉金属轨细节。
pub fn blit_command_bar_track(
    page: &mut RgbaImage,
    chrome: &BattleHudChrome,
    snap: &LayoutSnapshot,
    with_buttons: bool,
    pressed_slot: Option<usize>,
) {
    let bar = rect_px_from_snapshot(snap, "command_bar");
    if bar.w <= 0 || bar.h <= 0 {
        return;
    }
    // 只垫不透明底，轨身细线由 `lspacer`/`lendcap`/`rendcap` 画，禁止靠纯黑冒充。
    fill_rect(page, bar, [0, 0, 0, 255]);

    let lendcap = rect_px_from_snapshot(snap, "lendcap");
    let rendcap = rect_px_from_snapshot(snap, "rendcap");
    if let Some(s) = &chrome.lendcap {
        blit_button_in_cell(page, &s.image, lendcap);
    }

    let track_left = lendcap.x + lendcap.w;
    let track_right = rendcap.x;
    if with_buttons {
        let mut last_btn_right = track_left;
        for (visual, id) in COMMAND_BAR_BUTTON_IDS.iter().enumerate() {
            let cell = rect_px_from_snapshot(snap, id);
            if cell.w <= 0 {
                continue;
            }
            let Some(shp_i) = command_bar_shp_index_for_visual(visual) else {
                continue;
            };
            let Some(normal) = chrome.command_buttons.get(shp_i).and_then(|s| s.as_ref()) else {
                continue;
            };
            let sprite = if pressed_slot == Some(visual) {
                chrome
                    .command_buttons_pressed
                    .get(shp_i)
                    .and_then(|s| s.as_ref())
                    .unwrap_or(normal)
            } else {
                normal
            };
            blit_button_in_cell(page, &sprite.image, cell);
            last_btn_right = cell.x + cell.w;
        }
        let gap_w = (track_right - last_btn_right).max(0);
        if gap_w > 0 {
            if let Some(s) = &chrome.lspacer {
                blit_lspacer_gap(page, &s.image, RectPx::new(last_btn_right, bar.y, gap_w, bar.h));
            }
        }
    } else {
        let gap_w = (track_right - track_left).max(0);
        if gap_w > 0 {
            if let Some(s) = &chrome.lspacer {
                blit_lspacer_gap(page, &s.image, RectPx::new(track_left, bar.y, gap_w, bar.h));
            }
        }
    }

    if let Some(s) = &chrome.rendcap {
        blit_button_in_cell(page, &s.image, rendcap);
    }
}

/// 便捷：按视口与 chrome 嵌套包度量生成布局并绘制。
pub fn paint_battle_hud_chrome(page: &mut RgbaImage, chrome: &BattleHudChrome) {
    let metrics = BattleHudChromeMetrics::for_mix(&chrome.mix);
    let snap = solve_battle_hud_with_metrics(page.width(), page.height(), metrics);
    blit_battle_hud_chrome(page, chrome, &snap, metrics.power_w);
}

/// 对局 HUD 可点入口（几何权威为 `solve_battle_hud` snapshot）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleHudHit {
    /// 修理模式。
    Repair,
    /// 出售模式。
    Sell,
    /// 选项。
    Options,
    /// 外交。
    Diplomacy,
    /// 底边命令条可视槽（`cmdN` / `ButtonList` 下标）。
    CommandButton(usize),
    /// 分类页签（0=建筑 / 1=步兵 / 2=载具 / 3=飞行器）。
    SidebarTab(usize),
    /// 当前页可视 cameo 槽。
    Cameo(usize),
}

impl BattleHudHit {
    /// 由 snapshot 控件 id 解析。
    pub fn from_entry_id(id: &str) -> Option<Self> {
        match id {
            "repair" => Some(Self::Repair),
            "sell" => Some(Self::Sell),
            "opt_btn" => Some(Self::Options),
            "diplo_btn" => Some(Self::Diplomacy),
            "tab00" => Some(Self::SidebarTab(0)),
            "tab01" => Some(Self::SidebarTab(1)),
            "tab02" => Some(Self::SidebarTab(2)),
            "tab03" => Some(Self::SidebarTab(3)),
            _ => {
                if let Some(rest) = id.strip_prefix("cmd") {
                    if let Ok(slot) = rest.parse::<usize>() {
                        if slot < COMMAND_BAR_BUTTON_COUNT {
                            return Some(Self::CommandButton(slot));
                        }
                    }
                }
                None
            }
        }
    }

    /// 稳定入口 id。
    pub fn entry_id(self) -> &'static str {
        match self {
            Self::Repair => "repair",
            Self::Sell => "sell",
            Self::Options => "opt_btn",
            Self::Diplomacy => "diplo_btn",
            Self::SidebarTab(0) => "tab00",
            Self::SidebarTab(1) => "tab01",
            Self::SidebarTab(2) => "tab02",
            Self::SidebarTab(3) => "tab03",
            Self::SidebarTab(_) => "tab00",
            Self::Cameo(_) => "cameo_band",
            Self::CommandButton(slot) => COMMAND_BAR_BUTTON_IDS
                .get(slot)
                .copied()
                .unwrap_or(COMMAND_BAR_BUTTON_IDS[0]),
        }
    }
}

const BATTLE_HUD_HIT_IDS: [&str; 8] = [
    "repair",
    "sell",
    "opt_btn",
    "diplo_btn",
    "tab00",
    "tab01",
    "tab02",
    "tab03",
];

/// 视口像素命中（侧栏与命令条均走 snapshot）。
pub fn hit_at(snap: &LayoutSnapshot, x: i32, y: i32) -> Option<BattleHudHit> {
    hit_at_with_chrome(snap, None, BattleHudChromeMetrics::sidec01().power_w, 0, x, y)
}

/// 带 chrome / cameo 槽数的命中。
pub fn hit_at_with_chrome(
    snap: &LayoutSnapshot,
    chrome: Option<&BattleHudChrome>,
    power_meter_w: i32,
    cameo_count: usize,
    x: i32,
    y: i32,
) -> Option<BattleHudHit> {
    if let Some(chrome) = chrome {
        let bar = rect_px_from_snapshot(snap, "command_bar");
        if bar.contains(x, y) {
            let point = Point2 {
                x: x as f32,
                y: y as f32,
            };
            for (visual, id) in COMMAND_BAR_BUTTON_IDS.iter().enumerate() {
                let Some(shp_i) = command_bar_shp_index_for_visual(visual) else {
                    continue;
                };
                if chrome.command_buttons.get(shp_i).and_then(|s| s.as_ref()).is_none() {
                    continue;
                }
                if snap
                    .get(id)
                    .is_some_and(|el| el.layout.rect.width > 0.0 && el.layout.rect.contains(point))
                {
                    return Some(BattleHudHit::CommandButton(visual));
                }
            }
            // 点在命令条空白（金属轨）上仍吞掉，避免穿透到地图手势。
            return None;
        }
    }

    let point = Point2 {
        x: x as f32,
        y: y as f32,
    };
    for id in BATTLE_HUD_HIT_IDS {
        if snap
            .get(id)
            .is_some_and(|el| el.layout.rect.contains(point))
        {
            return BattleHudHit::from_entry_id(id);
        }
    }

    let band = rect_px_from_snapshot(snap, "cameo_band");
    if let Some(slot) = hit_cameo_slot(band, power_meter_w, x, y) {
        if slot < cameo_count {
            return Some(BattleHudHit::Cameo(slot));
        }
        // 点在空 cameo 槽仍吞掉，避免误清选中。
        return None;
    }
    if band.contains(x, y) {
        return None;
    }
    let _ = SIDEBAR_TAB_COUNT;
    None
}

/// 单枚建造栏图标绘制描述。
#[derive(Debug, Clone, Copy)]
pub struct BattleCameoPaint<'a> {
    /// 规则类型键。
    pub type_id: &'a str,
    /// 已解码图标（缺图时画占位）。
    pub image: Option<&'a RgbaImage>,
    /// 当前是否可下单。
    pub enabled: bool,
    /// 是否处于放置选中态。
    pub selected: bool,
}

/// 将 cameo 列表画进侧栏内容区。
pub fn blit_battle_cameos(
    page: &mut RgbaImage,
    snap: &LayoutSnapshot,
    power_meter_w: i32,
    cameos: &[BattleCameoPaint<'_>],
) {
    let band = rect_px_from_snapshot(snap, "cameo_band");
    for (slot, item) in cameos.iter().enumerate() {
        let Some(cell) = cameo_slot_rect(band, power_meter_w, slot) else {
            break;
        };
        if let Some(img) = item.image {
            blit_button_in_cell(page, img, cell);
        } else {
            fill_rect(page, cell, [24, 28, 36, 255]);
        }
        if !item.enabled {
            fill_rect_alpha(page, cell, [0, 0, 0, 120]);
        }
        if item.selected {
            stroke_rect(page, cell, [255, 220, 64, 255]);
        }
    }
}

/// 按 `art.ini` 的 `Cameo=`（及回退名）解码建造栏图标。
pub fn decode_cameo_sprite(
    source: &GameAssetSource,
    art: Option<&ra_assets::IniDocument>,
    type_id: &str,
) -> Option<DecodedUiSprite> {
    let mut names = Vec::new();
    if let Some(art) = art {
        if let Some(c) = art.get(type_id, "Cameo").map(str::trim).filter(|s| !s.is_empty()) {
            names.push(format!("{c}.shp"));
        }
        let image_key = art.get(type_id, "Image").unwrap_or(type_id);
        if !image_key.eq_ignore_ascii_case(type_id) {
            if let Some(c) = art.get(image_key, "Cameo").map(str::trim).filter(|s| !s.is_empty()) {
                names.push(format!("{c}.shp"));
            }
        }
        if let Some(c) = art.get(type_id, "AltCameo").map(str::trim).filter(|s| !s.is_empty()) {
            names.push(format!("{c}.shp"));
        }
    }
    names.push(format!("{type_id}icon.shp"));
    names.push(format!("{type_id}.shp"));

    let mut last_err = None;
    for name in names {
        match decode_cameo_named(source, &name) {
            Ok(s) => return Some(s),
            Err(e) => last_err = Some(e),
        }
    }
    let _ = last_err;
    None
}

fn decode_cameo_named(source: &GameAssetSource, name: &str) -> Result<DecodedUiSprite, String> {
    let hit = source
        .resolve_preferring(name, "cameo.mix")
        .or_else(|| source.resolve(name))
        .ok_or_else(|| format!("{name}: 不可读"))?;
    let shp = ShpFile::parse(&hit.bytes).map_err(|e| format!("{name}: SHP 解析失败 · {e}"))?;
    if shp.frames.is_empty() {
        return Err(format!("{name}: SHP 无帧"));
    }
    let pal_hit = source
        .resolve_preferring("cameo.pal", "cameo.mix")
        .or_else(|| source.resolve("cameo.pal"))
        .or_else(|| source.resolve("sidebar.pal"))
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

fn fill_rect_alpha(page: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    for y in rect.y..rect.y + rect.h {
        if y < 0 || y as u32 >= page.height() {
            continue;
        }
        for x in rect.x..rect.x + rect.w {
            if x < 0 || x as u32 >= page.width() {
                continue;
            }
            let di = ((y as u32 * page.width() + x as u32) * 4) as usize;
            let sa = rgba[3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                page.as_mut()[di..di + 4].copy_from_slice(&rgba);
                continue;
            }
            let inv = 255 - sa;
            for c in 0..3 {
                let s = rgba[c] as u32;
                let d = page.as_mut()[di + c] as u32;
                page.as_mut()[di + c] = ((s * sa + d * inv) / 255) as u8;
            }
            page.as_mut()[di + 3] = 255;
        }
    }
}

fn stroke_rect(page: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    for x in rect.x..rect.x + rect.w {
        put_px(page, x, rect.y, rgba);
        put_px(page, x, rect.y + rect.h - 1, rgba);
    }
    for y in rect.y..rect.y + rect.h {
        put_px(page, rect.x, y, rgba);
        put_px(page, rect.x + rect.w - 1, y, rgba);
    }
}

fn put_px(page: &mut RgbaImage, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x as u32 >= page.width() || y as u32 >= page.height() {
        return;
    }
    let di = ((y as u32 * page.width() + x as u32) * 4) as usize;
    page.as_mut()[di..di + 4].copy_from_slice(&rgba);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_layout::{
        cameo_slot_rect, rect_px_from_snapshot, solve_battle_hud_with_metrics, BattleHudChromeMetrics,
    };

    #[test]
    fn hit_tabs_and_cameo_slots() {
        let metrics = BattleHudChromeMetrics::sidec01();
        let snap = solve_battle_hud_with_metrics(800, 600, metrics);
        let tab0 = rect_px_from_snapshot(&snap, "tab00");
        assert_eq!(
            hit_at_with_chrome(&snap, None, metrics.power_w, 4, tab0.x + 1, tab0.y + 1),
            Some(BattleHudHit::SidebarTab(0))
        );
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let cell = cameo_slot_rect(band, metrics.power_w, 0).expect("slot0");
        assert_eq!(
            hit_at_with_chrome(&snap, None, metrics.power_w, 2, cell.x + 1, cell.y + 1),
            Some(BattleHudHit::Cameo(0))
        );
        assert_eq!(
            hit_at_with_chrome(&snap, None, metrics.power_w, 0, cell.x + 1, cell.y + 1),
            None,
            "空列表时 cameo 槽应吞掉点击"
        );
    }
}
