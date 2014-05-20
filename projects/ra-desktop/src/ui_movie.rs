//! 主菜单循环影片：自研 Bink 解码器 + 帧时钟（上传前合成进 UI 页）。

use ra_assets::{BinkFile, BinkVideoDecoder, parse_bink_file};
use ra_renderer::RgbaImage;

/// 壳层持有的菜单影片播放器。
pub struct MenuMoviePlayer {
    name: String,
    bytes: Vec<u8>,
    file: BinkFile,
    decoder: BinkVideoDecoder,
    frame_index: usize,
    accum_secs: f64,
    frame: Option<RgbaImage>,
    stalled: Option<String>,
}

impl MenuMoviePlayer {
    /// 解析容器、构造解码器并尝试解第 0 帧。
    pub fn open(name: impl Into<String>, bytes: Vec<u8>) -> Result<Self, String> {
        let name = name.into();
        let file = parse_bink_file(&bytes)?;
        let decoder = BinkVideoDecoder::new(&file.header).map_err(|e| e.to_string())?;
        let mut player = Self { name, bytes, file, decoder, frame_index: 0, accum_secs: 0.0, frame: None, stalled: None };
        player.decode_current()?;
        Ok(player)
    }

    /// 资源短名。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 当前 RGBA 帧（可供合成）。
    pub fn frame(&self) -> Option<&RgbaImage> {
        self.frame.as_ref()
    }

    /// 若解码失步，返回原因（之后不再推进）。
    pub fn stalled_reason(&self) -> Option<&str> {
        self.stalled.as_deref()
    }

    /// 推进时钟；解出新帧时返回 `true`。
    pub fn tick(&mut self, dt_secs: f64) -> bool {
        if self.stalled.is_some() || dt_secs <= 0.0 {
            return false;
        }
        let dur = f64::from(self.file.frame_duration_us()) / 1_000_000.0;
        if dur <= 0.0 {
            return false;
        }
        self.accum_secs += dt_secs;
        let mut advanced = false;
        while self.accum_secs >= dur {
            self.accum_secs -= dur;
            let n = self.file.header.num_frames as usize;
            if n == 0 {
                break;
            }
            let next = (self.frame_index + 1) % n;
            if next == 0 {
                self.decoder.reset();
            }
            self.frame_index = next;
            match self.decode_current() {
                Ok(()) => advanced = true,
                Err(e) => {
                    tracing::warn!(
                        name = %self.name,
                        frame = self.frame_index,
                        "菜单影片解码失步 · {e}"
                    );
                    self.stalled = Some(e);
                    break;
                }
            }
        }
        advanced
    }

    fn decode_current(&mut self) -> Result<(), String> {
        let pkt = self.file.frame_packet(&self.bytes, self.frame_index)?;
        let yuv = self.decoder.decode_packet(pkt.video, pkt.is_keyframe).map_err(|e| e.to_string())?;
        let rgba = yuv.to_rgba8();
        self.frame = Some(
            RgbaImage::from_raw(yuv.width, yuv.height, rgba)
                .ok_or_else(|| format!("RGBA 尺寸非法 {}×{}", yuv.width, yuv.height))?,
        );
        Ok(())
    }
}
