use ra_map::{IsoCell, OverlayCell, TileBlit, compose_terrain_rgba, paint_cell_sprites, paint_overlay_markers};

#[test]
fn compose_one_opaque_tile() {
    let mut rgba = vec![0u8; 60 * 30 * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[10, 20, 30, 255]);
    }
    let cells = [IsoCell { x: 2, y: 3, tile_num: 0, sub_tile: 0, z: 0, flags: 0 }];
    let img =
        compose_terrain_rgba(&cells, |_, _| Some(TileBlit { width: 60, height: 30, offset_x: 0, offset_y: 0, rgba: rgba.clone() })).unwrap();
    assert_eq!(img.drawn, 1);
    assert!(img.image.width() >= 60);
    assert!(img.image.height() >= 30);
    assert!(img.image.as_raw().chunks(4).any(|c| c[3] == 255));
}

#[test]
fn cliff_extra_draws_over_lower_neighbor() {
    // 低处水面钻石在北；高地悬崖 extra 向上伸出。若按 blit 顶边排序会被水面盖住。
    let mut water = vec![0u8; 60 * 30 * 4];
    for px in water.chunks_exact_mut(4) {
        px.copy_from_slice(&[0, 0, 200, 255]);
    }
    let mut cliff = vec![0u8; 60 * 60 * 4];
    // 上半 30 行：悬崖面（红）；下半 30 行：台地（绿）
    for y in 0..60 {
        for x in 0..60 {
            let i = (y * 60 + x) * 4;
            if y < 30 {
                cliff[i..i + 4].copy_from_slice(&[200, 40, 40, 255]);
            } else {
                cliff[i..i + 4].copy_from_slice(&[40, 180, 40, 255]);
            }
        }
    }
    let cells = [
        IsoCell { x: 5, y: 4, tile_num: 1, sub_tile: 0, z: 0, flags: 0 },
        IsoCell { x: 5, y: 5, tile_num: 2, sub_tile: 0, z: 1, flags: 0 },
    ];
    let img = compose_terrain_rgba(&cells, |tile, _| match tile {
        1 => Some(TileBlit { width: 60, height: 30, offset_x: 0, offset_y: 0, rgba: water.clone() }),
        2 => Some(TileBlit { width: 60, height: 60, offset_x: 0, offset_y: -30, rgba: cliff.clone() }),
        _ => None,
    })
    .unwrap();
    assert_eq!(img.drawn, 2);
    // 两格重叠区应留下悬崖红，而非水面蓝。
    let red = img.image.as_raw().chunks(4).filter(|c| c[0] > 150 && c[2] < 80 && c[3] > 0).count();
    let blue = img.image.as_raw().chunks(4).filter(|c| c[2] > 150 && c[0] < 80 && c[3] > 0).count();
    assert!(red > 100, "expected cliff face visible, red={red} blue={blue}");
}

#[test]
fn paint_overlay_marks_pixel() {
    let mut rgba = vec![0u8; 60 * 30 * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[10, 20, 30, 255]);
    }
    let cells = [IsoCell { x: 2, y: 3, tile_num: 0, sub_tile: 0, z: 0, flags: 0 }];
    let mut img =
        compose_terrain_rgba(&cells, |_, _| Some(TileBlit { width: 60, height: 30, offset_x: 0, offset_y: 0, rgba: rgba.clone() })).unwrap();
    let overlays = [OverlayCell { x: 2, y: 3, overlay_id: 110, data: 0 }];
    let n = paint_overlay_markers(&mut img, &overlays, |_, _| 0);
    assert_eq!(n, 1);
    assert!(img.image.as_raw().chunks(4).any(|c| c[0] == 230 && c[1] == 190 && c[2] == 40));
}

#[test]
fn paint_cell_sprite_marks_pixel() {
    let mut rgba = vec![0u8; 60 * 30 * 4];
    for px in rgba.chunks_exact_mut(4) {
        px.copy_from_slice(&[10, 20, 30, 255]);
    }
    let cells = [IsoCell { x: 2, y: 3, tile_num: 0, sub_tile: 0, z: 0, flags: 0 }];
    let mut img =
        compose_terrain_rgba(&cells, |_, _| Some(TileBlit { width: 60, height: 30, offset_x: 0, offset_y: 0, rgba: rgba.clone() })).unwrap();
    let mut sprite = vec![0u8; 4 * 4 * 4];
    for px in sprite.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]);
    }
    let items = [(2u16, 3u16, TileBlit { width: 4, height: 4, offset_x: 28, offset_y: 13, rgba: sprite })];
    let n = paint_cell_sprites(&mut img, &items, |_, _| 0);
    assert_eq!(n, 1);
    assert!(img.image.as_raw().chunks(4).any(|c| c[0] == 255 && c[1] == 0));
}
