//! 集成测试：原 `src/ui_compose.rs` 内联测试迁出。

use ra_widgets::{
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
    let page = compose_main_menu_page(&decoded, 800, 600, Some("single_player"), None, None, None, None, None, Some(ShellWaveFrames { buttons: &frames, tiles: &[], animate_empty_tiles: false }), 0).unwrap();
    let layout = main_menu_layout(800, 600);
    let cell = layout.buttons[0];
    let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 0, 255, 255]);
}

#[test]
fn compose_hides_button_caption_while_wave_frames_active() {
    use ra_assets::{FONT_MAGIC, FntFile};

    const LOOKUP_TABLE_BYTES: usize = 65536 * 2;
    let bytes_per_row = 1u32;
    let bitmap_rows = 1u32;
    let cell_height = 2u32;
    let num_slots = 1u32;
    let glyph_stride = 1 + bytes_per_row * bitmap_rows;
    let mut data = Vec::new();
    data.extend_from_slice(&FONT_MAGIC.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&bytes_per_row.to_le_bytes());
    data.extend_from_slice(&bitmap_rows.to_le_bytes());
    data.extend_from_slice(&cell_height.to_le_bytes());
    data.extend_from_slice(&num_slots.to_le_bytes());
    data.extend_from_slice(&glyph_stride.to_le_bytes());
    let mut lut = vec![0u8; LOOKUP_TABLE_BYTES];
    let off = (b's' as usize) * 2;
    lut[off] = 1;
    lut[off + 1] = 0;
    data.extend_from_slice(&lut);
    data.push(1);
    data.push(0b1000_0000);
    let fnt = FntFile::parse(&data).unwrap();

    let layout = main_menu_layout(800, 600);
    let cell = layout.buttons[0];
    let w = cell.w.max(1) as u16;
    let h = cell.h.max(1) as u16;
    let px = (u32::from(w) * u32::from(h)) as usize;
    let normal_big = DecodedUiSprite {
        label: "sdbtnanm.shp#2".into(),
        image: RgbaImage::from_raw(u32::from(w), u32::from(h), vec![10u8; px * 4]).unwrap(),
        origin: "test".into(),
        frame: 2,
        canvas: (w, h),
        frame_rect: (0, 0, w, h),
    };
    let wave_big = DecodedUiSprite {
        label: "sdbtnanm.shp#10".into(),
        image: RgbaImage::from_raw(u32::from(w), u32::from(h), [0u8, 0, 255, 255].repeat(px)).unwrap(),
        origin: "test".into(),
        frame: 10,
        canvas: (w, h),
        frame_rect: (0, 0, w, h),
    };
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: Vec::new(),
        button_normals: MAIN_MENU_BUTTON_IDS.iter().map(|id| (*id, normal_big.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: Vec::new(),
        sdbtnanm_frames: vec![wave_big],
        errors: Vec::new(),
    };
    let sample = |page: &RgbaImage| {
        let x = (cell.x + cell.w / 2) as u32;
        let y = (cell.y + cell.h / 2) as u32;
        let di = ((y * page.width() + x) * 4) as usize;
        page.as_raw()[di..di + 4].to_vec()
    };

    let steady = compose_main_menu_page(&decoded, 800, 600, None, None, None, Some(&fnt), None, None, None, 0).unwrap();
    // 稳态会叠黄字，中心附近不应再是纯钮面灰。
    assert_ne!(&sample(&steady)[..3], &[10, 10, 10]);

    let frames = [10u16, 10, 10, 10, 10, 10];
    let waving = compose_main_menu_page(&decoded, 800, 600, None, None, None, Some(&fnt), None, None, Some(ShellWaveFrames { buttons: &frames, tiles: &[], animate_empty_tiles: false }), 0).unwrap();
    // 波浪中只留 `SDBTNANM` 帧色，字等停稳后再叠。
    assert_eq!(&sample(&waving)[..], &[0, 0, 255, 255]);
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
    let state = ra_widgets::options_dialog::OptionsDialogState::from_shell(
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

#[test]
fn compose_skirmish_overlays_sdtp_frame1_and_sdmpbtn() {
    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let sdtp0 = DecodedUiSprite {
        label: "sdtp.shp#0".into(),
        image: RgbaImage::from_raw(168, 199, vec![40u8; 168 * 199 * 4]).unwrap(),
        origin: "test".into(),
        frame: 0,
        canvas: (168, 199),
        frame_rect: (0, 0, 168, 199),
    };
    let mut top1_px = vec![0u8; 168 * 199 * 4];
    for px in top1_px.chunks_exact_mut(4) {
        px.copy_from_slice(&[20, 80, 120, 255]);
    }
    let sdtp1 = DecodedUiSprite {
        label: "sdtp.shp#1".into(),
        image: RgbaImage::from_raw(168, 199, top1_px).unwrap(),
        origin: "test".into(),
        frame: 1,
        canvas: (168, 199),
        frame_rect: (0, 0, 168, 199),
    };
    let mut plate_px = vec![0u8; 156 * 84 * 4];
    for px in plate_px.chunks_exact_mut(4) {
        px.copy_from_slice(&[200, 40, 40, 255]);
    }
    let sdmpbtn = DecodedUiSprite {
        label: "sdmpbtn.shp#0".into(),
        image: RgbaImage::from_raw(156, 84, plate_px).unwrap(),
        origin: "test".into(),
        frame: 0,
        canvas: (156, 84),
        frame_rect: (0, 0, 156, 84),
    };
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: vec![sdtp0, sdtp1, sdmpbtn],
        button_normals: SKIRMISH_LOBBY_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: Vec::new(),
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let paint = SkirmishLobbyPaint::default();
    let page = compose_skirmish_lobby_page(&decoded, 800, 600, None, None, None, None, None, None, &paint, None, 0).unwrap();
    let layout = skirmish_lobby_layout(800, 600);
    // 顶盖被帧 1 覆盖。
    let ti = ((layout.shell.panel_top.y as u32 * page.width() + (layout.shell.panel_top.x as u32 + 2)) * 4) as usize;
    assert_eq!(&page.as_raw()[ti..ti + 4], &[20, 80, 120, 255]);
    // 地图名底板贴到 `sdmpbtn` 格。
    let pi = ((layout.map_name_plate.y as u32 * page.width() + layout.map_name_plate.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[pi..pi + 4], &[200, 40, 40, 255]);
}

#[test]
fn compose_empty_tiles_use_wave_sdbtnanm_instead_of_static_bkgd() {
    let layout = main_menu_layout(800, 600);
    let tile_h = layout.panel_tile.h;
    let tile_y0 = layout.panel_tile.y;
    // 找一个无按钮占用的平铺格。
    let empty_ti = (0..layout.panel_tile_count)
        .find(|&ti| {
            let y = tile_y0 + ti * tile_h;
            !layout.buttons.iter().any(|b| b.w > 0 && b.y == y)
        })
        .expect("main menu should have empty panel tiles");
    let empty_y = tile_y0 + empty_ti * tile_h;
    let cell_x = layout.panel_tile.x + (ra_layout::RIGHT_PANEL_W - ra_layout::BUTTON_CELL_W);

    let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
    let bkgd = solid_sprite("sdbtnbkgd.shp#0", [90, 90, 90, 255]);
    let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
    let mut wave10 = solid_sprite("sdbtnanm.shp#10", [0, 255, 0, 255]);
    wave10.frame = 10;
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: vec![bkgd],
        button_normals: MAIN_MENU_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
        button_hovers: Vec::new(),
        button_presseds: Vec::new(),
        sdbtnanm_frames: vec![wave10],
        errors: Vec::new(),
    };
    let buttons = [10u16; 6];
    let mut tiles = vec![1u16; layout.panel_tile_count as usize];
    tiles[empty_ti as usize] = 10;
    let page = compose_main_menu_page(
        &decoded,
        800,
        600,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(ShellWaveFrames {
            buttons: &buttons,
            tiles: &tiles,
            animate_empty_tiles: true,
        }),
        0,
    )
    .unwrap();
    let di = ((empty_y as u32 * page.width() + cell_x as u32) * 4) as usize;
    // 出去时：空格钮格叠波浪帧绿。
    assert_eq!(&page.as_raw()[di..di + 4], &[0, 255, 0, 255]);
    // 左侧红线带仍是 `sdbtnbkgd`，不被波浪藏掉。
    let wire_x = layout.panel_tile.x as u32;
    let wi = ((empty_y as u32 * page.width() + wire_x) * 4) as usize;
    assert_eq!(&page.as_raw()[wi..wi + 4], &[90, 90, 90, 255]);

    let slide_in = compose_main_menu_page(
        &decoded,
        800,
        600,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(ShellWaveFrames {
            buttons: &buttons,
            tiles: &tiles,
            animate_empty_tiles: false,
        }),
        0,
    )
    .unwrap();
    let di_in = ((empty_y as u32 * page.width() + cell_x as u32) * 4) as usize;
    // 进来时：空格不叠满钮，只留底图，避免收束后消失。
    assert_eq!(&slide_in.as_raw()[di_in..di_in + 4], &[90, 90, 90, 255]);
}

#[test]
fn compose_load_screen_paints_country_art_and_progress() {
    let bg = solid_sprite("ls800ustates.shp#0", [1, 2, 3, 255]);
    let mut bar = solid_sprite("progbarm.shp#0", [200, 40, 40, 255]);
    bar.image = RgbaImage::from_raw(80, 5, vec![200u8, 40, 40, 255].repeat(80 * 5)).unwrap();
    let btn = solid_sprite("mnbttn.shp#0", [200, 20, 20, 255]);
    let decoded = PageDecodeReport {
        background: Some(bg),
        panels: vec![bar],
        button_normals: vec![("retry", btn.clone()), ("cancel", btn)],
        button_hovers: Vec::new(),
        button_presseds: Vec::new(),
        sdbtnanm_frames: Vec::new(),
        errors: Vec::new(),
    };
    let loading = compose_load_screen_page(
        &decoded,
        800,
        600,
        None,
        None,
        None,
        None,
        LoadScreenPaint {
            side: "Americans",
            player_name: "Player",
            side_flag: None,
            status: "装载中",
            allow_retry: false,
            progress: 0.5,
            brief_csf_override: None,
        },
    )
    .unwrap();
    // 国家艺术铺满画布左上。
    assert_eq!(&loading.as_raw()[0..4], &[1, 2, 3, 255]);
    // 进度条在中下偏左（原版约 y=332）。
    let px = ((332u32 * loading.width() + 56) * 4) as usize;
    assert_eq!(&loading.as_raw()[px..px + 4], &[200, 40, 40, 255]);

    let failed = compose_load_screen_page(
        &decoded,
        800,
        600,
        None,
        None,
        None,
        None,
        LoadScreenPaint {
            side: "Americans",
            player_name: "Player",
            side_flag: None,
            status: "装载失败 · test",
            allow_retry: true,
            progress: 1.0,
            brief_csf_override: None,
        },
    )
    .unwrap();
    assert_eq!(failed.width(), 800);
    assert_eq!(failed.height(), 600);
}

#[test]
fn compose_battle_hud_overlay_right_strip_opaque() {
    let page = compose_battle_hud_overlay(
        800,
        600,
        None,
        BattleHudModel {
            tick: 12,
            funds: 1000,
            power_output: 100,
            power_drain: 50,
            low_power: false,
            selected_summary: "—",
            produce_queue: None,
            reject: None,
            paused: false,
            pause_reason: None,
            outcome: None,
        },
        None,
    )
    .unwrap();
    // 左侧透明。
    assert_eq!(page.as_raw()[3], 0);
    // 右侧栏内有可见像素（避开顶栏描边与资金框）。
    let x = (800 - 40) as u32;
    let y = 120u32;
    let di = ((y * 800 + x) * 4) as usize;
    assert!(page.as_raw()[di + 3] >= 230, "right strip alpha={}", page.as_raw()[di + 3]);
}
