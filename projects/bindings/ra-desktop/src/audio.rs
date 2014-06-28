//! 桌面音频输出：rodio 设备 + PCM 一次/循环播放。
//!
//! 解码在 `ra-assets`（Symphonia）；本模块只负责设备与播放生命周期。

use std::num::NonZero;

use ra_assets::PcmAudio;
use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};

/// 壳层音频：BGM 单轨循环 + SFX 短音。
pub struct ShellAudio {
    _device: MixerDeviceSink,
    music: Option<Player>,
    /// 尚未播完的短音效播放器（下次 `play_sfx` 时清理已空轨）。
    sfx: Vec<Player>,
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
                Some(Self {
                    _device: device,
                    music: None,
                    sfx: Vec::new(),
                    music_volume: 0.4,
                    sfx_volume: 0.7,
                })
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

    /// 设置短音效音量（0..1），作用于随后的 `play_sfx`。
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    /// 当前短音效音量（0..1）。
    pub fn sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    /// 播放一次短音效（不打断 BGM）。
    pub fn play_sfx(&mut self, pcm: &PcmAudio) {
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
        player.set_volume(self.sfx_volume);
        player.append(source);
        self.sfx.push(player);
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
    } else {
        pcm.samples.iter().map(|&s| s as f32 / 32768.0).collect()
    };
    let out_channels = if pcm.channels == 1 {
        NonZero::new(2).unwrap()
    } else {
        channels
    };
    Some(SamplesBuffer::new(out_channels, rate, samples))
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
    PcmAudio {
        sample_rate: RATE,
        channels: 1,
        samples,
    }
}

