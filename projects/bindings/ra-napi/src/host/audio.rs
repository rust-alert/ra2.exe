//! 桌面音频输出：rodio 设备 + PCM 一次/循环播放。
//!
//! 解码在 `ra-assets`（Symphonia）；本模块只负责设备与播放生命周期。
//!
//! 通道约定：
//! - **music**：BGM 单轨循环
//! - **sfx**：短音效多实例（`MAX_SFX` FIFO 淘汰）
//! - **voice**：EVA / 旁白单轨（替换上一句，不进短音淘汰池）

use std::num::NonZero;

use ra_assets::PcmAudio;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source, buffer::SamplesBuffer};

/// 对局 / 壳层事件 id 是否应走语音轨（`EVA_*`）。
#[inline]
pub fn is_eva_event_id(event_id: &str) -> bool {
    event_id.starts_with("EVA_")
}

/// `sound.ini` 空间衰减参数（`Volume`/`MinVolume` 已归一到 0..1；`Range` 为格）。
///
/// 零售 `[Defaults]`：`Range=10`、`Volume=80`、`MinVolume=50`。超出 `Range` 且非 `GLOBAL` 时静音。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundSpatialParams {
    /// 衰减半径（地图格）。
    pub range_cells: f32,
    /// 近距音量（0..1）。
    pub volume: f32,
    /// 到达 `Range` 时的音量下限（0..1）；仅 `global` 时在更远处仍保持该值。
    pub min_volume: f32,
    /// `Type` 含 `GLOBAL`：全图至少 `min_volume`。
    pub global: bool,
}

impl SoundSpatialParams {
    /// 零售 `sound.ini` `[Defaults]`。
    pub const DEFAULTS: Self = Self { range_cells: 10.0, volume: 0.80, min_volume: 0.50, global: false };
}

/// 按与听者（镜头中心格）的格距，计算事件音量倍率（0..1，已含 `Volume`/`MinVolume`）。
///
/// - `dist <= Range`：从 `volume` 线性落到 `min_volume`
/// - `dist > Range`：非 `GLOBAL` → `0`；`GLOBAL` → `min_volume`
pub fn spatial_volume_scale(dist_cells: f32, params: &SoundSpatialParams) -> f32 {
    let dist = dist_cells.max(0.0);
    let range = params.range_cells.max(0.001);
    let near = params.volume.clamp(0.0, 1.0);
    let far = params.min_volume.clamp(0.0, 1.0);
    if dist > range {
        return if params.global { far } else { 0.0 };
    }
    let t = dist / range;
    near + (far - near) * t
}

/// 两格欧氏距离（浮点格）。
#[inline]
pub fn cell_distance(ax: u16, ay: u16, bx: u16, by: u16) -> f32 {
    let dx = f32::from(ax) - f32::from(bx);
    let dy = f32::from(ay) - f32::from(by);
    (dx * dx + dy * dy).sqrt()
}

/// 解析 `Type=` 是否含 `GLOBAL`（大小写不敏感；可与其它 flag 并列）。
pub fn sound_type_is_global(type_line: &str) -> bool {
    type_line.split_whitespace().any(|t| t.eq_ignore_ascii_case("GLOBAL"))
}

/// 壳层音频：BGM 单轨循环 + SFX 短音 + EVA 独立语音轨。
pub struct ShellAudio {
    _device: MixerDeviceSink,
    music: Option<Player>,
    /// 尚未播完的短音效播放器（下次 `play_sfx` 时清理已空轨）。
    sfx: Vec<Player>,
    /// EVA / 语音单轨：不与战斗短音共享 `MAX_SFX` 淘汰，避免遇袭播报被开火音掐断。
    voice: Option<Player>,
    /// 主菜单主题音量（相对全量）。
    music_volume: f32,
    /// 点击等音效音量。
    sfx_volume: f32,
}

