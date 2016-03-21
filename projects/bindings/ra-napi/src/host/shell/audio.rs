//! 壳层音频：BGM、UI 音效与音量。

use ra_assets::{AudioIndex, IniDocument, PcmAudio, decode_audio_bytes};
use ra_types::AssetSource;
use ra_widgets::original_screen::OriginalScreen;

use super::Shell;

impl Shell {
    /// 按配置应用壳层 BGM / 短音效音量（设备缺失时无操作）。
    pub(super) fn apply_audio_volumes(&mut self, music_volume: f32, sound_volume: f32) {
        if let Some(audio) = self.audio.as_mut() {
            audio.set_music_volume(music_volume);
            audio.set_sfx_volume(sound_volume);
            tracing::info!(music_volume = audio.music_volume(), sound_volume = audio.sfx_volume(), "已应用壳层音量");
        }
    }

    /// 惰性解析 `audio.idx` / `audio.bag`，结果缓存在壳层。
    pub(super) fn ensure_audio_bag(&mut self) {
        if self.audio_bag_tried {
            return;
        }
        self.audio_bag_tried = true;
        self.ensure_menu_assets();
        let idx_bytes = self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| s.read("audio.idx").ok());
        let bag_bytes = self.menu_assets.as_ref().and_then(|a| a.source.as_ref()).and_then(|s| s.read("audio.bag").ok());
        let Some((idx, bag)) = idx_bytes.zip(bag_bytes)
        else {
            tracing::debug!("audio.idx/audio.bag 不可读");
            return;
        };
        match AudioIndex::parse(&idx, bag) {
            Some(index) => {
                tracing::info!(entries = index.len(), "已缓存 audio.bag 索引");
                self.audio_bag = Some(index);
            }
            None => tracing::warn!("audio.idx 解析失败"),
        }
    }

    /// 从缓存的 bag 按候选名解码首个命中采样。
    pub(super) fn decode_bag_named(&mut self, names: &[&str]) -> Option<PcmAudio> {
        self.ensure_audio_bag();
        let index = self.audio_bag.as_ref()?;
        for name in names {
            if let Some(pcm) = index.decode(name) {
                tracing::info!(%name, frames = pcm.samples.len(), "已从 audio.bag 解码采样");
                return Some(pcm);
            }
        }
        tracing::debug!(entries = index.len(), "audio.bag 未命中候选名");
        None
    }

    /// 对局/EVA 采样：先 `audio.bag`，再试 MIX 内独立 `{stem}.wav` / `.aud`（如 `ceva015.wav`）。
    pub(super) fn decode_sfx_named(&mut self, names: &[&str]) -> Option<PcmAudio> {
        if let Some(pcm) = self.decode_bag_named(names) {
            return Some(pcm);
        }
        self.ensure_menu_assets();
        for raw in names {
            let stem = {
                let t = raw.trim().trim_start_matches(['$', '#']);
                match t.rsplit_once('.') {
                    Some((s, ext)) if ext.eq_ignore_ascii_case("wav") || ext.eq_ignore_ascii_case("aud") => s,
                    _ => t,
                }
            };
            if stem.is_empty() {
                continue;
            }
            for ext in ["wav", "aud"] {
                let file = format!("{stem}.{ext}");
                let Some(bytes) = self.read_asset_bytes(&file)
                else {
                    continue;
                };
                match decode_audio_bytes(&bytes, Some(ext)) {
                    Ok(pcm) => {
                        tracing::info!(
                            %file,
                            frames = pcm.samples.len() / pcm.channels.max(1) as usize,
                            rate = pcm.sample_rate,
                            "已从 MIX 解码采样"
                        );
                        return Some(pcm);
                    }
                    Err(e) => {
                        tracing::debug!(%file, error = %e, "MIX 采样解码失败");
                    }
                }
            }
        }
        None
    }

    /// `theme.ini` `[INTRO]` 的 `Sound=` 词干（缺省 `Grinder`）。
    pub(super) fn menu_theme_sound_stem(&self) -> String {
        let from_doc = self
            .read_ini_doc("theme.ini")
            .as_ref()
            .and_then(|d| d.get("INTRO", "Sound"))
            .map(crate::host::audio::theme_sound_stem)
            .map(str::to_string)
            .filter(|s| !s.is_empty());
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("theme.ini")
            .and_then(|b| crate::host::audio::soft_ini_get(&b, "INTRO", "Sound"))
            .map(|s| crate::host::audio::theme_sound_stem(&s).to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Grinder".into())
    }

    /// 规则里的主菜单点击事件 id（缺省 `MenuClick`）。
    pub(super) fn menu_click_sound_id(&self) -> String {
        let from_doc = self
            .read_ini_doc("rules.ini")
            .as_ref()
            .and_then(|d| d.get("AudioVisual", "GUIMainButtonSound"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("rules.ini")
            .and_then(|b| crate::host::audio::soft_ini_get(&b, "AudioVisual", "GUIMainButtonSound"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "MenuClick".into())
    }

    /// 规则里的壳层出去事件 id（缺省 `MenuSlideOut`）。
    pub(super) fn menu_move_out_sound_id(&self) -> String {
        let from_doc = self
            .read_ini_doc("rules.ini")
            .as_ref()
            .and_then(|d| d.get("AudioVisual", "GUIMoveOutSound"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("rules.ini")
            .and_then(|b| crate::host::audio::soft_ini_get(&b, "AudioVisual", "GUIMoveOutSound"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "MenuSlideOut".into())
    }

    /// 规则里的壳层进来事件 id（缺省 `MenuSlideIn`）。
    pub(super) fn menu_move_in_sound_id(&self) -> String {
        let from_doc = self
            .read_ini_doc("rules.ini")
            .as_ref()
            .and_then(|d| d.get("AudioVisual", "GUIMoveInSound"))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(s) = from_doc {
            return s;
        }
        self.read_asset_bytes("rules.ini")
            .and_then(|b| crate::host::audio::soft_ini_get(&b, "AudioVisual", "GUIMoveInSound"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "MenuSlideIn".into())
    }

    /// `sound.ini` 事件 → `Sounds=` 采样名列表。
    pub(super) fn sound_event_sample_names(&self, event_id: &str) -> Vec<String> {
        let line = self
            .read_ini_doc("sound.ini")
            .as_ref()
            .and_then(|d| d.get(event_id, "Sounds"))
            .map(str::to_string)
            .or_else(|| self.read_asset_bytes("sound.ini").and_then(|b| crate::host::audio::soft_ini_get(&b, event_id, "Sounds")))
            .unwrap_or_default();
        line.split_whitespace().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
    }

    /// 按 `theme.ini` 词干尝试 `{stem}.wav` / `{stem}.aud`（通常来自 `THEME.MIX`）。
    ///
    /// 不回退 `intro.aud`：那是别的曲目，不是主菜单 `Grinder`。
    pub(super) fn decode_theme_track(&self, stem: &str) -> Option<PcmAudio> {
        let mut names: Vec<String> = Vec::new();
        for ext in ["wav", "aud"] {
            names.push(format!("{stem}.{ext}"));
        }
        for name in &names {
            let Some(bytes) = self.read_asset_bytes(name)
            else {
                continue;
            };
            let ext = name.rsplit_once('.').map(|(_, e)| e);
            match decode_audio_bytes(&bytes, ext) {
                Ok(pcm) => {
                    tracing::info!(
                        %name,
                        frames = pcm.samples.len() / pcm.channels.max(1) as usize,
                        rate = pcm.sample_rate,
                        "已加载菜单 BGM"
                    );
                    return Some(pcm);
                }
                Err(e) => {
                    tracing::warn!(%name, error = %e, "主题曲解码失败");
                }
            }
        }
        None
    }

    /// 主题曲缺失诊断（仅当前 `--path` 安装根）。
    pub(super) fn warn_theme_unavailable(&self, stem: &str) {
        let theme_note = match self.read_asset_bytes("theme.mix").or_else(|| self.read_asset_bytes("Theme.mix")) {
            Some(bytes) if bytes.as_slice() == b"CLASS" || bytes.len() < 64 => {
                format!("theme.mix 为占位（{} 字节），无法读取 {stem}.wav", bytes.len())
            }
            Some(bytes) => format!("theme.mix 可读（{} 字节）但未解出 {stem}.wav/.aud", bytes.len()),
            None => format!("无 theme.mix，且未解出 {stem}.wav/.aud"),
        };
        tracing::warn!(%stem, %theme_note, "菜单主题曲不可用，BGM 静音");
    }

    /// 惰性装载菜单 BGM / 点击 / 切页进出采样。
    pub(super) fn ensure_menu_audio_assets(&mut self) {
        if (self.menu_bgm.is_some() || self.menu_bgm_tried) && self.menu_click.is_some() && self.menu_move_out_tried && self.menu_move_in_tried
        {
            return;
        }
        self.ensure_menu_assets();
        if self.menu_bgm.is_none() && !self.menu_bgm_tried {
            self.menu_bgm_tried = true;
            let stem = self.menu_theme_sound_stem();
            if let Some(pcm) = self.decode_theme_track(&stem) {
                self.menu_bgm = Some(pcm);
            }
            else {
                self.warn_theme_unavailable(&stem);
            }
        }
        if self.menu_click.is_none() {
            let event_id = self.menu_click_sound_id();
            let mut candidates: Vec<String> = self.sound_event_sample_names(&event_id);
            // 零售 `[MenuClick] Sounds=umenucl1`；sound.ini 解析失败时仍走 bag 名。
            for fallback in ["umenucl1", "UMENUCL1", "MenuClick"] {
                if !candidates.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    candidates.push(fallback.into());
                }
            }
            let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
            let loaded = self.decode_bag_named(&refs);
            self.menu_click = Some(loaded.unwrap_or_else(|| {
                tracing::warn!(%event_id, "菜单点击采样未命中，使用合成占位");
                crate::host::audio::synthetic_ui_click()
            }));
        }
        if !self.menu_move_out_tried {
            self.menu_move_out_tried = true;
            let event_id = self.menu_move_out_sound_id();
            let mut candidates: Vec<String> = self.sound_event_sample_names(&event_id);
            for fallback in ["uslide2", "USLIDE2", "MenuSlideOut"] {
                if !candidates.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    candidates.push(fallback.into());
                }
            }
            let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
            self.menu_move_out = self.decode_bag_named(&refs);
            if self.menu_move_out.is_none() {
                tracing::warn!(%event_id, "壳层出去采样未命中");
            }
        }
        if !self.menu_move_in_tried {
            self.menu_move_in_tried = true;
            let event_id = self.menu_move_in_sound_id();
            let mut candidates: Vec<String> = self.sound_event_sample_names(&event_id);
            for fallback in ["uslide1", "USLIDE1", "MenuSlideIn"] {
                if !candidates.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    candidates.push(fallback.into());
                }
            }
            let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
            self.menu_move_in = self.decode_bag_named(&refs);
            if self.menu_move_in.is_none() {
                tracing::warn!(%event_id, "壳层进来采样未命中");
            }
        }
    }

    /// 前置壳层页播 BGM；结算播 SCORE 主题；离开则停。
    pub(super) fn sync_shell_audio(&mut self) {
        self.ensure_menu_audio_assets();
        self.ensure_score_bgm();
        let want_kind: Option<&'static str> = if self.screen == OriginalScreen::Results {
            Some("score")
        } else if matches!(
            self.screen,
            OriginalScreen::MainMenu
                | OriginalScreen::SinglePlayerMenu
                | OriginalScreen::Campaign
                | OriginalScreen::Options
                | OriginalScreen::ExitConfirm
                | OriginalScreen::SkirmishLobby
                | OriginalScreen::ChooseMap
                | OriginalScreen::Network
        ) {
            Some("menu")
        } else {
            None
        };
        if self.shell_bgm_kind == want_kind && (want_kind.is_none() || self.menu_bgm_playing) {
            return;
        }
        if self.menu_bgm_playing {
            if let Some(audio) = self.audio.as_mut() {
                audio.stop_music();
            }
            self.menu_bgm_playing = false;
        }
        self.shell_bgm_kind = want_kind;
        match want_kind {
            Some("score") => {
                if let (Some(audio), Some(bgm)) = (self.audio.as_mut(), self.score_bgm.as_ref()) {
                    audio.play_music_loop(bgm);
                    self.menu_bgm_playing = true;
                }
            }
            Some("menu") => {
                if let (Some(audio), Some(bgm)) = (self.audio.as_mut(), self.menu_bgm.as_ref()) {
                    audio.play_music_loop(bgm);
                    self.menu_bgm_playing = true;
                }
            }
            _ => {}
        }
    }

    /// 惰性装载结算主题曲（`theme.ini` `[SCORE]`）。
    pub(super) fn ensure_score_bgm(&mut self) {
        if self.score_bgm.is_some() || self.score_bgm_tried {
            return;
        }
        self.score_bgm_tried = true;
        self.ensure_menu_assets();
        let stem = self
            .read_ini_doc("theme.ini")
            .as_ref()
            .and_then(|d| d.get("SCORE", "Sound"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "RA2-Sco".into());
        if let Some(pcm) = self.decode_theme_track(&stem) {
            tracing::info!(%stem, "已装载结算主题曲");
            self.score_bgm = Some(pcm);
        } else {
            tracing::debug!(%stem, "结算主题曲不可用");
        }
    }

    /// 菜单按钮按下时播一次点击音。
    pub(super) fn play_menu_click(&mut self) {
        self.ensure_menu_audio_assets();
        if let (Some(audio), Some(click)) = (self.audio.as_mut(), self.menu_click.as_ref()) {
            audio.play_sfx(click);
        }
    }

    /// 壳层出去波浪开始时播一次（`GUIMoveOutSound`）。
    pub(super) fn play_menu_move_out(&mut self) {
        self.ensure_menu_audio_assets();
        if let (Some(audio), Some(sfx)) = (self.audio.as_mut(), self.menu_move_out.as_ref()) {
            audio.play_sfx(sfx);
        }
    }

    /// 壳层进来波浪开始时播一次（`GUIMoveInSound`）。
    pub(super) fn play_menu_move_in(&mut self) {
        self.ensure_menu_audio_assets();
        if let (Some(audio), Some(sfx)) = (self.audio.as_mut(), self.menu_move_in.as_ref()) {
            audio.play_sfx(sfx);
        }
    }

    /// 战役选边悬停切入语音（`sound.ini` Allied/BootCamp/SovietCampaignSelect）。
    pub(super) fn play_campaign_side_hover(&mut self, side: &str) {
        let slot = match side {
            "allied" => 0,
            "tutorial" => 1,
            "soviet" => 2,
            _ => return,
        };
        let event_id = match side {
            "allied" => "AlliedCampaignSelect",
            "tutorial" => "BootCampSelect",
            "soviet" => "SovietCampaignSelect",
            _ => return,
        };
        if self.campaign_side_sfx[slot].is_none() {
            self.ensure_menu_assets();
            let mut names: Vec<String> = self
                .sound_event_sample_names(event_id)
                .into_iter()
                .map(|s| s.trim().trim_start_matches(['$', '#']).to_string())
                .filter(|s| !s.is_empty())
                .collect();
            // 零售缺省采样名（sound.ini 解析失败时仍可从 bag 取）。
            for fallback in match side {
                "allied" => ["itanatc", "ITANATC"],
                "tutorial" => ["igisea", "IGISEA"],
                _ => ["vgrsatc", "VGRSATC"],
            } {
                if !names.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                    names.push(fallback.into());
                }
            }
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            self.campaign_side_sfx[slot] = self.decode_bag_named(&refs);
            if self.campaign_side_sfx[slot].is_none() {
                tracing::warn!(%event_id, "战役选边悬停采样未命中");
            }
        }
        if let (Some(audio), Some(pcm)) = (self.audio.as_mut(), self.campaign_side_sfx[slot].as_ref()) {
            audio.play_sfx(pcm);
        }
    }

    /// 对局短音效 / EVA：`sound.ini` 或 `eva.ini` → `audio.bag` 或 MIX 内 `.wav`。
    ///
    /// 成功解码时返回 PCM（供结算延迟按采样时长对齐）。
    pub(super) fn play_battle_sfx_event(&mut self, event_id: &str) -> Option<ra_assets::PcmAudio> {
        if event_id.is_empty() {
            return None;
        }
        self.ensure_audio_bag();
        let mut candidates: Vec<String> = Vec::new();
        if event_id.starts_with("EVA_") || event_id.eq_ignore_ascii_case("EVA_BattleControlTerminated") {
            candidates.extend(self.eva_sample_names(event_id));
        }
        candidates.extend(self.sound_event_sample_names(event_id));
        // 零售 `[PlaceBuilding] Sounds=uplace`；解析失败时仍走 bag / MIX 名。
        let fallbacks: &[&str] = match event_id {
            id if id.eq_ignore_ascii_case("PlaceBuilding") => &["uplace", "UPLACE", "PlaceBuilding"],
            id if id.eq_ignore_ascii_case("EVA_BattleControlTerminated") => &["ceva015", "csof015", "CEVA015", "CSOF015"],
            id if id.eq_ignore_ascii_case("EVA_MissionAccomplished") => &["ceva013", "csof013", "CEVA013", "CSOF013"],
            id if id.eq_ignore_ascii_case("EVA_MissionFailed") => &["ceva014", "csof014", "CEVA014", "CSOF014"],
            _ => &[],
        };
        for fallback in fallbacks {
            if !candidates.iter().any(|c| c.eq_ignore_ascii_case(fallback)) {
                candidates.push((*fallback).into());
            }
        }
        if candidates.is_empty() {
            candidates.push(event_id.to_string());
        }
        let refs: Vec<&str> = candidates.iter().map(String::as_str).collect();
        let Some(pcm) = self.decode_sfx_named(&refs)
        else {
            tracing::warn!(%event_id, candidates = ?candidates, "对局音效/EVA 未命中");
            return None;
        };
        if let Some(audio) = self.audio.as_mut() {
            audio.play_sfx(&pcm);
            tracing::info!(%event_id, frames = pcm.samples.len(), "已播放对局音效/EVA");
        }
        Some(pcm)
    }

    /// `eva.ini` 事件 → Allied/Russian 采样名（按本机 UI 阵营族优先）。
    pub(super) fn eva_sample_names(&self, event_id: &str) -> Vec<String> {
        let Some(doc) = self.read_ini_doc("eva.ini")
        else {
            return Vec::new();
        };
        let chrome = self
            .battle_controller
            .as_ref()
            .and_then(|c| c.ui_faction_chrome().cloned())
            .or_else(|| {
                self.battle_controller.as_ref().and_then(|c| {
                    c.session.as_ref().and_then(|s| s.battle()).and_then(|g| {
                        g.world
                            .players
                            .iter()
                            .find(|p| p.id == g.world.local_player)
                            .map(|p| {
                                let house = p.house.as_ref();
                                let fid = self
                                    .lobby_countries
                                    .iter()
                                    .find(|c| c.id.eq_ignore_ascii_case(house))
                                    .map(|c| c.side.as_str())
                                    .filter(|s| !s.is_empty());
                                self.resolve_ui_faction_chrome(house, fid)
                            })
                    })
                })
            })
            .unwrap_or_else(|| ra_widgets::skirmish_setup::UiFactionChrome::from_mix_index(1, false));
        // 按 `EVA.Tag` 再 Allied/Russian 键取采样名，不按苏盟二元猜优先序。
        let mut out = Vec::new();
        for key in chrome.eva_sample_keys() {
            let stem = doc.get(event_id, &key).unwrap_or("").trim().to_string();
            if stem.is_empty() {
                continue;
            }
            if !out.iter().any(|c: &String| c.eq_ignore_ascii_case(&stem)) {
                out.push(stem);
            }
        }
        out
    }
}
