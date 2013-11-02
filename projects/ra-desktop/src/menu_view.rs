//! 原版主 UI 资产接线前的菜单占位绘制（色块按钮）。
//!
//! **不是 Pre-Alpha 交付。** 色块可点导航只证明页面状态机与输入接线，
//! 不能替代原版 SHP/字体/布局与截图对照验收。正式复刻见产品主菜单路径。
//!
//! 命中框与入口 id 以 [`crate::ui_slots`] 为单一来源。

use ra_renderer::RgbaImage;

use crate::{screen::OriginalScreen, ui_slots::{UiPageSlots, slots_for}};

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
    let page = slots_for(screen)?;
    Some(paint_from_slots(w, h, &page))
}

fn paint_from_slots(width: u32, height: u32, page: &UiPageSlots) -> MenuLayout {
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
        let color = if btn.enabled {
            if i % 2 == 0 {
                [48, 92, 160, 255]
            }
            else {
                [40, 78, 140, 255]
            }
        }
        else {
            [40, 40, 48, 255]
        };
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
