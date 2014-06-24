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

    /// 播放一次短音效（不打断 BGM）。
    pub fn play_sfx(&mut self, pcm: &PcmAudio) {
        self.sfx.retain(|p| !p.empty());
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
