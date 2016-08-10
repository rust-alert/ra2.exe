//! 天气氛围粒子（飘雪 / 薄雾 / 雨丝）：呈现层 CPU 叠画，不进仿真哈希。
//!
//! 剧院默认：雪地剧院启用飘雪；其余剧院无粒子。可显式构造 `Fog` / `Rain`。

use image::RgbaImage;

use crate::Theater;

/// 天气粒子种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[doc(hidden)]
pub enum WeatherKind {
    /// 无粒子。
    #[default]
    None,
    /// 飘雪（斜落白点）。
    Snow,
    /// 薄雾（慢漂移半透明团）。
    Fog,
    /// 雨丝（细长竖向半透明条）。
    Rain,
}

/// 单颗粒子。
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub struct WeatherParticle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    /// 雪：半径；雨：长度；雾：半径。
    pub size: f32,
    pub alpha: u8,
}

/// 可在预览 RGBA 上叠画的天气粒子场。
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct WeatherParticleField {
    pub kind: WeatherKind,
    pub width: u32,
    pub height: u32,
    pub particles: Vec<WeatherParticle>,
    pub rng: u64,
    /// 累计时间（毫秒），供确定性推进。
    pub elapsed_ms: u64,
}

impl WeatherParticleField {
    /// 空场（不画）。
    pub fn none() -> Self {
        Self { kind: WeatherKind::None, width: 0, height: 0, particles: Vec::new(), rng: 0xC0FFEE_u64, elapsed_ms: 0 }
    }

    /// 按剧院选默认天气（雪地 → 飘雪）。
    pub fn for_theater(theater: Theater, width: u32, height: u32) -> Self {
        let kind = match theater {
            Theater::Snow => WeatherKind::Snow,
            _ => WeatherKind::None,
        };
        Self::new(kind, width, height, 0xA5F10E57_u64 ^ (width as u64) << 16 ^ height as u64)
    }

    /// 构造指定种类的粒子场。
    pub fn new(kind: WeatherKind, width: u32, height: u32, seed: u64) -> Self {
        let mut field = Self { kind, width, height, particles: Vec::new(), rng: seed | 1, elapsed_ms: 0 };
        field.respawn_all();
        field
    }

    /// 当前种类。
    pub fn kind(&self) -> WeatherKind {
        self.kind
    }

    /// 是否会叠画。
    pub fn is_active(&self) -> bool {
        self.kind != WeatherKind::None && self.width > 0 && self.height > 0 && !self.particles.is_empty()
    }

