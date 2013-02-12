//! 探测剧院 MIX 中的 TMP，并解一砖 RGBA。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{MixArchive, MixVfs, Palette, TmpFile};
use ra_map::{theater_mix_names, theater_palette, Theater};
use ra_types::GameEdition;
use std::path::PathBuf;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../Red_Alert_2"));
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

    let pal_name = theater_palette(Theater::Temperate);
    let pal = Palette::parse(&vfs.read(pal_name).expect(pal_name)).expect("pal");
    eprintln!("{pal_name} ok");

    let bytes = vfs.read("temperat.mix").expect("temperat.mix");
    let mix = MixArchive::parse(bytes).expect("parse temperat");
    let mut parsed = 0usize;
    let mut failed = 0usize;
    let mut previewed = false;
    for e in mix.entries() {
        let Some(data) = mix.get_by_id(e.id) else {
            continue;
        };
        if data.len() < 16 {
            continue;
        }
        let tw = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let th = u32::from_le_bytes(data[12..16].try_into().unwrap());
        if tw != 60 || th != 30 {
            continue;
        }
        match TmpFile::parse(data) {
            Ok(tmp) => {
                parsed += 1;
                if !previewed {
                    if let Some(tile) = tmp.tiles.iter().flatten().next() {
                        let rgba = tmp.tile_to_rgba(0, &pal).expect("rgba");
                        let opaque = rgba.chunks(4).filter(|c| c[3] > 0).count();
                        eprintln!(
                            "sample id={:#010x} cells={} first={}x{} opaque_px={}",
                            e.id as u32,
                            tmp.cell_count(),
                            tile.pixel_width,
                            tile.pixel_height,
                            opaque
                        );
                        previewed = true;
                    }
                }
            }
            Err(err) => {
                failed += 1;
                if failed <= 3 {
                    eprintln!("fail id={:#010x}: {err}", e.id as u32);
                }
            }
        }
    }
    eprintln!("TMP parse ok={parsed} fail={failed}");
    if parsed == 0 {
        std::process::exit(1);
    }
}
