//! 集成测试：原 `src/audio.rs` 内联测试迁出。

use ra_napi::host::audio::*;

#[test]
fn soft_ini_get_reads_section_after_noise() {
    let bytes = b"[SoundList]\n1=MenuClick\nbad line without eq\n[MenuClick]\nSounds=umenucl1\nVolume=90\n";
    assert_eq!(soft_ini_get(bytes, "MenuClick", "Sounds").as_deref(), Some("umenucl1"));
    assert_eq!(theme_sound_stem(" $Grinder "), "Grinder");
}