impl ShellAudio {
    /// 打开默认输出设备；失败时返回 `None`（壳层继续无声运行）。
    pub fn try_open() -> Option<Self> {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(device) => {
                tracing::info!("音频输出已打开");
                Some(Self { _device: device, music: None, sfx: Vec::new(), voice: None, music_volume: 0.4, sfx_volume: 0.7 })
            }
            Err(e) => {
                tracing::warn!(error = %e, "音频输出不可用，继续静音运行");
                None
            }
        }
    }

    /// 停止当前 BGM 并以循环方式播放新曲。
    pub fn play_music_loop(&mut self, pcm: &PcmAudio) {
        let Some(source) = pcm_to_source(pcm)
        else {
            tracing::warn!("BGM 采样无效，跳过播放");
            return;
        };
        if let Some(player) = self.music.take() {
            player.stop();
        }
        let player = Player::connect_new(self._device.mixer());
        player.set_volume(self.music_volume);
        player.append(source.repeat_infinite());
        self.music = Some(player);
        tracing::info!(
            sample_rate = pcm.sample_rate,
            channels = pcm.channels,
            frames = pcm.samples.len() / pcm.channels.max(1) as usize,
            "BGM 循环播放已开始"
        );
    }

    /// 停止 BGM。
    pub fn stop_music(&mut self) {
        if let Some(player) = self.music.take() {
            player.stop();
        }
    }

    /// 停止全部短音效与语音轨（进结算时掐断残留 EVA，避免压过 SCORE 主题）。
    pub fn stop_sfx(&mut self) {
        for player in self.sfx.drain(..) {
            player.stop();
        }
        self.stop_voice();
    }

    /// 停止 EVA 语音轨。
    pub fn stop_voice(&mut self) {
        if let Some(player) = self.voice.take() {
            player.stop();
        }
    }

    /// 设置 BGM 音量（0..1），立刻作用于当前音乐轨。
    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        if let Some(player) = self.music.as_ref() {
            player.set_volume(self.music_volume);
        }
    }

    /// 当前 BGM 音量（0..1）。
    pub fn music_volume(&self) -> f32 {
        self.music_volume
    }

    /// 设置短音效 / 语音音量（0..1），作用于随后的 `play_sfx` / `play_voice`，并立刻作用于当前语音轨。
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
        if let Some(player) = self.voice.as_ref() {
            player.set_volume(self.sfx_volume);
        }
    }

    /// 当前短音效音量（0..1）。
    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    /// 播放一次短音效（不打断 BGM / EVA 语音轨）；音量为 `sfx_volume`。
    pub fn play_sfx(&mut self, pcm: &PcmAudio) {
        self.play_sfx_gain(pcm, 1.0);
    }

    /// 播放一次短音效，额外乘以 `gain`（0..1，来自空间衰减 / 事件 `Volume`）。
    ///
    /// `gain ≈ 0` 时跳过，避免占满 `MAX_SFX`。
    pub fn play_sfx_gain(&mut self, pcm: &PcmAudio, gain: f32) {
        let gain = gain.clamp(0.0, 1.0);
        if gain < 0.01 {
            return;
        }
        self.sfx.retain(|p| !p.empty());
        // 连点时丢弃最旧实例，避免短音轨无限堆积。
        const MAX_SFX: usize = 4;
        while self.sfx.len() >= MAX_SFX {
            if let Some(old) = self.sfx.first() {
                old.stop();
            }
            self.sfx.remove(0);
        }
        let Some(source) = pcm_to_source(pcm)
        else {
            tracing::warn!("SFX 采样无效，跳过播放");
            return;
        };
        let player = Player::connect_new(self._device.mixer());
        player.set_volume(self.sfx_volume * gain);
        player.append(source);
        self.sfx.push(player);
    }

    /// 播放一句 EVA / 旁白（单轨替换；不进入短音 `MAX_SFX` 淘汰池）。
    pub fn play_voice(&mut self, pcm: &PcmAudio) {
        if self.voice.as_ref().is_some_and(|p| p.empty()) {
            self.voice = None;
        }
        let Some(source) = pcm_to_source(pcm)
        else {
            tracing::warn!("语音采样无效，跳过播放");
            return;
        };
        if let Some(player) = self.voice.take() {
            player.stop();
        }
        let player = Player::connect_new(self._device.mixer());
        player.set_volume(self.sfx_volume);
        player.append(source);
        self.voice = Some(player);
    }
}

