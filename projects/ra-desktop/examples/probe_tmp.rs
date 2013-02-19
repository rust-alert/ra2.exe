//! 验证剧院瓷砖表与 `isotemp.mix` 中的 TMP。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{MixVfs, Palette, TmpFile};
use ra_map::{
    parse_tileset_ini, theater_ini_name, theater_mix_names, theater_palette,
    theater_tmp_extension, Theater,
};
use ra_types::GameEdition;
use std::path::PathBuf;

fn main() {
    let Some(root) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("用法: probe_tmp <游戏目录>");
        std::process::exit(2);
    };
    let manifest = detect_edition(&root, Some(GameEdition::Ra2)).expect("detect");
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        if let Some(p) = find_ci_file(&root, name) {
            let _ = vfs.mount_bytes(name.clone(), std::fs::read(p).unwrap());
        }
    }
    for n in manifest.chain.nested_mix_files {
        let _ = vfs.mount_nested(n);
    }
    for n in theater_mix_names(Theater::Temperate) {
        match vfs.mount_nested(n) {
            Ok(true) => eprintln!("mounted {n}"),
            Ok(false) => eprintln!("missing {n}"),
            Err(e) => eprintln!("skip {n}: {e}"),
        }
    }

    let ext = theater_tmp_extension(Theater::Temperate);
    let ini = vfs
        .read(theater_ini_name(Theater::Temperate))
        .expect("temperat.ini");
    let lookup = parse_tileset_ini(&ini, ext).expect("tileset");
    eprintln!("tileset slots={}", lookup.len());
    for id in [0i32, 1, 9, 14] {
        eprintln!("tile {id} => {:?}", lookup.filename(id));
    }

    let pal = Palette::parse(
        &vfs.read(theater_palette(Theater::Temperate))
            .expect("isotem.pal"),
    )
    .expect("pal");

    let name = lookup.filename(0).expect("clear tile");
    let data = vfs.read(name).expect(name);
    let tmp = TmpFile::parse(&data).expect("tmp");
    let rgba = tmp.tile_to_rgba(0, &pal).expect("rgba");
    let opaque = rgba.chunks(4).filter(|c| c[3] > 0).count();
    eprintln!(
        "OK {name} cells={} tile={}x{} opaque_px={}",
        tmp.cell_count(),
        tmp.tile_width,
        tmp.tile_height,
        opaque
    );
}
