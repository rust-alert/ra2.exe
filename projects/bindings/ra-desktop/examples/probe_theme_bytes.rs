//! 验证本安装 `intro.aud` 可解码为菜单 BGM，并打印峰值 / RMS。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{MixVfs, decode_audio_bytes};
use ra_types::GameEdition;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(r"E:/ra 红色警戒/Red_Alert_2");
    let manifest = detect_edition(&root, Some(GameEdition::Ra2)).unwrap();
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        if let Some(path) = find_ci_file(&root, name) {
            if let Ok(data) = std::fs::read(path) {
                if data.len() >= 64 {
                    let _ = vfs.mount_bytes(name.clone(), data);
                }
            }
        }
    }
    for name in manifest.chain.nested_mix_files {
        let _ = vfs.mount_nested_all_from_parents(name);
    }
    let bytes = vfs.read("intro.aud").expect("intro.aud");
    let pcm = decode_audio_bytes(&bytes, Some("aud")).expect("decode");
    let mut peak = 0i32;
    let mut sum = 0f64;
    for &s in &pcm.samples {
        let a = (s as i32).abs();
        if a > peak {
            peak = a;
        }
        sum += (s as f64) * (s as f64);
    }
    let n = pcm.samples.len().max(1) as f64;
    println!(
        "frames={} peak={} rms={:.1} rate={} ch={}",
        pcm.samples.len(),
        peak,
        (sum / n).sqrt(),
        pcm.sample_rate,
        pcm.channels
    );
}