#[cfg(test)]
mod tests {
    use super::{SoundSpatialParams, cell_distance, is_eva_event_id, sound_type_is_global, spatial_volume_scale};

    #[test]
    fn eva_event_id_prefix() {
        assert!(is_eva_event_id("EVA_OurBaseIsUnderAttack"));
        assert!(is_eva_event_id("EVA_UnitLost"));
        assert!(!is_eva_event_id("PlaceBuilding"));
        assert!(!is_eva_event_id("SellBuilding"));
        assert!(!is_eva_event_id(""));
    }

    #[test]
    fn spatial_volume_falls_off_then_mutes_beyond_range() {
        let p = SoundSpatialParams::DEFAULTS;
        assert!((spatial_volume_scale(0.0, &p) - 0.80).abs() < 1e-4);
        assert!((spatial_volume_scale(10.0, &p) - 0.50).abs() < 1e-4);
        assert_eq!(spatial_volume_scale(11.0, &p), 0.0);
        let global = SoundSpatialParams { global: true, ..p };
        assert!((spatial_volume_scale(50.0, &global) - 0.50).abs() < 1e-4);
    }

    #[test]
    fn cell_distance_and_global_type_flags() {
        assert!((cell_distance(0, 0, 3, 4) - 5.0).abs() < 1e-4);
        assert!(sound_type_is_global("GLOBAL SHROUD"));
        assert!(sound_type_is_global("normal Global"));
        assert!(!sound_type_is_global("NORMAL SCREEN UNSHROUD"));
    }
}

fn pcm_to_source(pcm: &PcmAudio) -> Option<SamplesBuffer> {
    let channels = NonZero::new(pcm.channels)?;
    let rate = NonZero::new(pcm.sample_rate)?;
    if pcm.samples.is_empty() {
        return None;
    }
    // 升混到立体声，避免部分设备对单声道缓冲不友好。
    let samples: Vec<f32> = if pcm.channels == 1 {
        pcm.samples
            .iter()
            .flat_map(|&s| {
                let f = s as f32 / 32768.0;
                [f, f]
            })
            .collect()
    }
    else {
        pcm.samples.iter().map(|&s| s as f32 / 32768.0).collect()
    };
    let out_channels = if pcm.channels == 1 { NonZero::new(2).unwrap() } else { channels };
    Some(SamplesBuffer::new(out_channels, rate, samples))
}

/// 宽松抽取 INI `section`/`key`（整文件严格解析失败时的回退）。
pub fn soft_ini_get(bytes: &[u8], section: &str, key: &str) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut in_section = false;
    for raw in text.lines() {
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(name) = rest.strip_suffix(']') {
                in_section = name.trim().eq_ignore_ascii_case(section);
                continue;
            }
        }
        if !in_section {
            continue;
        }
        let Some((k, v)) = line.split_once('=')
        else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case(key) {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// 去掉主题 `Sound=` 前导 `$` / `#`。
pub fn theme_sound_stem(raw: &str) -> &str {
    raw.trim().trim_start_matches(['$', '#'])
}

/// 资源缺失时的短点击占位（约 40ms @ 22050 mono）。
pub fn synthetic_ui_click() -> PcmAudio {
    const RATE: u32 = 22_050;
    const FRAMES: usize = 882; // ~40ms
    let mut samples = Vec::with_capacity(FRAMES);
    for i in 0..FRAMES {
        let t = i as f32 / RATE as f32;
        let env = (1.0 - t / 0.04).clamp(0.0, 1.0);
        let s = (t * 1800.0 * std::f32::consts::TAU).sin() * env * 0.35;
        samples.push((s * 32767.0) as i16);
    }
    PcmAudio { sample_rate: RATE, channels: 1, samples }
}
