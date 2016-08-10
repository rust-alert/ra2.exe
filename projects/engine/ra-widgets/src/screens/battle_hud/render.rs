//! 对局侧栏 chrome：按已解析 [`UiFactionChrome`] 从 `sidecNN` 解码并合成。
//!
//! 文件名与菜单壳层分离；同名 SHP 靠 `MixFileIndex` 嵌套包区分外观。
//! 战术区铺到命令条顶边；chrome 含右侧栏与底边命令条。

use ra_layout::{
    BattleHudChromeMetrics, COMMAND_BAR_BUTTON_IDS, LayoutSnapshot, RectPx, SIDEBAR_TAB_COUNT, cameo_slot_rect, rect_px_from_snapshot,
    solve_battle_hud_with_metrics,
};
use ra_renderer::RgbaImage;

use super::{
    chrome::BattleHudChrome, command_bar::command_bar_shp_index_for_visual, decode::RADAR_OPEN_FRAME_TICKS, hit_test::BattleCameoPaint,
};

pub(super) fn blit_rgba(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
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

pub(super) fn blit_stretched(dst: &mut RgbaImage, src: &RgbaImage, rect: RectPx) {
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
pub(super) fn blit_src_cols(dst: &mut RgbaImage, src: &RgbaImage, src_x: u32, src_w: u32, dst_x: i32, dst_y: i32, dst_h: i32) {
    if src_w == 0 || dst_h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    let src_x = src_x.min(src.width().saturating_sub(1));
    let src_w = src_w.min(src.width().saturating_sub(src_x));
    let sh = src.height();
    for row in 0..dst_h as u32 {
        let sy = if dst_h as u32 == sh { row } else { row * sh / dst_h as u32 };
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
pub(super) fn blit_lspacer_gap(dst: &mut RgbaImage, src: &RgbaImage, gap: RectPx) {
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
        blit_src_cols(dst, src, tile_x, chunk, gap.x + written, gap.y, gap.h);
        written += chunk as i32;
    }
}

/// 侧栏主 chrome：与槽同尺寸则 1:1，否则最近邻铺满。
pub(super) fn blit_chrome_slot(dst: &mut RgbaImage, src: &RgbaImage, slot: RectPx) {
    if slot.w <= 0 || slot.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    if src.width() as i32 == slot.w && src.height() as i32 == slot.h {
        blit_rgba(dst, src, slot.x, slot.y);
    }
    else {
        blit_stretched(dst, src, slot);
    }
}

/// 钮面优先按 SHP 画布原尺寸居中贴入命中格；仅当源图大于格时才拉伸，避免变形。
pub(super) fn blit_button_in_cell(dst: &mut RgbaImage, src: &RgbaImage, cell: RectPx) {
    if cell.w <= 0 || cell.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    if sw <= cell.w && sh <= cell.h {
        let x = cell.x + (cell.w - sw) / 2;
        let y = cell.y + (cell.h - sh) / 2;
        blit_rgba(dst, src, x, y);
    }
    else {
        blit_stretched(dst, src, cell);
    }
}

pub(super) fn fill_rect(dst: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
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

pub(super) fn sample_opaque_rgb(img: &RgbaImage) -> Option<[u8; 4]> {
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
pub fn blit_battle_hud_chrome(page: &mut RgbaImage, chrome: &BattleHudChrome, snap: &LayoutSnapshot, power_meter_w: i32) {
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
    blit_battle_hud_chrome_ex(page, chrome, snap, power_meter_w, command_pressed, false, false, false, false, 0, [true; SIDEBAR_TAB_COUNT], 0);
}

/// `pause_menu == true`：不画修理/出售/页签/选项外交/命令钮，底边只留端盖+`lspacer` 轨。
///
/// `tabs_visible`：无对应可建造基础的分类页签不绘制。
///
/// `active_tab`：当前侧栏页签（贴 `tabNN` 按下帧）。
///
/// `repair_active` / `sell_active`：侧栏工具切换态，贴 `repair`/`sell` 按下帧。
///
/// `radar_online`：本机有电且有雷达时播开图帧，否则贴关图徽。
pub fn blit_battle_hud_chrome_ex(
    page: &mut RgbaImage,
    chrome: &BattleHudChrome,
    snap: &LayoutSnapshot,
    power_meter_w: i32,
    command_pressed: Option<usize>,
    pause_menu: bool,
    repair_active: bool,
    sell_active: bool,
    radar_online: bool,
    tick: u64,
    tabs_visible: [bool; SIDEBAR_TAB_COUNT],
    active_tab: usize,
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
    let radar_sprite = if radar_online && !chrome.radar_open.is_empty() {
        let idx = ((tick / RADAR_OPEN_FRAME_TICKS) as usize) % chrome.radar_open.len();
        chrome.radar_open.get(idx).or(chrome.radar.as_ref())
    }
    else {
        chrome.radar.as_ref()
    };
    if let Some(s) = radar_sprite {
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
    let bottom_fill = chrome.addon.as_ref().or(chrome.side3.as_ref()).and_then(|s| sample_opaque_rgb(&s.image)).unwrap_or(sidebar_fill);
    fill_rect(page, bottom_strip, bottom_fill);
    if let Some(s) = &chrome.side3 {
        blit_chrome_slot(page, &s.image, side3);
    }
    if let Some(s) = &chrome.addon {
        blit_chrome_slot(page, &s.image, addon);
    }
    if !pause_menu {
        let repair_sprite = if repair_active { chrome.repair_pressed.as_ref().or(chrome.repair.as_ref()) } else { chrome.repair.as_ref() };
        if let Some(s) = repair_sprite {
            blit_button_in_cell(page, &s.image, repair);
        }
        let sell_sprite = if sell_active { chrome.sell_pressed.as_ref().or(chrome.sell.as_ref()) } else { chrome.sell.as_ref() };
        if let Some(s) = sell_sprite {
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
            blit_stretched(page, &s.image, RectPx::new(sidebar.x, y, meter_w, h));
            y += strip_h;
        }
    }
    if !pause_menu {
        // 四分类页签贴入布局槽位，勿压住修理/出售拱钮。
        for (i, tab) in chrome.tabs.iter().enumerate() {
            if !tabs_visible.get(i).copied().unwrap_or(false) {
                continue;
            }
            let pressed = i == active_tab;
            let sprite = if pressed { chrome.tabs_pressed.get(i).and_then(|t| t.as_ref()).or(tab.as_ref()) } else { tab.as_ref() };
            if let Some(tab) = sprite {
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
    }
    else {
        // 暂停：整段命令轨保留金属细节，不露编队/部署钮。
        blit_command_bar_track(page, chrome, snap, /* with_buttons */ false, None);
    }
}

/// 便捷：按视口与 chrome 嵌套包度量生成布局并绘制。
pub fn paint_battle_hud_chrome(page: &mut RgbaImage, chrome: &BattleHudChrome) {
    let metrics = BattleHudChromeMetrics::for_mix(&chrome.mix);
    let snap = solve_battle_hud_with_metrics(page.width(), page.height(), metrics);
    blit_battle_hud_chrome(page, chrome, &snap, metrics.power_w);
}

/// 将 cameo 列表画进侧栏内容区。
///
/// `tick` 用于完工待放 cameo 的闪烁相位。
pub fn blit_battle_cameos(page: &mut RgbaImage, snap: &LayoutSnapshot, power_meter_w: i32, cameos: &[BattleCameoPaint<'_>], tick: u64) {
    let band = rect_px_from_snapshot(snap, "cameo_band");
    for (slot, item) in cameos.iter().enumerate() {
        let Some(cell) = cameo_slot_rect(band, power_meter_w, slot)
        else {
            break;
        };
        if let Some(img) = item.image {
            blit_button_in_cell(page, img, cell);
        }
        else {
            fill_rect(page, cell, [24, 28, 36, 255]);
        }
        if let Some(progress) = item.progress {
            paint_cameo_progress_clock(page, cell, progress.clamp(0.0, 1.0));
        }
        let ready = item.progress.is_some_and(|p| p >= 1.0);
        if ready && cameo_ready_flash_on(tick) {
            fill_rect_alpha(page, cell, [255, 255, 160, 72]);
        }
        if !item.enabled {
            fill_rect_alpha(page, cell, [0, 0, 0, 120]);
        }
        if item.selected {
            let stroke = if ready && cameo_ready_flash_on(tick) { [255, 255, 120, 255] } else { [220, 220, 80, 220] };
            stroke_rect(page, cell, stroke);
        }
    }
}

/// 完工待放闪烁：约每 8 逻辑 tick 亮/灭交替。
pub fn cameo_ready_flash_on(tick: u64) -> bool {
    (tick / 8) % 2 == 0
}

/// 原版风格时钟擦除：从 12 点顺时针揭开，未完成扇区半透明压暗。
pub fn paint_cameo_progress_clock(page: &mut RgbaImage, cell: RectPx, progress: f32) {
    if progress >= 1.0 || cell.w <= 0 || cell.h <= 0 {
        return;
    }
    let cx = cell.x as f32 + cell.w as f32 * 0.5;
    let cy = cell.y as f32 + cell.h as f32 * 0.5;
    let dark = [0u8, 0, 0, 150];
    for y in cell.y..cell.y + cell.h {
        if y < 0 || y as u32 >= page.height() {
            continue;
        }
        for x in cell.x..cell.x + cell.w {
            if x < 0 || x as u32 >= page.width() {
                continue;
            }
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            // 0 = 正上，顺时针增大到 1。
            let ang = dx.atan2(-dy);
            let mut frac = ang / (std::f32::consts::TAU);
            if frac < 0.0 {
                frac += 1.0;
            }
            if frac >= progress {
                blend_px(page, x, y, dark);
            }
        }
    }
}

pub(super) fn fill_rect_alpha(page: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
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

pub(super) fn stroke_rect(page: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
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

pub(super) fn blend_px(page: &mut RgbaImage, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x as u32 >= page.width() || y as u32 >= page.height() {
        return;
    }
    let di = ((y as u32 * page.width() + x as u32) * 4) as usize;
    let sa = rgba[3] as u32;
    if sa == 0 {
        return;
    }
    if sa == 255 {
        page.as_mut()[di..di + 4].copy_from_slice(&rgba);
        return;
    }
    let inv = 255 - sa;
    for c in 0..3 {
        let s = rgba[c] as u32;
        let d = page.as_mut()[di + c] as u32;
        page.as_mut()[di + c] = ((s * sa + d * inv) / 255) as u8;
    }
    page.as_mut()[di + 3] = 255;
}

pub(super) fn put_px(page: &mut RgbaImage, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x as u32 >= page.width() || y as u32 >= page.height() {
        return;
    }
    let di = ((y as u32 * page.width() + x as u32) * 4) as usize;
    page.as_mut()[di..di + 4].copy_from_slice(&rgba);
}

fn blit_command_bar(page: &mut RgbaImage, chrome: &BattleHudChrome, snap: &LayoutSnapshot, pressed_slot: Option<usize>) {
    blit_command_bar_track(page, chrome, snap, /* with_buttons */ true, pressed_slot);
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
            let Some(shp_i) = command_bar_shp_index_for_visual(visual)
            else {
                continue;
            };
            let Some(normal) = chrome.command_buttons.get(shp_i).and_then(|s| s.as_ref())
            else {
                continue;
            };
            let sprite = if pressed_slot == Some(visual) {
                chrome.command_buttons_pressed.get(shp_i).and_then(|s| s.as_ref()).unwrap_or(normal)
            }
            else {
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
    }
    else {
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
