//! 原版主 UI 资产接线前的菜单占位绘制（色块按钮）。
//!
//! **不是 Pre-Alpha 交付。** 色块可点导航只证明页面状态机与输入接线，
//! 不能替代原版 SHP/字体/布局与截图对照验收。正式复刻见产品主菜单路径。
//!
//! 命中框与入口 id 以 [`crate::ui_slots`] 为单一来源。

use ra_renderer::RgbaImage;

use crate::{
    boot::BootMapCandidate,
    screen::OriginalScreen,
    ui_slots::{UiPageSlots, slots_for},
};

/// 菜单上的一个可点区域（窗口归一化坐标 0..1）。
#[derive(Debug, Clone, Copy)]
pub struct MenuHit {
    /// 逻辑入口 id。
    pub entry_id: &'static str,
    /// 动作标识。
    pub action: MenuAction,
    /// 左。
    pub x0: f32,
    /// 上。
    pub y0: f32,
    /// 右。
    pub x1: f32,
    /// 下。
    pub y1: f32,
    /// 是否可点。
    pub enabled: bool,
}

/// 占位菜单导航动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// 进入单人页。
    OpenSinglePlayer,
    /// 网络（禁用）。
    OpenNetwork,
    /// 选项。
    OpenOptions,
    /// 退出。
    Exit,
    /// 进入遭遇战大厅。
    OpenSkirmish,
    /// 返回上一级。
    Back,
    /// 开始装载遭遇战。
    StartSkirmish,
    /// 取消进行中的遭遇战装载。
    CancelLoad,
    /// 选中大厅地图列表中的一项。
    SelectMap(usize),
}

/// 某页的占位布局：底图 + 命中区。
pub struct MenuLayout {
    /// 上传用底图。
    pub image: RgbaImage,
    /// 点击命中。
    pub hits: Vec<MenuHit>,
}

