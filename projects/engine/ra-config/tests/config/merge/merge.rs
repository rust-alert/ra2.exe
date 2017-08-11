//! 自顶层 `merge.rs`。

use std::path::PathBuf;

use ra_config::{
    ConfigLayer, ConfigTable, DesktopSettings, DesktopState, MergedConfig, SkirmishLobbyPrefs, set_test_user_data_dir,
};
use ra_types::DisplayMode;

#[test]
fn launch_override_screen_is_cli_only() {
    ra_config::clear_launch_override();
    assert_eq!(ra_config::launch_override_screen(), None);
    ra_config::set_launch_override(ra_config::LaunchOverride {
        ra2_dir: PathBuf::from("."),
        edition: None,
        screen: Some("skirmish".into()),
    });
    assert_eq!(ra_config::launch_override_screen().as_deref(), Some("skirmish"));
    let mut table = ConfigTable::new();
    table.insert("ra2_dir", ".");
    table.insert("screen", "skirmish");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let _ = DesktopSettings::from_merged(&merged);
    ra_config::clear_launch_override();
}

#[test]
fn later_layer_overrides() {
    let mut a = ConfigTable::new();
    a.insert("ra2_dir", ".");
    let mut b = ConfigTable::new();
    b.insert("ra2_dir", "C:/games/ra2");
    b.insert("edition", "yr");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "a".into(), table: a }, ConfigLayer { label: "b".into(), table: b }]);
    let s = DesktopSettings::from_merged(&merged);
    assert_eq!(s.ra2_dir, PathBuf::from("C:/games/ra2"));
    assert_eq!(s.edition.as_deref(), Some("yr"));
}

#[test]
fn display_mode_from_merged_and_alias() {
    let mut table = ConfigTable::new();
    table.insert("ra2_dir", ".");
    table.insert("display_mode", "800x600");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert_eq!(s.display_mode, DisplayMode::W800H600);

    let mut table = ConfigTable::new();
    table.insert("resolution", "640x480");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    assert_eq!(DesktopSettings::from_merged(&merged).display_mode, DisplayMode::W640H480);
}

#[test]
fn unknown_display_mode_keeps_default() {
    let mut table = ConfigTable::new();
    table.insert("display_mode", "1920x1080");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    assert_eq!(DesktopSettings::from_merged(&merged).display_mode, DisplayMode::DEFAULT);
}

#[test]
fn audio_volumes_from_merged_and_aliases() {
    let mut table = ConfigTable::new();
    table.insert("music_volume", "0.25");
    table.insert("sound_volume", "0.9");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert!((s.music_volume - 0.25).abs() < 1e-6);
    assert!((s.sound_volume - 0.9).abs() < 1e-6);

    let mut table = ConfigTable::new();
    table.insert("score_volume", "1.5");
    table.insert("sfx_volume", "-0.2");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert!((s.music_volume - 1.0).abs() < 1e-6);
    assert!((s.sound_volume - 0.0).abs() < 1e-6);
}

#[test]
fn invalid_audio_volume_keeps_default() {
    let mut table = ConfigTable::new();
    table.insert("music_volume", "loud");
    table.insert("sound_volume", "nan");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    let s = DesktopSettings::from_merged(&merged);
    assert!((s.music_volume - 0.4).abs() < 1e-6);
    assert!((s.sound_volume - 0.7).abs() < 1e-6);
}

#[test]
fn shell_slide_gap_secs_from_merged_and_disable() {
    assert!((DesktopSettings::default().shell_slide_gap_secs - 0.2).abs() < 1e-9);

    let mut table = ConfigTable::new();
    table.insert("shell_slide_gap_secs", "0.5");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    assert!((DesktopSettings::from_merged(&merged).shell_slide_gap_secs - 0.5).abs() < 1e-9);

    let mut table = ConfigTable::new();
    table.insert("shell_slide_gap_secs", "0");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    assert!((DesktopSettings::from_merged(&merged).shell_slide_gap_secs - 0.0).abs() < 1e-9);

    let mut table = ConfigTable::new();
    table.insert("shell_slide_gap_secs", "-1");
    let merged = MergedConfig::merge_layers(&[ConfigLayer { label: "t".into(), table }]);
    assert!((DesktopSettings::from_merged(&merged).shell_slide_gap_secs - 0.0).abs() < 1e-9);
}

struct TempDataDir {
    path: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TempDataDir {
    fn new(tag: &str) -> Self {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        ra_config::clear_launch_override();
        let path = std::env::temp_dir().join(format!(
            "ra_config_{tag}_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        set_test_user_data_dir(Some(path.clone()));
        Self { path, _guard: guard }
    }
}

impl Drop for TempDataDir {
    fn drop(&mut self) {
        set_test_user_data_dir(None);
        ra_config::clear_launch_override();
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn settings_json_round_trip() {
    let _tmp = TempDataDir::new("settings");
    let mut s = DesktopSettings::default();
    s.display_mode = DisplayMode::W800H600;
    s.music_volume = 0.33;
    s.present.dither = false;
    s.persist().unwrap();

    let (again, diags) = DesktopSettings::load_or_default();
    assert!(diags.iter().all(|d| !d.message.contains("解析失败")), "{diags:?}");
    assert_eq!(again.display_mode, DisplayMode::W800H600);
    assert!((again.music_volume - 0.33).abs() < 1e-6);
    assert!(!again.present.dither);
}

#[test]
fn state_json_skirmish_round_trip() {
    let _tmp = TempDataDir::new("state");
    let prefs = SkirmishLobbyPrefs {
        preferred_map: Some("mp01t4.map".into()),
        mode_id: Some(1),
        player_name: "Commander".into(),
        row_countries: vec!["Americans".into(), "Russians".into()],
        row_colors: vec![0, 2],
        difficulty: "Hard".into(),
        short_game: false,
        mcv_repacks: true,
        crates: false,
        superweapons: true,
        build_off_ally: true,
        game_speed: 4,
        credits: 5000,
        tech_level: 8,
        unit_count: 5,
    }
    .sanitized();
    DesktopState::persist_skirmish(&prefs).unwrap();

    let (state, diags) = DesktopState::load_or_default();
    assert!(diags.iter().all(|d| !d.message.contains("解析失败")), "{diags:?}");
    assert_eq!(state.skirmish.preferred_map.as_deref(), Some("mp01t4.map"));
    assert_eq!(state.skirmish.mode_id, Some(1));
    assert_eq!(state.skirmish.player_name, "Commander");
    assert_eq!(state.skirmish.row_countries, vec!["Americans", "Russians"]);
    assert_eq!(state.skirmish.difficulty, "Hard");
    assert!(!state.skirmish.short_game);
    assert_eq!(state.skirmish.game_speed, 4);
}

#[test]
fn optional_install_root_reads_settings_json() {
    let _tmp = TempDataDir::new("install");
    let fake_game = _tmp.path.join("game");
    std::fs::create_dir_all(&fake_game).unwrap();
    let mut s = DesktopSettings::default();
    s.ra2_dir = fake_game.clone();
    s.edition = Some("ra2".into());
    s.persist().unwrap();

    let (root, edition) = ra_config::resolve_optional_install_root().expect("settings root");
    assert_eq!(root, fake_game);
    assert_eq!(edition.as_deref(), Some("ra2"));
}
