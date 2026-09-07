//! 原版主 UI 资产接线前的菜单占位绘制（色块按钮，非最终交付）。
//!
//! 仅用于证明「启动进入主菜单且可点击导航」。正式复刻须换成原版 SHP/字体资产。

use ra_renderer::RgbaImage;

use crate::screen::OriginalScreen;

/// 菜单上的一个可点区域（窗口归一化坐标 0..1）。
#[derive(Debug, Clone, Copy)]
pub struct MenuHit {
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
        if win_w <= 0.0 || win_h <= 0.0 {
            return None;
        }
        let nx = (cursor_x / win_w) as f32;
        let ny = (cursor_y / win_h) as f32;
        for h in &self.hits {
            if !h.enabled {
                continue;
            }
            if nx >= h.x0 && nx <= h.x1 && ny >= h.y0 && ny <= h.y1 {
                return Some(h.action);
            }
        }
        None
    }
}

/// 为当前原版产品页生成占位布局；对局/结算页返回 `None`。
pub fn layout_for(screen: OriginalScreen, width: u32, height: u32) -> Option<MenuLayout> {
    let (w, h) = (width.max(320), height.max(240));
    match screen {
        OriginalScreen::MainMenu => Some(paint_menu(
            w,
            h,
            &[
                ("SINGLE PLAYER", MenuAction::OpenSinglePlayer, true, 0.28, 0.32),
                ("NETWORK", MenuAction::OpenNetwork, false, 0.28, 0.44),
                ("OPTIONS", MenuAction::OpenOptions, true, 0.28, 0.56),
                ("EXIT", MenuAction::Exit, true, 0.28, 0.68),
            ],
        )),
        OriginalScreen::SinglePlayerMenu => Some(paint_menu(
            w,
            h,
            &[
                ("CAMPAIGN", MenuAction::Back, false, 0.28, 0.30),
                ("SKIRMISH", MenuAction::OpenSkirmish, true, 0.28, 0.42),
                ("TRAINING", MenuAction::Back, false, 0.28, 0.54),
                ("BACK", MenuAction::Back, true, 0.28, 0.68),
            ],
        )),
        OriginalScreen::SkirmishLobby => Some(paint_menu(
            w,
            h,
            &[
                ("START", MenuAction::StartSkirmish, true, 0.28, 0.40),
                ("BACK", MenuAction::Back, true, 0.28, 0.56),
            ],
        )),
        OriginalScreen::Network => Some(paint_menu(
            w,
            h,
            &[("BACK (NETWORK DISABLED)", MenuAction::Back, true, 0.22, 0.48)],
        )),
        OriginalScreen::Options => Some(paint_menu(
            w,
            h,
            &[("BACK", MenuAction::Back, true, 0.28, 0.48)],
        )),
        OriginalScreen::LoadScreen => Some(paint_menu(w, h, &[("LOADING…", MenuAction::Back, false, 0.30, 0.45)])),
        OriginalScreen::Match | OriginalScreen::Results => None,
    }
}

fn paint_menu(width: u32, height: u32, rows: &[(&str, MenuAction, bool, f32, f32)]) -> MenuLayout {
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    // 深蓝底，避免看起来像对局地图。
    fill_rect(&mut pixels, width, height, 0, 0, width, height, [12, 18, 36, 255]);
    // 顶条提示「占位」。
    fill_rect(&mut pixels, width, height, 0, 0, width, height / 10, [28, 40, 72, 255]);

    let mut hits = Vec::with_capacity(rows.len());
    let btn_w = 0.44_f32;
    let btn_h = 0.08_f32;
    for (i, &(_label, action, enabled, x0, y0)) in rows.iter().enumerate() {
        let x1 = x0 + btn_w;
        let y1 = y0 + btn_h;
        let color = if enabled {
            if i % 2 == 0 { [48, 92, 160, 255] } else { [40, 78, 140, 255] }
        }
        else {
            [40, 40, 48, 255]
        };
        let px0 = (x0 * width as f32) as u32;
        let py0 = (y0 * height as f32) as u32;
        let px1 = (x1 * width as f32) as u32;
        let py1 = (y1 * height as f32) as u32;
        fill_rect(&mut pixels, width, height, px0, py0, px1.saturating_sub(px0), py1.saturating_sub(py0), color);
        // 左侧亮条表示可点。
        if enabled {
            fill_rect(&mut pixels, width, height, px0, py0, 6, py1.saturating_sub(py0), [220, 180, 64, 255]);
        }
        hits.push(MenuHit { action, x0, y0, x1, y1, enabled });
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
    use super::{MenuAction, layout_for};
    use crate::screen::OriginalScreen;

    #[test]
    fn main_menu_hit_single_player() {
        let layout = layout_for(OriginalScreen::MainMenu, 1024, 768).unwrap();
        let action = layout.hit(400.0, 280.0, 1024.0, 768.0);
        assert_eq!(action, Some(MenuAction::OpenSinglePlayer));
    }

    #[test]
    fn disabled_network_not_hit() {
        let layout = layout_for(OriginalScreen::MainMenu, 1024, 768).unwrap();
        // NETWORK 行约 y=0.44 → 338px，禁用。
        let action = layout.hit(400.0, 340.0, 1024.0, 768.0);
        assert_eq!(action, None);
    }
}