impl MenuLayout {
    /// 窗口像素点击 → 动作。
    pub fn hit(&self, cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<MenuAction> {
        self.hit_at(cursor_x, cursor_y, win_w, win_h).map(|(_, action)| action)
    }

    /// 命中命中区下标（仅 `enabled`）。
    pub fn hover_index(&self, cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<usize> {
        self.hit_at(cursor_x, cursor_y, win_w, win_h).map(|(i, _)| i)
    }

    fn hit_at(&self, cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<(usize, MenuAction)> {
        if win_w <= 0.0 || win_h <= 0.0 {
            return None;
        }
        let nx = (cursor_x / win_w) as f32;
        let ny = (cursor_y / win_h) as f32;
        for (i, h) in self.hits.iter().enumerate() {
            if !h.enabled {
                continue;
            }
            if nx >= h.x0 && nx <= h.x1 && ny >= h.y0 && ny <= h.y1 {
                return Some((i, h.action));
            }
        }
        None
    }
}

/// 为当前原版产品页生成占位布局；对局/结算页返回 `None`。
pub fn layout_for(
    screen: OriginalScreen,
    width: u32,
    height: u32,
    hover: Option<usize>,
    pressed: Option<usize>,
) -> Option<MenuLayout> {
    let (w, h) = (width.max(320), height.max(240));
    let page = slots_for(screen)?;
    Some(paint_from_slots(w, h, &page, hover, pressed))
}

/// 遭遇战大厅：地图列表 + Start/Back 槽位。
pub fn layout_skirmish_lobby(
    width: u32,
    height: u32,
    maps: &[BootMapCandidate],
    selected: Option<&str>,
    hover: Option<usize>,
    pressed: Option<usize>,
) -> MenuLayout {
    let (w, h) = (width.max(320), height.max(240));
    let mut pixels = vec![0u8; (w as usize) * (h as usize) * 4];
    fill_rect(&mut pixels, w, h, 0, 0, w, h, [12, 18, 36, 255]);
    fill_rect(&mut pixels, w, h, 0, 0, w, h / 10, [28, 40, 72, 255]);

    let mut hits = Vec::new();
    let list_top = 0.18_f32;
    let row_h = 0.07_f32;
    for (i, map) in maps.iter().enumerate().take(6) {
        let y0 = list_top + (i as f32) * (row_h + 0.015);
        let y1 = y0 + row_h;
        let x0 = 0.18_f32;
        let x1 = 0.82_f32;
        let selected_row = selected == Some(map.file_name.as_str());
        let hovered = hover == Some(i);
        let is_pressed = pressed == Some(i);
        let color = if is_pressed {
            [24, 48, 36, 255]
        }
        else if selected_row {
            [64, 120, 72, 255]
        }
        else if hovered {
            [72, 110, 180, 255]
        }
        else {
            [40, 70, 120, 255]
        };
        let px0 = (x0 * w as f32) as u32;
        let py0 = (y0 * h as f32) as u32;
        let px1 = (x1 * w as f32) as u32;
        let py1 = (y1 * h as f32) as u32;
        fill_rect(
            &mut pixels,
            w,
            h,
            px0,
            py0,
            px1.saturating_sub(px0),
            py1.saturating_sub(py0),
            color,
        );
        if selected_row || hovered {
            fill_rect(
                &mut pixels,
                w,
                h,
                px0,
                py0,
                8,
                py1.saturating_sub(py0),
                [220, 180, 64, 255],
            );
        }
        hits.push(MenuHit {
            entry_id: "map",
            action: MenuAction::SelectMap(i),
            x0,
            y0,
            x1,
            y1,
            enabled: true,
        });
    }

    let map_hit_count = hits.len();
    // Start / Back 沿用槽位矩形，叠在列表下方。
    if let Some(page) = slots_for(OriginalScreen::SkirmishLobby) {
        for (i, btn) in page.buttons.iter().enumerate() {
            let hit_i = map_hit_count + i;
            let (x0, y0, x1, y1) = btn.hit;
            let hovered = hover == Some(hit_i);
            let color = button_color(btn.enabled, i, hovered, pressed == Some(hit_i));
            let px0 = (x0 * w as f32) as u32;
            let py0 = (y0 * h as f32) as u32;
            let px1 = (x1 * w as f32) as u32;
            let py1 = (y1 * h as f32) as u32;
            fill_rect(
                &mut pixels,
                w,
                h,
                px0,
                py0,
                px1.saturating_sub(px0),
                py1.saturating_sub(py0),
                color,
            );
            if btn.enabled {
                fill_rect(
                    &mut pixels,
                    w,
                    h,
                    px0,
                    py0,
                    6,
                    py1.saturating_sub(py0),
                    [220, 180, 64, 255],
                );
            }
            hits.push(MenuHit {
                entry_id: btn.entry_id,
                action: btn.action,
                x0,
                y0,
                x1,
                y1,
                enabled: btn.enabled,
            });
        }
    }

    let image = RgbaImage::new(w, h, pixels).expect("lobby image size");
    MenuLayout { image, hits }
}

fn button_color(enabled: bool, index: usize, hovered: bool, pressed: bool) -> [u8; 4] {
    if !enabled {
        return [40, 40, 48, 255];
    }
    if pressed {
        return [24, 48, 96, 255];
    }
    if hovered {
        return [88, 140, 220, 255];
    }
    if index % 2 == 0 {
        [48, 92, 160, 255]
    }
    else {
        [40, 78, 140, 255]
    }
}

fn paint_from_slots(
    width: u32,
    height: u32,
    page: &UiPageSlots,
    hover: Option<usize>,
    pressed: Option<usize>,
) -> MenuLayout {
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    let bg = match page.screen {
        OriginalScreen::LoadScreen => [18, 22, 40, 255],
        _ => [12, 18, 36, 255],
    };
    fill_rect(&mut pixels, width, height, 0, 0, width, height, bg);
    fill_rect(&mut pixels, width, height, 0, 0, width, height / 10, [28, 40, 72, 255]);

    let mut hits = Vec::with_capacity(page.buttons.len());
    for (i, btn) in page.buttons.iter().enumerate() {
        let (x0, y0, x1, y1) = btn.hit;
        let color = button_color(btn.enabled, i, hover == Some(i), pressed == Some(i));
        let px0 = (x0 * width as f32) as u32;
        let py0 = (y0 * height as f32) as u32;
        let px1 = (x1 * width as f32) as u32;
        let py1 = (y1 * height as f32) as u32;
        fill_rect(
            &mut pixels,
            width,
            height,
            px0,
            py0,
            px1.saturating_sub(px0),
            py1.saturating_sub(py0),
            color,
        );
        if btn.enabled {
            fill_rect(
                &mut pixels,
                width,
                height,
                px0,
                py0,
                6,
                py1.saturating_sub(py0),
                [220, 180, 64, 255],
            );
        }
        hits.push(MenuHit {
            entry_id: btn.entry_id,
            action: btn.action,
            x0,
            y0,
            x1,
            y1,
            enabled: btn.enabled,
        });
    }

    let image = RgbaImage::new(width, height, pixels).expect("menu image size");
    MenuLayout { image, hits }
}

fn fill_rect(pixels: &mut [u8], width: u32, height: u32, x: u32, y: u32, w: u32, h: u32, rgba: [u8; 4]) {
    let x1 = (x + w).min(width);
    let y1 = (y + h).min(height);
    for py in y..y1 {
        for px in x..x1 {
            let i = ((py * width + px) * 4) as usize;
            pixels[i..i + 4].copy_from_slice(&rgba);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuAction, layout_for, layout_skirmish_lobby};
    use crate::screen::OriginalScreen;

    #[test]
    fn main_menu_hit_single_player() {
        let layout = layout_for(OriginalScreen::MainMenu, 1024, 768, None, None).unwrap();
        assert_eq!(layout.hits[0].entry_id, "single_player");
        let action = layout.hit(400.0, 280.0, 1024.0, 768.0);
        assert_eq!(action, Some(MenuAction::OpenSinglePlayer));
    }

    #[test]
    fn disabled_network_not_hit() {
        let layout = layout_for(OriginalScreen::MainMenu, 1024, 768, None, None).unwrap();
        // NETWORK 行约 y=0.44 → 338px，禁用。
        let action = layout.hit(400.0, 340.0, 1024.0, 768.0);
        assert_eq!(action, None);
    }

    #[test]
    fn lobby_map_row_is_selectable() {
        let maps = vec![ra_map::BootMapCandidate {
            file_name: "mp03t4.map".into(),
            width: 50,
            height: 50,
            theater: ra_map::Theater::Temperate,
        }];
        let layout = layout_skirmish_lobby(1024, 768, &maps, Some("mp03t4.map"), None, None);
        // 首行约 y=0.18 → 138px
        let action = layout.hit(400.0, 150.0, 1024.0, 768.0);
        assert_eq!(action, Some(MenuAction::SelectMap(0)));
    }

    #[test]
    fn hover_index_tracks_enabled_button() {
        let layout = layout_for(OriginalScreen::MainMenu, 1024, 768, None, None).unwrap();
        assert_eq!(layout.hover_index(400.0, 280.0, 1024.0, 768.0), Some(0));
        assert_eq!(layout.hover_index(400.0, 340.0, 1024.0, 768.0), None);
    }
}
