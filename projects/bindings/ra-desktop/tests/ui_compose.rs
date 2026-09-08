//! 集成测试：原 `src/ui_compose.rs` 内联测试迁出。

use ra_desktop::{
    ui_compose::*,
    ui_decode::{DecodedUiSprite, PageDecodeReport},
    ui_layout::{
        MAIN_MENU_BUTTON_IDS, OPTIONS_BUTTON_IDS, SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_LOBBY_BUTTON_IDS, main_menu_layout, options_layout,
        single_player_layout, skirmish_lobby_layout,
    },
};
use ra_renderer::RgbaImage;
#[test]
fn blit_writes_opaque_pixel() {
    let mut dst = RgbaImage::from_raw(2, 2, vec![0u8; 16]).unwrap();
    let src = RgbaImage::from_raw(1, 1, vec![10, 20, 30, 255]).unwrap();
    blit_rgba(&mut dst, &src, 1, 1);
    assert_eq!(&dst.as_mut()[12..16], &[10, 20, 30, 255]);
}

fn solid_sprite(label: &str, rgba: [u8; 4]) -> DecodedUiSprite {
    DecodedUiSprite {
        label: label.into(),
        image: RgbaImage::from_raw(1, 1, rgba.to_vec()).unwrap(),
        origin: "test".into(),
        frame: 0,
        canvas: (1, 1),
        frame_rect: (0, 0, 1, 1),
    }
}

#[test]
fn compose_uses_pressed_sprite_when_entry_matches() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let pressed = solid_sprite("sdbtnanm.shp#4", [200, 0, 0, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: MAIN_MENU_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: vec![("single_player", pressed)],
        errors: Vec::new(),
    };
    let page = compose_main_menu_page(&decoded, 800, 600, Some("single_player"), None, None, None, None).unwrap();
    let layout = main_menu_layout(800, 600);
    let cell = layout.buttons[0];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[200, 0, 0, 255]);
}

#[test]
fn compose_uses_hover_sprite_when_not_pressed() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let hover = solid_sprite("sdbtnanm.shp#3", [0, 200, 0, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: MAIN_MENU_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: vec![("single_player", hover)],
        button_presseds: Vec::new(),
        errors: Vec::new(),
    };
    let page = compose_main_menu_page(&decoded, 800, 600, None, Some("single_player"), None, None, None).unwrap();
    let layout = main_menu_layout(800, 600);
    let cell = layout.buttons[0];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 200, 0, 255]);
}

#[test]
fn compose_single_player_uses_skirmish_id() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let pressed = solid_sprite("sdbtnanm.shp#4", [0, 200, 0, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: SINGLE_PLAYER_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: vec![("skirmish", pressed)],
        errors: Vec::new(),
    };
    let page = compose_single_player_page(&decoded, 800, 600, Some("skirmish"), None, None, None, None).unwrap();
    let layout = single_player_layout(800, 600);
    let cell = layout.buttons[1];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 200, 0, 255]);
}
#[test]
fn compose_skirmish_lobby_uses_side_id() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let pressed = solid_sprite("sdbtnanm.shp#4", [0, 0, 200, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: SKIRMISH_LOBBY_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: vec![("side", pressed)],
        errors: Vec::new(),
    };
    let page = compose_skirmish_lobby_page(&decoded, 800, 600, Some("side"), None, None, None, None, &[]).unwrap();
    let layout = skirmish_lobby_layout(800, 600);
    let cell = layout.buttons[0];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 0, 200, 255]);
}

#[test]
fn compose_options_uses_main_menu_id() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let pressed = solid_sprite("sdbtnanm.shp#4", [200, 200, 0, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: OPTIONS_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: vec![("main_menu", pressed)],
        errors: Vec::new(),
    };
    let page = compose_options_page(&decoded, 800, 600, Some("main_menu"), None, None, None, None).unwrap();
    let layout = options_layout(800, 600);
    let cell = layout.buttons[2];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[200, 200, 0, 255]);
}
