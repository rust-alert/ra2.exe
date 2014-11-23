//! 集成测试：原 `src/ui_compose.rs` 内联测试迁出。

use ra_components::{
    ui_compose::*,
    ui_decode::{DecodedUiSprite, PageDecodeReport},
};
use ra_layout::ui_layout::{
    MAIN_MENU_BUTTON_IDS, OPTIONS_BUTTON_IDS, SINGLE_PLAYER_BUTTON_IDS, SKIRMISH_LOBBY_BUTTON_IDS, main_menu_layout, options_layout,
    single_player_layout, skirmish_lobby_layout,
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
fn compose_uses_wave_sdbtnanm_frame_over_pressed() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let pressed = solid_sprite("sdbtnanm.shp#4", [200, 0, 0, 255]);
    let mut wave10 = solid_sprite("sdbtnanm.shp#10", [0, 0, 255, 255]);
    wave10.frame = 10;
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: MAIN_MENU_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: vec![("single_player", pressed)],
        sdbtnanm_frames: vec![wave10],
        errors: Vec::new(),
    };
    let frames = [10u16, 10, 10, 10, 10, 10];
    let page = compose_main_menu_page(&decoded, 800, 600, Some("single_player"), None, None, None, None, None, Some(&frames), 0).unwrap();
    let layout = main_menu_layout(800, 600);
    let cell = layout.buttons[0];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 0, 255, 255]);
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
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let page = compose_main_menu_page(&decoded, 800, 600, Some("single_player"), None, None, None, None, None, None, 0).unwrap();
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
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let page = compose_main_menu_page(&decoded, 800, 600, None, Some("single_player"), None, None, None, None, None, 0).unwrap();
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
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let page = compose_single_player_page(&decoded, 800, 600, Some("skirmish"), None, None, None, None, None, None, 0).unwrap();
    let layout = single_player_layout(800, 600);
    let cell = layout.buttons[2];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 200, 0, 255]);
}

#[test]
fn compose_skirmish_lobby_uses_start_id() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let pressed = solid_sprite("sdbtnanm.shp#4", [0, 0, 200, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: SKIRMISH_LOBBY_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: vec![("start", pressed)],
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let paint = SkirmishLobbyPaint::default();
    let page = compose_skirmish_lobby_page(&decoded, 800, 600, Some("start"), None, None, None, None, None, &paint, None, 0).unwrap();
    let layout = skirmish_lobby_layout(800, 600);
    let cell = layout.shell.buttons[0];
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
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let state = ra_components::options_dialog::OptionsDialogState::from_shell(
        ra_types::DisplayMode::W800H600,
        0.4,
        0.7,
        ra_types::PresentFeel::DEFAULT,
    );
    let page = compose_options_page(&decoded, &state, 800, 600, Some("main_menu"), None, None, None, None, 0).unwrap();
    let layout = options_layout(800, 600);
    let cell = layout.buttons[2];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[200, 200, 0, 255]);
}

#[test]
fn compose_campaign_uses_back_id() {
    use ra_layout::ui_layout::{CAMPAIGN_BUTTON_IDS, campaign_layout};
    let bg = solid_sprite("fsbkgdlg.shp#0", [1, 2, 3, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let pressed = solid_sprite("sdbtnanm.shp#4", [0, 200, 200, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: CAMPAIGN_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: vec![("back", pressed)],
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let paint = CampaignPaint { selected_side: Some("allied"), difficulty: 1, track_thumb: None, side_anim_frame: 1 };
    let page = compose_campaign_page(&decoded, 800, 600, Some("back"), None, None, None, None, paint, None, 0).unwrap();
    let layout = campaign_layout(800, 600);
    let cell = layout.shell.buttons[0];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 200, 200, 255]);
}

#[test]
fn compose_blits_sdwrnanm_inside_sdtp_window_not_full_panel() {
    use ra_layout::ui_layout::{SDWRNANM_OFFSET_X, SDWRNANM_OFFSET_Y};
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    // `sdtp` 外壳：整幅灰；窗内会被 `sdwrnanm` 替换。
    let sdtp = DecodedUiSprite {
        label: "sdtp.shp#0".into(),
        image: RgbaImage::from_raw(168, 199, vec![40u8; 168 * 199 * 4]).unwrap(),
        origin: "test".into(),
        frame: 0,
        canvas: (168, 199),
        frame_rect: (0, 0, 168, 199),
    };
    let mut warn_px = vec![0u8; 92 * 53 * 4];
    for px in warn_px.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 128, 0, 255]);
    }
    let warn = DecodedUiSprite {
        label: "sdwrnanm.shp#0".into(),
        image: RgbaImage::from_raw(92, 53, warn_px).unwrap(),
        origin: "test".into(),
        frame: 0,
        canvas: (92, 53),
        frame_rect: (0, 0, 92, 53),
    };
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: vec![sdtp, warn],
        button_normals: MAIN_MENU_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: Vec::new(),
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let page = compose_main_menu_page(&decoded, 800, 600, None, None, None, None, None, None, None, 0).unwrap();
    let layout = main_menu_layout(800, 600);
    let wx = layout.panel_top.x + SDWRNANM_OFFSET_X;
    let wy = layout.panel_top.y + SDWRNANM_OFFSET_Y;
    let wi = ((wy as u32 * page.width() + wx as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[wi..wi + 4], &[255, 128, 0, 255]);
    // 窗缘外侧仍应是 `sdtp` 灰，证明没有整幅盖住顶盖。
    let edge_x = layout.panel_top.x + 2;
    let edge_y = layout.panel_top.y + 2;
    let ei = ((edge_y as u32 * page.width() + edge_x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[ei..ei + 4], &[40, 40, 40, 40]);
}