    /// 画布尺寸变化时重建粒子。
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        self.respawn_all();
    }

    /// 切换种类并重建。
    pub fn set_kind(&mut self, kind: WeatherKind) {
        if kind == self.kind {
            return;
        }
        self.kind = kind;
        self.respawn_all();
    }

    /// 推进 `dt_ms` 毫秒。
    pub fn tick(&mut self, dt_ms: u64) {
        if !self.is_active() || dt_ms == 0 {
            self.elapsed_ms = self.elapsed_ms.wrapping_add(dt_ms);
            return;
        }
        self.elapsed_ms = self.elapsed_ms.wrapping_add(dt_ms);
        let dt = (dt_ms as f32 / 1000.0).min(0.1);
        let w = self.width as f32;
        let h = self.height as f32;
        let kind = self.kind;
        let n = self.particles.len();
        for i in 0..n {
            self.particles[i].x += self.particles[i].vx * dt;
            self.particles[i].y += self.particles[i].vy * dt;
            match kind {
                WeatherKind::Snow => {
                    if self.particles[i].y > h + 4.0 {
                        let nx = self.next_f32() * w;
                        self.particles[i].y = -4.0;
                        self.particles[i].x = nx;
                    }
                    if self.particles[i].x < -4.0 {
                        self.particles[i].x = w + 4.0;
                    }
                    else if self.particles[i].x > w + 4.0 {
                        self.particles[i].x = -4.0;
                    }
                }
                WeatherKind::Fog => {
                    if self.particles[i].x > w + self.particles[i].size {
                        let ny = self.next_f32() * h;
                        self.particles[i].x = -self.particles[i].size;
                        self.particles[i].y = ny;
                    }
                    else if self.particles[i].x < -self.particles[i].size {
                        let ny = self.next_f32() * h;
                        self.particles[i].x = w + self.particles[i].size;
                        self.particles[i].y = ny;
                    }
                }
                WeatherKind::Rain => {
                    if self.particles[i].y > h + self.particles[i].size {
                        let nx = self.next_f32() * w;
                        self.particles[i].y = -self.particles[i].size;
                        self.particles[i].x = nx;
                    }
                }
                WeatherKind::None => {}
            }
        }
    }

    /// 叠画到 RGBA 预览（就地 alpha 混合）。
    pub fn paint_onto(&self, image: &mut RgbaImage) {
        if !self.is_active() {
            return;
        }
        let (iw, ih) = (image.width(), image.height());
        if iw == 0 || ih == 0 {
            return;
        }
        match self.kind {
            WeatherKind::Snow => {
                for p in &self.particles {
                    let r = p.size.max(1.0) as i32;
                    let cx = p.x as i32;
                    let cy = p.y as i32;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if dx * dx + dy * dy > r * r {
                                continue;
                            }
                            let x = cx + dx;
                            let y = cy + dy;
                            if x < 0 || y < 0 || x >= iw as i32 || y >= ih as i32 {
                                continue;
                            }
                            blend_add(image, x as u32, y as u32, 235, 245, 255, p.alpha);
                        }
                    }
                }
            }
            WeatherKind::Fog => {
                for p in &self.particles {
                    let r = p.size.max(8.0) as i32;
                    let cx = p.x as i32;
                    let cy = p.y as i32;
                    let r2 = r * r;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            let d2 = dx * dx + dy * dy;
                            if d2 > r2 {
                                continue;
                            }
                            let x = cx + dx;
                            let y = cy + dy;
                            if x < 0 || y < 0 || x >= iw as i32 || y >= ih as i32 {
                                continue;
                            }
                            let falloff = 1.0 - (d2 as f32 / r2 as f32);
                            let a = ((f32::from(p.alpha) * falloff * 0.55) as u8).max(1);
                            blend_add(image, x as u32, y as u32, 200, 210, 220, a);
                        }
                    }
                }
            }
            WeatherKind::Rain => {
                for p in &self.particles {
                    let len = p.size.max(4.0) as i32;
                    let x = p.x as i32;
                    let y0 = p.y as i32;
                    for i in 0..len {
                        let y = y0 + i;
                        if x < 0 || y < 0 || x >= iw as i32 || y >= ih as i32 {
                            continue;
                        }
                        let a = if i < 2 { p.alpha } else { p.alpha / 2 };
                        blend_add(image, x as u32, y as u32, 160, 180, 210, a);
                    }
                }
            }
            WeatherKind::None => {}
        }
    }

    fn respawn_all(&mut self) {
        self.particles.clear();
        if self.kind == WeatherKind::None || self.width == 0 || self.height == 0 {
            return;
        }
        let area = (self.width as u64).saturating_mul(self.height as u64);
        let count = match self.kind {
            WeatherKind::Snow => ((area / 2800).clamp(48, 420)) as usize,
            WeatherKind::Fog => ((area / 22000).clamp(10, 48)) as usize,
            WeatherKind::Rain => ((area / 1800).clamp(80, 520)) as usize,
            WeatherKind::None => 0,
        };
        let w = self.width as f32;
        let h = self.height as f32;
        self.particles.reserve(count);
        for _ in 0..count {
            let p = match self.kind {
                WeatherKind::Snow => WeatherParticle {
                    x: self.next_f32() * w,
                    y: self.next_f32() * h,
                    vx: -12.0 - self.next_f32() * 18.0,
                    vy: 55.0 + self.next_f32() * 70.0,
                    size: 1.0 + self.next_f32() * 1.6,
                    alpha: 140 + (self.next_u32() % 90) as u8,
                },
                WeatherKind::Fog => WeatherParticle {
                    x: self.next_f32() * w,
                    y: self.next_f32() * h,
                    vx: 8.0 + self.next_f32() * 14.0,
                    vy: (self.next_f32() - 0.5) * 6.0,
                    size: 28.0 + self.next_f32() * 48.0,
                    alpha: 28 + (self.next_u32() % 24) as u8,
                },
                WeatherKind::Rain => WeatherParticle {
                    x: self.next_f32() * w,
                    y: self.next_f32() * h,
                    vx: -8.0 - self.next_f32() * 10.0,
                    vy: 280.0 + self.next_f32() * 160.0,
                    size: 5.0 + self.next_f32() * 7.0,
                    alpha: 90 + (self.next_u32() % 70) as u8,
                },
                WeatherKind::None => unreachable!(),
            };
            self.particles.push(p);
        }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D)) >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}

#[doc(hidden)]
pub fn blend_add(image: &mut RgbaImage, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
    let px = image.get_pixel_mut(x, y);
    let src_a = f32::from(a) / 255.0;
    let dst_a = 1.0 - src_a;
    px.0[0] = (f32::from(px.0[0]) * dst_a + f32::from(r) * src_a).clamp(0.0, 255.0) as u8;
    px.0[1] = (f32::from(px.0[1]) * dst_a + f32::from(g) * src_a).clamp(0.0, 255.0) as u8;
    px.0[2] = (f32::from(px.0[2]) * dst_a + f32::from(b) * src_a).clamp(0.0, 255.0) as u8;
}
