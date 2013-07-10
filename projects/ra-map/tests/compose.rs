use ra_map::{IsoCell, OverlayCell, TileBlit, compose_terrain_rgba, paint_cell_sprites, paint_overlay_markers};

#[test]
fn compose_one_opaque_tile() {
    let mut rgba = vec![0u8; 60 * 30 * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[10, 20, 30, 255]);
    }
    let cells = [IsoCell { x: 2, y: 3, tile_num: 0, sub_tile: 0, z: 0, flags: 0 }];
    let img = compose_terrain_rgba(&cells, |_, _| {
        Some(TileBlit { width: 60, height: 30, offset_x: 0, offset_y: 0, rgba: rgba.clone() })
    })
    .unwrap();
    assert_eq!(img.drawn, 1);
    assert!(img.width >= 60);
    assert!(img.height >= 30);
    assert!(img.pixels.chunks(4).any(|c| c[3] == 255));
}

#[test]
fn paint_overlay_marks_pixel() {
    let mut rgba = vec![0u8; 60 * 30 * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[10, 20, 30, 255]);
    }
    let cells = [IsoCell { x: 2, y: 3, tile_num: 0, sub_tile: 0, z: 0, flags: 0 }];
    let mut img = compose_terrain_rgba(&cells, |_, _| {
        Some(TileBlit { width: 60, height: 30, offset_x: 0, offset_y: 0, rgba: rgba.clone() })
    })
    .unwrap();
    let overlays = [OverlayCell { x: 2, y: 3, overlay_id: 110, data: 0 }];
    let n = paint_overlay_markers(&mut img, &overlays, |_, _| 0);
    assert_eq!(n, 1);
    assert!(img.pixels.chunks(4).any(|c| c[0] == 230 && c[1] == 190 && c[2] == 40));
}

#[test]
fn paint_cell_sprite_marks_pixel() {
    let mut rgba = vec![0u8; 60 * 30 * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[10, 20, 30, 255]);
    }
    let cells = [IsoCell { x: 2, y: 3, tile_num: 0, sub_tile: 0, z: 0, flags: 0 }];
    let mut img = compose_terrain_rgba(&cells, |_, _| {
        Some(TileBlit { width: 60, height: 30, offset_x: 0, offset_y: 0, rgba: rgba.clone() })
    })
    .unwrap();
    let mut sprite = vec![0u8; 4 * 4 * 4];
    for px in sprite.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]);
    }
    let items = [(2u16, 3u16, TileBlit { width: 4, height: 4, offset_x: 28, offset_y: 13, rgba: sprite })];
    let n = paint_cell_sprites(&mut img, &items, |_, _| 0);
    assert_eq!(n, 1);
    assert!(img.pixels.chunks(4).any(|c| c[0] == 255 && c[1] == 0));
}
